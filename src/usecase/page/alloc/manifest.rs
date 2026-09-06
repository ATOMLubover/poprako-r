//! Authoritative chapter page manifest transactions.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use poprako_orchestra::{Context, OperStep as _};

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::slot::{ObjSlot, ObjSlotSpec};
use poprako_obj_dept::oper::{DeleteObjs, GenObjSlots, ListObjMetas};

use crate::complex::chapter::ChapterComplex;
use crate::complex::page::{PageComplex, manifest};
use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::{
    PageImageSpec, PageManifestEntry, PageRawIdentReplacement,
};
use crate::part::obj_dept::PageImage;
use crate::part::prom::Prom;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::task::Task;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCountMetrics,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    ApplyPageManifest, DeletePages, ListPageInfosExcluded, SetPageRawIdents,
    ShiftPageIndexesTemporary,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash, PageImageKey};

/// Applies the manifest diff and object obligations in one transaction.
#[expect(clippy::too_many_lines, reason = "coordinates manifest invariants")]
pub async fn apply_manifest<C, R, P, O>(
    (repo, prom, obj_dept, context): (&R, &P, &O, &mut C),
    user_id: &str,
    chapter_id: &str,
    page_specs: &[PageImageSpec],
    page_count: usize,
) -> BaseRest<Vec<(String, usize, ImageHash, ImageExt, Option<ObjSlot>)>>
where
    C: Context + Send,
    R: ChapterRepo<C> + ComicRepo<C> + PageRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    let chapter_info = GetChapterInfoExcluded {
        id: chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    ChapterComplex::ensure_chapter_writable(&chapter_info)?;

    let existing_page_infos = ListPageInfosExcluded {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let existing_page_ids = existing_page_infos
        .iter()
        .map(|page_info| page_info.id.as_str())
        .collect::<Vec<_>>();

    let existing_obj_metas = ListObjMetas::<PageImage>::new(&existing_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let manifest_candidates = existing_page_infos
        .iter()
        .map(|page_info| {
            //
            let obj_meta = existing_obj_metas.get(&page_info.id);

            manifest::PageManifestCand {
                id: &page_info.id,
                chapter_id: &page_info.chapter_id,
                index: page_info.index,
                has_units: page_info.total_unit_count > 0,
                image_uploaded: obj_meta.is_some_and(|meta| meta.is_avail),
                image_hash: obj_meta.map(|meta| meta.hash.as_slice()),
                image_ext: obj_meta.map(|meta| meta.ext.as_str()),
            }
        })
        .collect::<Vec<_>>();

    let manifest_plan = manifest::PageManifestComplex::build(
        &chapter_info.id,
        &manifest_candidates,
        page_specs,
    )?;

    let mut retained_ids = HashSet::new();

    for manifest_match in &manifest_plan.matches {
        //
        let Some(existing_index) = manifest_match.existing_index else {
            continue;
        };

        let page_info = existing_page_infos
            .get(existing_index)
            .ok_or_else(page_manifest_result_missing)?;

        retained_ids.insert(page_info.id.as_str());
    }

    let deleted_page_ids = manifest_plan
        .deleted_existing_indexes
        .iter()
        .map(|existing_index| {
            //
            existing_page_infos
                .get(*existing_index)
                .map(|page_info| page_info.id.clone())
                .ok_or_else(page_manifest_result_missing)
        })
        .collect::<BaseRest<Vec<_>>>()?;

    let manifest_entries = manifest_plan
        .matches
        .iter()
        .enumerate()
        .map(|(index, manifest_match)| {
            //
            let id = match manifest_match.existing_index {
                //
                Some(existing_index) => existing_page_infos
                    .get(existing_index)
                    .map(|page_info| page_info.id.clone())
                    .ok_or_else(page_manifest_result_missing)?,

                None => PageComplex::gen_id(),
            };

            accept(PageManifestEntry {
                id,
                chapter_id: chapter_info.id.clone(),
                index,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    ShiftPageIndexesTemporary {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let page_infos = ApplyPageManifest {
        entries: &manifest_entries,
    }
    .step_on(repo, context)
    .await?;

    let raw_ident_replacements = manifest_entries
        .iter()
        .zip(page_specs)
        .map(|(entry, spec)| PageRawIdentReplacement {
            page_id: entry.id.clone(),
            raw_ident: spec.raw_ident.clone(),
        })
        .collect::<Vec<_>>();

    SetPageRawIdents {
        replacements: &raw_ident_replacements,
    }
    .step_on(repo, context)
    .await?;

    let page_infos_by_id = page_infos
        .iter()
        .map(|page_info| (page_info.id.as_str(), page_info))
        .collect::<HashMap<_, _>>();

    if page_infos_by_id.len() != page_count {
        return Err(page_manifest_result_missing());
    }

    let retained_page_ids = manifest_entries
        .iter()
        .zip(page_specs)
        .filter(|(_, page_spec)| page_spec.new_byte_len.is_none())
        .map(|(manifest_entry, _)| manifest_entry.id.as_str())
        .collect::<Vec<_>>();

    let retained_obj_metas = ListObjMetas::<PageImage>::new(&retained_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    for (manifest_entry, page_spec) in manifest_entries.iter().zip(page_specs) {
        //
        let None = page_spec.new_byte_len else {
            continue;
        };

        let page_info = page_infos_by_id
            .get(manifest_entry.id.as_str())
            .ok_or_else(page_manifest_result_missing)?;

        ensure_retained_obj(
            page_info,
            page_spec,
            retained_obj_metas.get(&manifest_entry.id),
        )?;
    }

    let obj_specs = manifest_entries
        .iter()
        .zip(page_specs)
        .filter_map(|(manifest_entry, page_spec)| {
            //
            let byte_len = page_spec.new_byte_len?;

            Some(ObjSlotSpec {
                dom: PageImageKey {
                    chapter_id: chapter_info.id.clone(),
                    page_id: manifest_entry.id.clone(),
                    ext: page_spec.ext,
                },
                hash: page_spec.image_hash.as_bytes(),
                content_type: page_spec.ext.content_type(),
                byte_len,
            })
        })
        .collect::<Vec<_>>();

    let mut obj_slots = GenObjSlots::<PageImage>::new(&obj_specs)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let page_allocs = manifest_entries
        .iter()
        .zip(page_specs)
        .map(|(manifest_entry, page_spec)| {
            //
            let page_info = page_infos_by_id
                .get(manifest_entry.id.as_str())
                .ok_or_else(page_manifest_result_missing)?;

            let obj_slot = match page_spec.new_byte_len {
                //
                Some(_) => obj_slots.remove(&manifest_entry.id),

                None => None,
            };

            accept((
                page_info.id.clone(),
                page_info.index,
                page_spec.image_hash.clone(),
                page_spec.ext,
                obj_slot,
            ))
        })
        .collect::<BaseRest<Vec<_>>>()?;

    DeleteObjs::<PageImage>::new(&deleted_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    DeletePages::Ids {
        ids: &deleted_page_ids,
    }
    .step_on(repo, context)
    .await?;

    let (total_unit_count, translated_unit_count, proofread_unit_count) =
        page_count_metrics(&existing_page_infos, &retained_ids);

    SetChapterPageCountMetrics {
        id: &chapter_info.id,
        page_count,
        total_unit_count,
        translated_unit_count,
        proofread_unit_count,
    }
    .step_on(repo, context)
    .await?;

    let advance_id = next_snowflake_id();

    let advance_payload = TaskPayload::Chapter {
        payload: ChapterPayload::TryAdvanceRawProvideStage {
            chapter_id: chapter_info.id.clone(),
            actor_user_id: Some(user_id.to_owned()),
        },
    };

    let advance_task = Task {
        id: &advance_id,
        payload: &advance_payload,
        delay: Some(Duration::from_mins(20)),
    };

    Defer::new(advance_task).step_on(prom, context).await?;

    TouchComicLastActive {
        id: &chapter_info.comic_id,
    }
    .step_on(repo, context)
    .await?;

    accept(page_allocs)
}

// Builds an internal error for an incomplete page manifest result.
fn page_manifest_result_missing() -> BaseError {
    //
    BaseError::Unrecoverable {
        message: "page manifest result is incomplete".into(),
    }
}

// Verifies that a retained page still points at the requested bytes.
fn ensure_retained_obj(
    page_info: &PageInfo,
    page_spec: &PageImageSpec,
    obj_meta: Option<&ObjMeta>,
) -> BaseRest<()> {
    //
    let Some(obj_meta) = obj_meta else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: format!("page {} requires a new upload", page_info.id),
        });
    };

    let same_hash = obj_meta.hash.as_slice() == page_spec.image_hash.as_bytes();

    match (same_hash, obj_meta.ext == page_spec.ext.suffix()) {
        //
        (true, true) => accept(()),

        _ => Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: format!("page {} requires a new upload", page_info.id),
        }),
    }
}

// Sums unit counters retained by the new manifest.
fn page_count_metrics(
    page_infos: &[PageInfo],
    retained_ids: &HashSet<&str>,
) -> (usize, usize, usize) {
    //
    page_infos
        .iter()
        .filter(|page_info| retained_ids.contains(page_info.id.as_str()))
        .fold((0, 0, 0), |count_metrics, page_info| {
            //
            (
                count_metrics.0 + page_info.total_unit_count,
                count_metrics.1 + page_info.translated_unit_count,
                count_metrics.2 + page_info.proofread_unit_count,
            )
        })
}
