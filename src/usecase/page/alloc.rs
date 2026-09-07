//! Chapter page-manifest and page-image allocation.

// Authoritative chapter manifest transaction.
mod manifest;

/// Manifest validation rules.
pub mod validation;

use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::model::slot::{ObjSlot, ObjSlotSpec};
use poprako_obj_dept::oper::GenObjSlot;
use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::config::image::ImageConfig;
use crate::data::instr::page::{AllocChapterPagesInstr, AllocPageImageInstr};
use crate::data::val::page::{AllocChapterPagesVal, AllocatedPageVal};
use crate::data::view::image::ImageUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::model::write::page::{PageImageSpec, PageRawIdentsRepl};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageImage;
use crate::part::prom::Prom;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::task::Task;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, UpdatePageRawIdents,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::page::alloc::manifest::apply_manifest;
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash, ImageKind, PageImageKey};

/// Allocates the authoritative page manifest and its required image uploads.
#[instrument(level = "info", skip(nucl, repo, prom, obj_dept, image_config, token), fields(actor_user_id = %token.user_id))]
pub async fn alloc_chapter_pages<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    instr: AllocChapterPagesInstr,
) -> BaseRest<AllocChapterPagesVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    let AllocChapterPagesInstr { chapter_id, pages } = instr;

    let page_specs = pages
        .into_iter()
        .map(PageImageSpec::from)
        .collect::<Vec<_>>();

    let page_count = validation::validate_page_specs(
        image_config,
        &page_specs,
        &chapter_id,
        &token.user_id,
    )?;

    ensure_alloc_perm::<C, R>(repo, &token, &chapter_id).await?;

    let allocs = nucl
        .coord(async move |context| {
            //
            apply_manifest(
                (repo, prom, obj_dept, context),
                &token.user_id,
                &chapter_id,
                &page_specs,
                page_count,
            )
            .await
        })
        .await?;

    let pages = allocs
        .into_iter()
        .map(|(page_id, index, image_hash, ext, obj_slot)| {
            //
            alloc_val(PageAlloc {
                page_id,
                index,
                image_hash,
                ext,
                obj_slot,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept(AllocChapterPagesVal { pages })
}

/// Allocates an image generation for one page.
#[instrument(level = "info", skip(nucl, repo, prom, obj_dept, image_config, token), fields(actor_user_id = %token.user_id))]
pub async fn alloc_image<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    id: String,
    instr: AllocPageImageInstr,
) -> BaseRest<AllocatedPageVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    PageComplex::ensure_raw_ident(instr.raw_ident.as_deref())?;

    ImageComplex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::PageImage,
    )?;

    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    ensure_alloc_perm::<C, R>(repo, &token, &page_info.chapter_id).await?;

    let page_id = id.clone();

    let page_index = page_info.index;

    let image_hash = instr.image_hash.clone();

    let image_ext = instr.ext;

    let obj_slot = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &page_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            GetPageInfoExcluded { id: &id }
                .step_on(repo, context)
                .await?;

            let obj_spec = ObjSlotSpec {
                dom: PageImageKey {
                    chapter_id: page_info.chapter_id.clone(),
                    page_id: id.clone(),
                    ext: instr.ext,
                },
                hash: instr.image_hash.as_bytes(),
                content_type: instr.ext.content_type(),
                byte_len: instr.new_byte_len,
            };

            let obj_slot = GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            let raw_ident_repl = PageRawIdentsRepl {
                idents: &[(id.as_str(), instr.raw_ident.as_deref())],
            };

            UpdatePageRawIdents {
                repl: &raw_ident_repl,
            }
            .step_on(repo, context)
            .await?;

            if obj_slot.is_some() {
                //
                let advance_id = next_snowflake_id();

                let advance_payload = TaskPayload::Chapter {
                    payload: ChapterPayload::TryAdvanceRawProvideStage {
                        chapter_id: page_info.chapter_id.clone(),
                        actor_user_id: Some(token.user_id.clone()),
                    },
                };

                let advance_task = Task {
                    id: &advance_id,
                    payload: &advance_payload,
                    delay: Some(Duration::from_mins(20)),
                };

                Defer::new(advance_task).step_on(prom, context).await?;
            }

            accept(obj_slot)
        })
        .await?;

    alloc_val(PageAlloc {
        page_id,
        index: page_index,
        image_hash,
        ext: image_ext,
        obj_slot,
    })
}

// Converts one internal allocation into a response view.
struct PageAlloc {
    //
    // Stable page identifier.
    page_id: String,
    // Final page position.
    index: usize,
    // Expected content hash.
    image_hash: ImageHash,
    // Expected image suffix.
    ext: ImageExt,
    // Optional new upload capability.
    obj_slot: Option<ObjSlot>,
}

// Converts one internal allocation into a response view.
fn alloc_val(alloc: PageAlloc) -> BaseRest<AllocatedPageVal> {
    //
    let slot = alloc.obj_slot.map(|obj_slot| ImageUploadSlotView {
        put_url: obj_slot.url.to_string(),
        image_ver: obj_slot.key.ver,
        headers: obj_slot.headers,
    });

    let index = u32::try_from(alloc.index).map_err(|_| {
        //
        BaseError::Unrecoverable {
            message: "page index is out of range".into(),
        }
    })?;

    accept(AllocatedPageVal {
        page_id: alloc.page_id,
        index,
        image_hash: alloc.image_hash,
        ext: alloc.ext,
        slot,
    })
}

// Validates the caller and current chapter state.
async fn ensure_alloc_perm<C, R>(
    repo: &R,
    token: &UserToken,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-page-alloc-role-required"),
        });
    };

    PagePermComplex::ensure_user_can_alloc(&assignment_info)
}
