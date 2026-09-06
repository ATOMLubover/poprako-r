//! Chapter port permission and access orchestration.

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter_port::perm::{
    ChapterExportAccess, ChapterPortPermComplex,
};
use crate::model::shared::user::UserToken;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Authorizes chapter export and returns whether the caller is assigned to the chapter.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn ensure_export_access<C, R>(
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
            ChapterPortPermComplex::ensure_user_can_export_translation(
                &ChapterExportAccess::Member { member_info },
            )?;

            accept(assignment_info.is_some())
        }

        (None, Some(assignment_info)) => {
            //
            ChapterPortPermComplex::ensure_user_can_export_translation(
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
