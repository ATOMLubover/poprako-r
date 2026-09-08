//! Diesel-backed prom (promise) adapter.
//!
//! [`RdbProm`] writes deferred actions through the caller's transaction and
//! owns the background queue-consumer lifecycle.

// Internal organization of the `entity` module.
mod entity;
// Internal organization of the `actor` module.
mod actor;
// Internal organization of the `repo` module.
mod repo;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `test_shared` module.
mod test_shared;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `tests` module.
mod tests;

use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Step};
use time::OffsetDateTime;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use poprako_rdb_core::{RdbConn, RdbCore};

use crate::part::effect::Develop;
use crate::part::nucl::ReptRead;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::task::Task;
use crate::part_impl::prom::rdb_impl::actor::base::{ObjView, RdbPromActor};
use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntryRow;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// RDB-backed prom adapter for transactional deferral and queue processing.
///
/// Call [`close`](RdbProm::close) to stop polling and drain claimed work before
/// shutdown. Pending records remain durable for the next process start.
pub struct RdbProm {
    //
    /// Shared relational database core used to construct the queue consumer.
    core: RdbCore,
    /// Cancellation signal for the queue supervisor.
    token: CancellationToken,
    /// Completion signal set after every worker drains.
    done: watch::Receiver<bool>,
}

impl RdbProm {
    /// Stops polling and waits for all claimed work to finish.
    #[instrument(level = "info", skip_all)]
    pub async fn close(&self) {
        //
        self.token.cancel();

        let mut done = self.done.clone();

        if let Err(error) = done.wait_for(|is_done| *is_done).await {
            //
            tracing::error!(
                err = %error,
                "[RdbProm::close] background task ended without completion",
            );
        }
    }
}

impl Drop for RdbProm {
    // Cancels polling when the owner is dropped without an explicit close.
    fn drop(&mut self) {
        self.token.cancel();
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

/// Starts the queue consumer with its statically typed business dependencies.
#[must_use]
pub fn new<V, D>(core: RdbCore, obj_view: V, develop: D) -> RdbProm
where
    V: ObjView + Send + Sync + 'static,
    D: Develop + Send + Sync + 'static,
{
    let token = CancellationToken::new();

    let (done_send, done) = watch::channel(false);

    let rdb_prom = RdbProm { core, token, done };

    let actor = RdbPromActor::new(
        rdb_prom.core.clone(),
        obj_view,
        develop,
        rdb_prom.token.clone(),
    );

    tokio::spawn(async move {
        //
        actor.run().await;

        done_send.send_replace(true);
    });

    rdb_prom
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
