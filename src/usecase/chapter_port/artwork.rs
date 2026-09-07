//! Direct chapter artwork uploads, completion, and export.

#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::key::ObjGen;
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::model::url::ObjUrlSpec;
use poprako_obj_dept::oper::{
    GenObjSlot, GenObjUrls, ListObjMetas, MarkObjUploaded,
};
use poprako_obj_dept::{ObjDept, ObjDeptView};

use crate::complex::chapter::ChapterComplex;
use crate::complex::chapter_port::artwork::ChapterArtworkComplex;
use crate::config::artwork::ArtworkConfig;
use crate::data::instr::chapter_port::{
    AllocChapterArtworkInstr, MarkChapterArtworkUploadedInstr,
};
use crate::data::val::chapter_port::{
    AllocChapterArtworkVal, ExportChapterArtworkVal,
};
use crate::data::view::chapter_port::ChapterArtworkUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter::ChapterStageRepl;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::ChapterWorkflowCompletedEvent;
use crate::part::effect::{Develop, EffectEvent as _};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::ChapterArtwork;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded, UpdateChapterStage,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::chapter_port::perm::ensure_export_access;
use crate::value::artwork::{ArtworkHash, ChapterArtworkKey};
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Allocates a direct upload or returns the already available content's version.
#[instrument(level = "info", skip(nucl, repo, obj_dept, config, token), fields(actor_user_id = %token.user_id))]
pub async fn alloc_artwork<N, C, R, O>(
    (nucl, repo, obj_dept, config): (&N, &R, &O, &ArtworkConfig),
    token: UserToken,
    chapter_id: String,
    instr: AllocChapterArtworkInstr,
) -> BaseRest<AllocChapterArtworkVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ChapterRepo<C> + AssignmentRepo<C> + Send + Sync,
    O: ObjDept<ChapterArtwork, C> + Send + Sync,
{
    //
    ChapterArtworkComplex::ensure_allocation(
        *config,
        instr.new_byte_len,
        &instr.ext,
    )?;

    let allocation = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ensure_upload_access(
                repo,
                context,
                &chapter_info.id,
                &token.user_id,
            )
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let artwork_spec = ObjSlotSpec {
                dom: ChapterArtworkKey {
                    chapter_id: chapter_info.id.clone(),
                    ext: instr.ext,
                },
                hash: instr.artwork_hash.as_bytes(),
                content_type: "application/octet-stream",
                byte_len: instr.new_byte_len,
            };

            let artwork_slot = GenObjSlot::<ChapterArtwork>::new(&artwork_spec)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            let artwork_metas =
                ListObjMetas::<ChapterArtwork>::new(&[chapter_info
                    .id
                    .as_str()])
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            let artwork_meta =
                artwork_metas.get(&chapter_info.id).ok_or_else(|| {
                    //
                    BaseError::Unrecoverable {
                        message: "allocated artwork metadata is missing".into(),
                    }
                })?;

            let allocation = AllocChapterArtworkVal {
                artwork_ver: artwork_meta.key.ver,
                slot: artwork_slot.map(|slot| ChapterArtworkUploadSlotView {
                    put_url: slot.url.to_string(),
                    headers: slot.headers,
                }),
            };

            accept(allocation)
        })
        .await?;

    accept(allocation)
}

