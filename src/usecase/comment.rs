//! Comment use cases — list and create team board comments.

/// Comment presentation assembly.
pub mod view;

#[cfg(test)]
// Unit tests that validate comment lifecycle and visibility constraints.
mod tests;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::comment::{CommentComplex, CommentPermComplex};
use crate::data::instr::comment::{CreateCommentInstr, ListCommentInfosInstr};
use crate::data::val::comment::CreateCommentVal;
use crate::data::view::comment::CommentInfoView;
use crate::model::read::spec::comment::CommentListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::comment::CommentEntry;
use crate::part::obj_dept::UserAvatar;
use crate::part::repo::comment::CommentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::comment::view::comment_info_views;

/// Lists comments under a team.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListCommentInfosInstr,
) -> BaseRest<Vec<CommentInfoView>>
where
    C: Context,
    R: CommentRepo<C> + MemberRepo<C> + Sync,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let comment_list_spec = Into::<CommentListSpec>::into(instr);

    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &comment_list_spec.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };

    CommentPermComplex::ensure_user_can_list_infos(&member_info)?;

    let comment_infos = ListCommentInfos {
        spec: &comment_list_spec,
    }
    .run_on(repo)
    .await?;

    let comment_info_vals = comment_info_views(obj_dept, comment_infos).await?;

    accept(comment_info_vals)
}

/// Creates a comment under a team.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn create<C, R>(
    repo: &R,
    token: UserToken,
    instr: CreateCommentInstr,
) -> BaseRest<CreateCommentVal>
where
    C: Context,
    R: CommentRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };

    CommentPermComplex::ensure_user_can_create(&member_info)?;

    let comment_entry = CommentEntry {
        id: CommentComplex::gen_id(),
        team_id: instr.team_id,
        user_id: token.user_id,
        content: instr.content,
    };

    let comment_info = CreateComment {
        entry: &comment_entry,
    }
    .run_on(repo)
    .await?;

    accept(CreateCommentVal {
        id: comment_info.id,
    })
}
