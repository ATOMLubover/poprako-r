//! Chapter deletion use case and cascade orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_util::i18n::trl;

use crate::complex::chapter::perm::ChapterPermComplex;
use crate::model::read::proj::subtree_delete::SubtreeDeleteScope;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter::ChapterPatch;
use crate::part::nucl::Serial;
use crate::part::obj_dept::{ChapterArtwork, PageImage};
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter::{
    ListChapterInfosExcluded, UnpinOtherChapters, UpdateChapter,
};
use crate::part::repo::oper::comic::{
    TouchComicLastActive, UpdateComicChapterCount,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::subtree_delete::{
    DeleteSubtree, LockSubtreeDeleteScope, SubtreeRoot,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::subtree_delete::delete_chapter_objs;

/// Deletes one chapter and its descendant core records.
#[instrument(level = "info", skip(nucl, repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: SubtreeRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
    O: ObjDept<ChapterArtwork, C> + ObjDept<PageImage, C> + Send + Sync,
{
    nucl.coord(async move |context| {
        //
        let delete_scope = LockSubtreeDeleteScope {
            root: SubtreeRoot::Chapter { id: &id },
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

        ChapterPermComplex::ensure_user_can_delete(&member_info)?;

        delete_chapter_objs(repo, obj_dept, context, &delete_scope).await?;

        DeleteSubtree {
            scope: &delete_scope,
        }
        .step_on(repo, context)
        .await?;

        let SubtreeDeleteScope::Chapter {
            comic_id,
            was_pinned,
            ..
        } = delete_scope
        else {
            //
            return Err(BaseError::Unrecoverable {
                message: "chapter deletion returned a different scope".into(),
            });
        };

        if was_pinned {
            repin_latest_chapter(repo, context, &comic_id).await?;
        }

        UpdateComicChapterCount {
            id: &comic_id,
            delta: -1,
        }
        .step_on(repo, context)
        .await?;

        TouchComicLastActive { id: &comic_id }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}

// Pin the newest remaining chapter after deleting the pinned chapter.
async fn repin_latest_chapter<C, R>(
    repo: &R,
    context: &mut C,
    comic_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: ChapterRepo<C> + Sync,
{
    let chapter_infos = ListChapterInfosExcluded { comic_id }
        .step_on(repo, context)
        .await?;

    let Some(chapter_info) = chapter_infos.first() else {
        return accept(());
    };

    let chapter_info_update = ChapterPatch {
        id: chapter_info.id.clone(),
        subtitle: None,
        pin: Some(true),
    };

    UpdateChapter {
        update: &chapter_info_update,
    }
    .step_on(repo, context)
    .await?;

    UnpinOtherChapters {
        comic_id: &chapter_info.comic_id,
        excluded_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}
