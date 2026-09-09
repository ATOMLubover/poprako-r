//! Diesel-backed prom (promise) adapter.
//!
//! [`RdbProm`] writes deferred actions through the caller's transaction and
//! is independent of the explicitly started queue consumer.

// Internal organization of the `entity` module.
// Diesel queue entities.
mod entity;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `test_shared` module.
mod test_shared;

/// Prom-consumer actor implementation.
pub mod actor;
/// Prom queue repository implementation.
pub mod repo;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `tests` module.
mod tests;

use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::part::nucl::ReptRead;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::task::Task;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntryRow;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// Transactional deferred-task writer, independent of background consumption.
#[derive(Clone, Copy, Default)]
pub struct RdbProm;

impl RdbProm {
    /// Constructs a writer without starting a consumer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<'a, L> Step<Defer<'a, String, TaskPayload, ()>, RdbContext<L>> for RdbProm
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &Defer<'a, String, TaskPayload, ()>,
    ) -> BaseRest<()> {
        defer(context.conn(), &oper.task).await
    }
}

impl<'t, 'a, L> Step<DeferBatch<'t, 'a, String, TaskPayload, ()>, RdbContext<L>>
    for RdbProm
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeferBatch<'t, 'a, String, TaskPayload, ()>,
    ) -> BaseRest<()> {
        defer_batch(context.conn(), oper.tasks).await
    }
}

// Implements defer.
#[instrument(level = "info", skip_all)]
async fn defer(
    conn: &mut RdbConn,
    task: &Task<'_, String, TaskPayload>,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = LocalMessageEntryRow::from_task(task, now)?;

    diesel::insert_into(t_local_message::table)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Implements defer batch.
#[instrument(level = "info", skip_all)]
async fn defer_batch(
    conn: &mut RdbConn,
    tasks: &[Task<'_, String, TaskPayload>],
) -> BaseRest<()> {
    //
    if tasks.is_empty() {
        return accept(());
    }

    let now = OffsetDateTime::now_utc();

    let entries = tasks
        .iter()
        .map(|task| LocalMessageEntryRow::from_task(task, now))
        .collect::<BaseRest<Vec<_>>>()?;

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}
