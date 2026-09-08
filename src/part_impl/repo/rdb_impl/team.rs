//! RDB-backed team repository.

// Row locking and workset index allocation.
mod coordination;
// Team lifecycle and profile persistence.
mod info;
// Team include resolution.
mod resolve;

/// Team RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::team::TeamInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, GetTeamInfo, GetTeamInfoExcluded,
    ListTeamInfos, LockTeam, UpdateTeam,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::team::coordination::{
    increment_workset_next_index, lock_team,
};
use crate::part_impl::repo::rdb_impl::team::info::{
    create, get_info_by_id, get_info_excluded, list_infos, update_info,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<CreateTeam<'_>> for HybRepo {
    // Map team creation orchestration failures to the shared base error type.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Submit a team create request through repository core to keep one call path.
    async fn run(
        &self,
        oper: &CreateTeam<'_>,
    ) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.rdb_core, create, oper.entry)
    }
}

impl Run<GetTeamInfo<'_>> for HybRepo {
    // Use the common base error for team info reads.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve a team read request by ID and return its details.
    async fn run(
        &self,
        oper: &GetTeamInfo<'_>,
    ) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.rdb_core, get_info_by_id, oper.id)
    }
}

impl Run<ListTeamInfos<'_>> for HybRepo {
    // Keep list query failures on a single repository-level error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return filtered and paginated team lists based on caller-provided criteria.
    async fn run(
        &self,
        oper: &ListTeamInfos<'_>,
    ) -> Result<Vec<TeamInfo>, Self::Error> {
        submit_query!(self.rdb_core, list_infos, oper.spec)
    }
}

impl Run<UpdateTeam<'_>> for HybRepo {
    // Keep update orchestration failures compatible with other team operations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Route team mutation data into the corresponding SQL update handler.
    async fn run(&self, oper: &UpdateTeam<'_>) -> BaseRest<()> {
        submit_query!(self.rdb_core, update_info, oper.repl)
    }
}

impl<L> Step<CreateTeam<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Convert repository step failures to base error during transaction execution.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist a new team row within an open transaction and return persisted info.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateTeam<'_>,
    ) -> BaseRest<TeamInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<UpdateTeam<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional team updates on the same base error contract.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply either profile updates or avatar flag updates in current transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateTeam<'_>,
    ) -> BaseRest<()> {
        update_info(context.conn(), oper.repl).await
    }
}

impl<L> Step<GetTeamInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Preserve consistent error typing for locked team detail fetches.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load team info with row lock and exclusion rules for transactional safety.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetTeamInfoExcluded<'_>,
    ) -> BaseRest<TeamInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<LockTeam<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep lock contention errors on the shared repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Acquire row lock for update sequencing before sensitive team writes.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &LockTeam<'_>,
    ) -> BaseRest<()> {
        lock_team(context.conn(), oper.id).await
    }
}

impl<L> Step<AllocTeamWorksetIndex<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep index allocation failures mapped to repository base errors.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Atomically increment and return previous index for next workset reservation.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &AllocTeamWorksetIndex<'_>,
    ) -> BaseRest<usize> {
        increment_workset_next_index(context.conn(), oper.id).await
    }
}
