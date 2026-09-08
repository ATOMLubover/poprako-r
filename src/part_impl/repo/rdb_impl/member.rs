//! RDB-backed member repository.

// Transaction-scoped member operations.
mod step_impl;

/// Member RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::member::MemberInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, DeleteUserMemberships, FindMemberInfo,
    GetMemberInfo, ListMemberInfos, LockTeamMemberInfos, UpdateMember,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::member::step_impl::{
    create, delete, delete_user_memberships, find_info_by_user_id_and_team_id,
    get_info_by_id, list_infos, list_infos_by_team_id_excluded,
    list_infos_by_user_id, update_role, update_user_nickname,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<FindMemberInfo<'_>> for HybRepo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Find a member by user and team IDs.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &FindMemberInfo<'_>,
    ) -> BaseRest<Option<MemberInfo>> {
        //
        submit_query!(
            self.rdb_core,
            find_info_by_user_id_and_team_id,
            oper.user_id,
            oper.team_id
        )
    }
}

impl Run<ListMemberInfos<'_>> for HybRepo {
    // Non-transactional query path for listing member infos.
    //
    // It picks the right query variant based on whether the caller provided
    // filters or a user-only scope.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Route list variants to the corresponding read-only query in the current repo.
    async fn run(
        &self,
        oper: &ListMemberInfos<'_>,
    ) -> BaseRest<Vec<MemberInfo>> {
        //
        match oper {
            //
            ListMemberInfos::Spec { spec } => {
                submit_query!(self.rdb_core, list_infos, spec)
            }

            ListMemberInfos::User { user_id } => {
                submit_query!(self.rdb_core, list_infos_by_user_id, user_id)
            }
        }
    }
}

impl Run<GetMemberInfo<'_, '_>> for HybRepo {
    // Non-transactional query path for loading one member info by identity.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve one member info by id through a submit-query read path.
    async fn run(&self, oper: &GetMemberInfo<'_, '_>) -> BaseRest<MemberInfo> {
        submit_query!(self.rdb_core, get_info_by_id, oper.id, oper.incls)
    }
}

impl<L> Step<CreateMember<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Create a new member row inside the active transaction context.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Invoke `create` step with raw entry payload and return created info.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateMember<'_>,
    ) -> BaseRest<MemberInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<UpdateMember<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Apply an in-transaction member update request.
    //
    // Supported requests either adjust nickname or change role based on branch.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Branch to nickname/role update path and execute inside the transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateMember<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateMember::UserNickname { repl } => {
                //
                update_user_nickname(
                    context.conn(),
                    &repl.user_id,
                    &repl.user_nickname,
                )
                .await
            }

            UpdateMember::Role { update } => {
                update_role(context.conn(), update).await
            }
        }
    }
}

impl<L> Step<ListMemberInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Transactional list path for member info queries.
    //
    // Reuses the same selection modes as non-transactional `run` execution,
    // but executes through an explicit DB connection context.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Execute list query within the transaction and return full member info set.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListMemberInfos<'_>,
    ) -> BaseRest<Vec<MemberInfo>> {
        //
        match oper {
            //
            ListMemberInfos::Spec { spec } => {
                list_infos(context.conn(), spec).await
            }

            ListMemberInfos::User { user_id } => {
                list_infos_by_user_id(context.conn(), user_id).await
            }
        }
    }
}

impl<L> Step<FindMemberInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Transactional lookup for a member by `(user_id, team_id)`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Find one member-row mapping by both user and team identifiers.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &FindMemberInfo<'_>,
    ) -> BaseRest<Option<MemberInfo>> {
        //
        find_info_by_user_id_and_team_id(
            context.conn(),
            oper.user_id,
            oper.team_id,
        )
        .await
    }
}

impl<L> Step<GetMemberInfo<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Transactional lookup for a full member info by id and requested includes.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Fetch a member info with requested include options within the transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetMemberInfo<'_, '_>,
    ) -> BaseRest<MemberInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl<L> Step<LockTeamMemberInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Transactional list for member infos excluding one side of relation.
    //
    // One branch excludes all members for a user, the other excludes a full team.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Execute exclusion-based member list variants inside the same tx context.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &LockTeamMemberInfos<'_>,
    ) -> BaseRest<Vec<MemberInfo>> {
        list_infos_by_team_id_excluded(context.conn(), oper.team_id).await
    }
}

impl<L> Step<DeleteMember<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Transactional delete operation for a single member by id.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Delete the member row for the provided identifier from the DB transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteMember<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<L> Step<DeleteUserMemberships<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required Orchestra execution level.
    type Level = ReptRead;

    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteUserMemberships<'_>,
    ) -> BaseRest<()> {
        delete_user_memberships(context.conn(), oper.user_id).await
    }
}
