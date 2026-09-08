//! Team-scoped online-user lease use cases.

#[cfg(test)]
// Online-user use-case tests cover membership gates and team isolation.
mod tests;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::team::TeamPermComplex;
use crate::model::shared::user::UserToken;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::online_user::OnlineUserRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::online_user::{ListOnlineUserIds, MarkUserOnline};
use crate::part::repo::oper::user::UpdateUser;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Marks the authenticated user online in one team for ten minutes.
/// Updates user and membership activity before renewing the lease, without a transaction.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn mark_self_online<C, R>(
    (repo,): (&R,),
    token: UserToken,
    team_id: String,
) -> BaseRest<()>
where
    C: Context,
    R: MemberRepo<C> + UserRepo<C> + OnlineUserRepo + Sync,
{
    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &team_id,
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

    TeamPermComplex::ensure_user_can_mark_self_online(&member_info)?;

    UpdateUser::TouchLastActive { id: &token.user_id }
        .run_on(repo)
        .await?;

    MarkUserOnline {
        team_id: &team_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    accept(())
}

/// Lists active user identifiers for one team in ascending order.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_online_user_ids<C, R>(
    (repo,): (&R,),
    token: UserToken,
    team_id: String,
) -> BaseRest<Vec<String>>
where
    C: Context,
    R: MemberRepo<C> + OnlineUserRepo + Sync,
{
    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &team_id,
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

    TeamPermComplex::ensure_user_can_list_online_user_ids(&member_info)?;

    let online_user_ids =
        ListOnlineUserIds { team_id: &team_id }.run_on(repo).await?;

    accept(online_user_ids)
}
