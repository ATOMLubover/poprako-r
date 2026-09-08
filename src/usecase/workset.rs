//! Workset use cases — create, read, update, list, and deletion.

/// Workset use-case test helpers.
#[cfg(test)]
pub mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::workset::{WorksetComplex, WorksetPermComplex};
use crate::data::instr::workset::{
    CreateWorksetInstr, ListWorksetInfosInstr, UpdateWorksetInfoInstr,
};
use crate::data::val::workset::CreateWorksetVal;
use crate::data::view::workset::WorksetInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::workset::{WorksetEntry, WorksetRepl};
use crate::part::nucl::{ReptRead, Serial};
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::subtree_delete::{
    LockSubtreeDeleteScope, MarkSubtree, SubtreeRoot,
};
use crate::part::repo::oper::team::AllocTeamWorksetIndex;
use crate::part::repo::oper::workset::{
    CreateWorkset, GetWorksetInfo, ListWorksetInfos, UpdateWorkset,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Creates a new workset inside a team.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateWorksetInstr,
) -> BaseRest<CreateWorksetVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
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
            message: trl("error-team-admin-required"),
        });
    };

    WorksetPermComplex::ensure_user_can_create(&member_info)?;

    let workset_id = nucl
        .coord(async move |context| {
            //
            let index = AllocTeamWorksetIndex { id: &instr.team_id }
                .step_on(repo, context)
                .await?;

            let workset_entry = WorksetEntry {
                id: WorksetComplex::gen_id(),
                team_id: instr.team_id,
                index,
                name: instr.name,
                description: instr.description,
            };

            let workset_info = CreateWorkset {
                entry: &workset_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(workset_info.id)
        })
        .await?;

    accept(CreateWorksetVal { id: workset_id })
}

/// Fetches a workset by ID.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<WorksetInfoView>
where
    C: Context,
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_workset(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    WorksetPermComplex::ensure_user_can_get_info(&member_info)?;

    let workset_info = GetWorksetInfo { id: &id }.run_on(repo).await?;

    accept(workset_info.into())
}

/// Lists worksets for a team.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListWorksetInfosInstr,
) -> BaseRest<Vec<WorksetInfoView>>
where
    C: Context,
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
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

    WorksetPermComplex::ensure_user_can_list_infos(&member_info)?;

    let workset_infos = ListWorksetInfos {
        team_id: &instr.team_id,
        offset: instr.offset,
        limit: instr.limit,
    }
    .run_on(repo)
    .await?;

    accept(workset_infos.into_iter().map(Into::into).collect())
}

/// Updates a workset's name and description.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: UpdateWorksetInfoInstr,
) -> BaseRest<()>
where
    C: Context,
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_workset(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.id,
    )
    .await?;

    WorksetPermComplex::ensure_user_can_update_info(&member_info)?;

    let workset_info_update = WorksetRepl {
        id: instr.id,
        name: instr.name,
        description: instr.description,
    };

    UpdateWorkset {
        update: &workset_info_update,
    }
    .run_on(repo)
    .await?;

    accept(())
}

/// Deletes a workset and its child data.
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
                root: SubtreeRoot::Workset { id: &id },
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

            WorksetPermComplex::ensure_user_can_delete(&member_info)?;

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
