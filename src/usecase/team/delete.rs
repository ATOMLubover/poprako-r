//! Team deletion marking orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::team::TeamPermComplex;
use crate::model::shared::user::UserToken;
use crate::part::nucl::Serial;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::subtree_delete::{
    LockSubtreeDeleteScope, MarkSubtree, SubtreeRoot,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Atomically hides a Team and marks its hierarchy for background cleanup.
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
    R: SubtreeRepo<C> + MemberRepo<C> + Send + Sync,
{
    let () = nucl
        .coord(async move |context| {
            //
            let delete_scope = LockSubtreeDeleteScope {
                root: SubtreeRoot::Team { id: &id },
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
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: trl("error-team-admin-required"),
                });
            };

            TeamPermComplex::ensure_user_can_delete(&member_info)?;

            MarkSubtree {
                scope: &delete_scope,
            }
            .step_on(repo, context)
            .await?;

            accept(())
        })
        .await?;

    accept(())
}
