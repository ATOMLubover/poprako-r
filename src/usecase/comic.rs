//! Comic use cases — create, read, update, cover management, and deletion.
/// Cover allocation use case.
pub mod alloc;
/// Comic cover upload confirmation use case.
pub mod cover;
/// Comic listing use cases.
pub mod list;
/// Comic presentation assembly.
pub mod view;

/// Comic use-case test helpers.
#[cfg(test)]
pub mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::data::instr::comic::{CreateComicInstr, UpdateComicInfoInstr};
use crate::data::val::comic::CreateComicVal;
use crate::data::view::comic::ComicInfoView;
use crate::model::read::proj::subtree_delete::SubtreeDeleteScope;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::chapter::ChapterEntry;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::model::write::comic::{ComicEntry, ComicRepl};
use crate::part::nucl::{ReptRead, Serial};
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::CreateAssignment;
use crate::part::repo::oper::chapter::{CreateChapter, UnpinOtherChapters};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, GetComicInfo, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::subtree_delete::{
    LockSubtreeDeleteScope, MarkSubtree, SubtreeRoot,
};
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, UpdateWorksetComicCount,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::comic::view::comic_info_view;
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;
use crate::value::role::RoleMask;

/// Creates a comic with its first chapter and creator assignment.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateComicInstr,
) -> BaseRest<CreateComicVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let member_info = MemberLoader::load_info_from_workset(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.workset_id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_create(
        &member_info,
        instr.preset_assignment_roles,
    )?;

    let (comic_id, chapter_id) = nucl
        .coord(async move |context| {
            //
            let index = AllocWorksetComicIndex {
                id: &instr.workset_id,
            }
            .step_on(repo, context)
            .await?;

            let comic_entry = ComicEntry {
                id: ComicComplex::gen_id(),
                workset_id: instr.workset_id,
                index,
                title: instr.title,
                author: instr.author,
                description: instr.description,
                creator_id: token.user_id.clone(),
            };

            let comic_info = CreateComic {
                entry: &comic_entry,
            }
            .step_on(repo, context)
            .await?;

            UpdateWorksetComicCount {
                id: &comic_entry.workset_id,
                delta: 1,
            }
            .step_on(repo, context)
            .await?;

            let chapter_index = AllocComicChapterIndex { id: &comic_info.id }
                .step_on(repo, context)
                .await?;

            let subtitle = ChapterComplex::subtitle_or_default(
                instr.first_chapter_subtitle,
                chapter_index,
            );

            let chapter_entry = ChapterEntry {
                id: ChapterComplex::gen_id(),
                comic_id: comic_info.id.clone(),
                is_pinned: true,
                index: chapter_index,
                subtitle,
                creator_id: token.user_id.clone(),
            };

            let chapter_info = CreateChapter {
                entry: &chapter_entry,
            }
            .step_on(repo, context)
            .await?;

            UnpinOtherChapters {
                comic_id: &chapter_info.comic_id,
                excluded_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            UpdateComicChapterCount {
                id: &chapter_info.comic_id,
                delta: 1,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            create_creator_assignment(
                repo,
                context,
                &chapter_info.id,
                token.user_id,
                instr.preset_assignment_roles,
            )
            .await?;

            accept((comic_info.id, chapter_info.id))
        })
        .await?;

    accept(CreateComicVal {
        id: comic_id,
        chapter_id,
    })
}

/// Fetches a comic by ID with cover URL resolution.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn get_info<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<ComicInfoView>
where
    C: Context,
    R: ComicRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_get_info(&member_info)?;

    let comic_info = GetComicInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    comic_info_view(repo, obj_dept, comic_info).await
}

/// Updates a comic's title, author, and description.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: UpdateComicInfoInstr,
) -> BaseRest<()>
where
    C: Context,
    R: ComicRepo<C> + TeamRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_update_info(&member_info)?;

    let comic_info = GetComicInfo {
        id: &instr.id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    ComicComplex::ensure_comic_writable(&comic_info)?;

    let comic_info_update = ComicRepl {
        id: instr.id,
        title: instr.title,
        author: instr.author,
        description: instr.description,
    };

    UpdateComic {
        update: &comic_info_update,
    }
    .run_on(repo)
    .await?;

    accept(())
}

/// Deletes a comic and updates the parent workset counter.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: SubtreeRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
{
    let () = nucl
        .coord(async move |context| {
            //
            let delete_scope = LockSubtreeDeleteScope {
                root: SubtreeRoot::Comic { id: &id },
            }
            .step_on(repo, context)
            .await?;

            let member_info = FindMemberInfo {
                user_id: &token.user_id,
                team_id: delete_scope.team_id(),
            }
            .step_on(repo, context)
            .await?;

            let Some(member_info) = member_info else {
                //
                let err_message = trl("error-team-member-required");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    "expected error: team membership required",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                });
            };

            ComicPermComplex::ensure_user_can_delete(&member_info)?;

            MarkSubtree {
                scope: &delete_scope,
            }
            .step_on(repo, context)
            .await?;

            let SubtreeDeleteScope::Comic { workset_id, .. } = delete_scope
            else {
                //
                return Err(BaseError::Unrecoverable {
                    message: "comic deletion returned a different scope".into(),
                });
            };

            UpdateWorksetComicCount {
                id: &workset_id,
                delta: -1,
            }
            .step_on(repo, context)
            .await?;

            accept(())
        })
        .await?;

    accept(())
}

// Creates the initial assignment for a comic creator.
async fn create_creator_assignment<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    user_id: String,
    preset_assignment_roles: Option<RoleMask>,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + ChapterWorkflowRecordRepo<C> + Sync,
{
    let assignment_entry = AssignmentEntry {
        id: AssignmentComplex::gen_id(),
        chapter_id: chapter_id.to_owned(),
        user_id: user_id.clone(),
        roles: AssignmentComplex::creator_roles(preset_assignment_roles),
    };

    CreateAssignment {
        entry: &assignment_entry,
    }
    .step_on(repo, context)
    .await?;

    let workflow_record_entry = ChapterWorkflowRecordEntry::new(
        chapter_id.to_owned(),
        Some(user_id),
        ChapterWorkflowRecordPayload::ChapterCreated,
    );

    CreateChapterWorkflowRecords {
        entries: std::slice::from_ref(&workflow_record_entry),
    }
    .step_on(repo, context)
    .await?;

    accept(())
}
