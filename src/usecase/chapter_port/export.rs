// Test coverage for chapter export payload shape and ordering.
#[cfg(test)]
mod tests;

// FIXME: specific models and values are necessary.

use std::collections::HashMap;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::oper::ListObjMetas;
use poprako_util::i18n::trl;

use crate::complex::chapter_port::export::ChapterExportComplex;
use crate::complex::chapter_port::perm::{
    ChapterExportAccess, ChapterPortPermComplex,
};
use crate::data::val::chapter_port::ExportChapterTranslationsVal;
use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::data::view::page_port::PageTranslationPortView;
use crate::data::view::unit_port::UnitTranslationPortView;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::ListPageInfos;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::oper::unit::ListUnitInfosByPageIds;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::stage::start_pending_stages;
use crate::value::chapter::stage::Stage;
use crate::value::chapter_port::ExportFormatSpec;
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Exports one chapter in every selected translation format.
#[instrument(level = "info", skip(nucl, repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn export<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    chapter_id: String,
    formats: ExportFormatSpec,
) -> BaseRest<ExportChapterTranslationsVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
    O: ObjDeptView<PageImage, C> + Sync,
{
    let actor_is_chapter_assignee =
        ensure_user_can_export::<C, R>(repo, &token, &chapter_id).await?;

    let chapter_info = GetChapterInfo {
        id: &chapter_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let comic_info = GetComicInfo {
        id: &chapter_info.comic_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let page_infos = ListPageInfos {
        chapter_id: &chapter_info.id,
    }
    .run_on(repo)
    .await?;

    let page_ids = page_infos
        .iter()
        .map(|page_info| page_info.id.as_str())
        .collect::<Vec<_>>();

    let unit_infos = ListUnitInfosByPageIds {
        page_ids: &page_ids,
    }
    .run_on(repo)
    .await?;

    let mut page_views = Vec::with_capacity(page_infos.len());

    let mut units_by_page_id = HashMap::<String, Vec<UnitInfo>>::new();

    let mut ext_by_page_id = HashMap::<String, String>::new();

    let page_ids = page_infos
        .iter()
        .map(|page_info| page_info.id.as_str())
        .collect::<Vec<_>>();

    let obj_metas = ListObjMetas::<PageImage>::new(&page_ids)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    for unit_info in unit_infos {
        //
        units_by_page_id
            .entry(unit_info.page_id.clone())
            .or_default()
            .push(unit_info);
    }

    for page_info in &page_infos {
        //
        if let Some(obj_meta) = obj_metas.get(&page_info.id) {
            ext_by_page_id.insert(page_info.id.clone(), obj_meta.ext.clone());
        }

        let unit_infos =
            units_by_page_id.remove(&page_info.id).unwrap_or_default();

        let unit_views = unit_infos
            .iter()
            .filter(|unit_info| unit_info.hidden_at.is_none())
            .enumerate()
            .map(|(index, unit_info)| {
                make_unit_export(page_info, index, unit_info)
            })
            .collect();

        page_views.push(PageTranslationPortView {
            page_id: page_info.id.clone(),
            page_index: page_info.index,
            units: unit_views,
        });

        let unit_infos = unit_infos
            .into_iter()
            .filter(|unit_info| unit_info.hidden_at.is_none())
            .collect();

        units_by_page_id.insert(page_info.id.clone(), unit_infos);
    }

    let poprako = ChapterTranslationPortView {
        chapter_id: chapter_info.id.clone(),
        chapter_index: chapter_info.index,
        chapter_subtitle: non_empty(&chapter_info.subtitle),
        comic_id: chapter_info.comic_id.clone(),
        comic_title: comic_info.title,
        pages: page_views,
    };

    let label_plus = formats.includes_label_plus().then(|| {
        //
        ChapterExportComplex::make_label_plus(
            &page_infos,
            &units_by_page_id,
            &ext_by_page_id,
        )
    });

    let val = ExportChapterTranslationsVal {
        label_plus,
        poprako: formats.includes_poprako().then_some(poprako),
    };

    persist_export_record(
        (nucl, repo),
        &chapter_info.id,
        token.user_id,
        formats,
        actor_is_chapter_assignee,
    )
    .await?;

    accept(val)
}

// Builds a [`UnitTranslationPortView`] from page and unit info.
fn make_unit_export(
    page_info: &PageInfo,
    index: usize,
    unit_info: &UnitInfo,
) -> UnitTranslationPortView {
    //
    // Convert one unit into export view fields used by downstream translators.
    UnitTranslationPortView {
        unit_id: unit_info.id.clone(),
        unit_index: index,
        page_id: page_info.id.clone(),
        page_index: page_info.index,
        x_coord: unit_info.coord.x_coord,
        y_coord: unit_info.coord.y_coord,
        is_bubble: unit_info.is_bubble,
        translated_text: unit_info.translated_text.clone(),
        translator_id: unit_info.last_translator_id.clone(),
        is_proofread: unit_info.is_proofread,
        proofread_text: unit_info.proofread_text.clone(),
        proofreader_id: unit_info.last_proofreader_id.clone(),
    }
}

// Returns [`Some`] with the text if non-empty, [`None`] otherwise.
fn non_empty(text: &str) -> Option<String> {
    //
    // Trim and normalize optional text fields before sending user-facing translation payloads.
    if text.trim().is_empty() {
        return None;
    }

    Some(text.to_string())
}

// Persists a completed export and conditionally starts typesetting/redraw in one transaction.
async fn persist_export_record<N, C, R>(
    (nucl, repo): (&N, &R),
    chapter_id: &str,
    actor_user_id: String,
    formats: ExportFormatSpec,
    actor_is_chapter_assignee: bool,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C> + ChapterWorkflowRecordRepo<C> + Send + Sync,
{
    let () = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                chapter_info.id.clone(),
                Some(actor_user_id.clone()),
                ChapterWorkflowRecordPayload::TranslationExported { formats },
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&workflow_record_entry),
            }
            .step_on(repo, context)
            .await?;

            if actor_is_chapter_assignee {
                //
                start_pending_stages(
                    repo,
                    context,
                    &chapter_info.id,
                    Some(actor_user_id),
                    ChapterWorkflowRecordOrigin::TranslationExport,
                    &[Stage::TypesetRedraw],
                )
                .await?;
            }

            accept(())
        })
        .await?;

    accept(())
}

// Authorizes chapter export and returns whether the caller is assigned to the chapter.
async fn ensure_user_can_export<C, R>(
    repo: &R,
    token: &UserToken,
    chapter_id: &str,
) -> BaseRest<bool>
where
    C: Context,
    R: TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
{
    let team_id = ResolveTeamId::Chapter { id: chapter_id }
        .run_on(repo)
        .await?;

    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &team_id,
    }
    .run_on(repo)
    .await?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    match (member_info.as_ref(), assignment_info.as_ref()) {
        //
        (Some(member_info), assignment_info) => {
            //
            ChapterPortPermComplex::ensure_user_can_export(
                &ChapterExportAccess::Member { member_info },
            )?;

            accept(assignment_info.is_some())
        }

        (None, Some(assignment_info)) => {
            //
            ChapterPortPermComplex::ensure_user_can_export(
                &ChapterExportAccess::Assignee { assignment_info },
            )?;

            accept(true)
        }

        (None, None) => {
            //
            let err_message = trl("error-chapter-port-export-perm-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Perm,
                err_message = %err_message,
                chapter_id = %chapter_id,
                user_id = %token.user_id,
                operation = "export",
                "expected error: chapter port export permission denied",
            );

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: err_message,
            })
        }
    }
}
