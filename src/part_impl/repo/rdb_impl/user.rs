//! RDB-backed user repository.

// Account creation, credentials, activity, and deletion.
mod account;
// Profile reads, locks, and updates.
mod info;

/// User RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, UpdateUser,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::user::account::{
    create, delete, get_credential_by_qid, touch_last_active,
    update_password_hash,
};
use crate::part_impl::repo::rdb_impl::user::info::{
    find_info_by_qid, get_info_by_id, get_info_by_id_excluded, update_info,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<GetUserInfo<'_>> for HybRepo {
    // Use `BaseError` for non-transactional repository reads.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route read by ID into the shared `submit_query!` orchestration.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetUserInfo<'_>,
    ) -> Result<UserInfo, Self::Error> {
        submit_query!(self.rdb_core, get_info_by_id, oper.id)
    }
}

impl Run<GetUserCredential<'_>> for HybRepo {
    // Use `BaseError` for non-transactional credential reads.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route credential read by QID to the shared repository query path.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetUserCredential<'_>,
    ) -> Result<UserCredential, Self::Error> {
        submit_query!(self.rdb_core, get_credential_by_qid, oper.qid)
    }
}

impl Run<FindUserInfo<'_>> for HybRepo {
    // Use `BaseError` for non-transactional optional reads.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route optional user lookup by QID to shared query layer.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &FindUserInfo<'_>,
    ) -> Result<Option<UserInfo>, Self::Error> {
        submit_query!(self.rdb_core, find_info_by_qid, oper.qid)
    }
}

impl Run<UpdateUser<'_>> for HybRepo {
    // Use `BaseError` for non-transactional user mutations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Map each update variant to a dedicated helper with explicit argument flow.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateUser<'_>) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateUser::TouchLastActive { id } => {
                submit_query!(self.rdb_core, touch_last_active, id)
            }

            UpdateUser::Info { repl } => {
                submit_query!(self.rdb_core, update_info, repl)
            }

            UpdateUser::PasswordHash { repl } => {
                submit_query!(self.rdb_core, update_password_hash, repl)
            }
        }
    }
}

impl<L> Step<CreateUser<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transaction-scoped operations on one repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Insert new user rows inside provided transaction context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateUser<'_>,
    ) -> BaseRest<UserInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<FindUserInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transaction-scoped reads on one repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve soft-miss lookup inside caller-owned transaction context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &FindUserInfo<'_>,
    ) -> BaseRest<Option<UserInfo>> {
        find_info_by_qid(context.conn(), oper.qid).await
    }
}

impl<L> Step<UpdateUser<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transaction-scoped updates on one repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Dispatch each mutable user operation to one explicit DB helper.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateUser<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateUser::Info { repl } => {
                update_info(context.conn(), repl).await
            }

            UpdateUser::TouchLastActive { id } => {
                touch_last_active(context.conn(), id).await
            }

            UpdateUser::PasswordHash { repl } => {
                update_password_hash(context.conn(), repl).await
            }
        }
    }
}

impl<L> Step<GetUserInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transaction-scoped exclusive reads on one repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Read user row with lock for callers that mutate next.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetUserInfoExcluded<'_>,
    ) -> BaseRest<UserInfo> {
        get_info_by_id_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<DeleteUser<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transaction-scoped deletion on one repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Execute user deletion as part of ongoing transaction flow.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteUser<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}