/// Confirms an exact generation and completes typesetting in the same transaction.
#[instrument(level = "info", skip(nucl, repo, obj_dept, develop, token), fields(actor_user_id = %token.user_id))]
pub async fn mark_artwork_uploaded<N, C, R, O, D>(
    (nucl, repo, obj_dept, develop): (&N, &R, &O, &D),
    token: UserToken,
    chapter_id: String,
    instr: MarkChapterArtworkUploadedInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ChapterRepo<C>
        + AssignmentRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
    O: ObjDept<ChapterArtwork, C> + Send + Sync,
    D: Develop + Send + Sync,
{
    let completed_chapter_id = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ensure_upload_access(
                repo,
                context,
                &chapter_info.id,
                &token.user_id,
            )
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let artwork_key = ObjGen {
                id: chapter_info.id.clone(),
                ver: instr.artwork_ver,
            };

            let marked = MarkObjUploaded::<ChapterArtwork>::new(&artwork_key)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            if !marked {
                //
                return Err(ChapterArtworkComplex::artwork_error(
                    ExpectedVariant::Args,
                    "error-stale-artwork-upload",
                ));
            }

            let previous_phase =
                chapter_info.stages.get_phase(Stage::TypesetRedraw);

            if previous_phase == StagePhase::Completed {
                return accept(None);
            }

            let stage_update = ChapterStageRepl {
                id: chapter_info.id.clone(),
                stages: chapter_info.stages.try_set_phase(
                    Stage::TypesetRedraw,
                    StagePhase::Completed,
                )?,
            };

            UpdateChapterStage {
                update: &stage_update,
            }
            .step_on(repo, context)
            .await?;

            let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                chapter_info.id.clone(),
                Some(token.user_id),
                ChapterWorkflowRecordPayload::StageTransitioned {
                    stage: Stage::TypesetRedraw,
                    previous_phase,
                    next_phase: StagePhase::Completed,
                    origin: ChapterWorkflowRecordOrigin::ArtworkUpload,
                },
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&workflow_record_entry),
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(Some(chapter_info.id))
        })
        .await?;

    if let Some(chapter_id) = completed_chapter_id {
        //
        Event::ChapterWorkflowCompleted {
            payload: ChapterWorkflowCompletedEvent {
                chapter_id,
                completed_stage: Stage::TypesetRedraw,
            },
        }
        .develop_on(develop)
        .await;
    }

    accept(())
}

/// Exports metadata and the original URL of the current available artwork.
#[instrument(level = "info", skip(nucl, repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn export_artwork<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<ExportChapterArtworkVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
    O: ObjDeptView<ChapterArtwork, C> + Sync,
{
    ensure_export_access::<C, R>(repo, &token, &chapter_id).await?;

    let chapter_info = GetChapterInfo {
        id: &chapter_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let artwork_metas =
        ListObjMetas::<ChapterArtwork>::new(&[chapter_info.id.as_str()])
            .run_on(obj_dept)
            .await
            .map_err(BaseError::from)?;

    let artwork_meta = artwork_metas
        .get(&chapter_info.id)
        .filter(|meta| meta.is_avail)
        .ok_or_else(|| {
            //
            ChapterArtworkComplex::artwork_error(
                ExpectedVariant::Args,
                "error-artwork-unavailable",
            )
        })?;

    let artwork_urls = GenObjUrls::<ChapterArtwork>::new(
        &artwork_metas,
        ObjUrlSpec::default().with_origin(),
    )
    .run_on(obj_dept)
    .await
    .map_err(BaseError::from)?;

    let download_url = artwork_urls
        .get(&chapter_info.id)
        .and_then(|urls| urls.origin_url.as_ref())
        .ok_or_else(|| BaseError::Unrecoverable {
            message: "available artwork has no original URL".into(),
        })?;

    let hash_bytes = artwork_meta.hash.as_slice().try_into().map_err(|_| {
        //
        BaseError::Unrecoverable {
            message: "invalid stored artwork SHA-256 length".into(),
        }
    })?;

    let artwork_export = ExportChapterArtworkVal {
        artwork_ver: artwork_meta.key.ver,
        artwork_hash: ArtworkHash::new(hash_bytes),
        ext: artwork_meta.ext.clone(),
        download_url: download_url.to_string(),
    };

    let () = nucl
        .coord(async move |context| {
            //
            let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                chapter_info.id,
                Some(token.user_id),
                ChapterWorkflowRecordPayload::ArtworkExported {
                    artwork_ver: artwork_meta.key.ver,
                },
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&workflow_record_entry),
            }
            .step_on(repo, context)
            .await?;

            accept(())
        })
        .await?;

    accept(artwork_export)
}

// Checks assignment evidence inside the owning chapter transaction.
async fn ensure_upload_access<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    user_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + Sync,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .step_on(repo, context)
    .await?;

    let assignment_info = assignment_info.ok_or_else(|| {
        //
        ChapterArtworkComplex::artwork_error(
            ExpectedVariant::Perm,
            "error-artwork-upload-role-required",
        )
    })?;

    ChapterArtworkComplex::ensure_user_can_upload(&assignment_info)
}
