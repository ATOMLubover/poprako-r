//! Repository for prom task handling and `t_local_message` lifecycle operations.
//!
//! These operation types and [`Step`] implementations are used exclusively
//! by the background actor. They are NOT part of the public [`Prom`]
//! port trait — only producer-side defer operations are exposed through the
//! port system.
//!
//! [`Prom`]: crate::part::prom::Prom

/// RDB prom repository integration tests.
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
pub mod tests;

use diesel::prelude::{
    BoolExpressionMethods as _, ExpressionMethods as _, QueryDsl as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Oper, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::part::nucl::ReptRead;
use crate::part_impl::prom::rdb_impl::entity::{
    LocalMessageRow, LocalMessageStatus,
};
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// Poll the oldest visible pending record from each topic without processing work.
/// A delayed retry is excluded before the per-topic selection, allowing later
/// visible work from the same topic to advance. A processing record blocks only
/// its own topic so separate application instances cannot consume that topic
/// concurrently.
#[derive(Oper)]
#[oper(output = Vec<LocalMessageRow>)]
pub struct PollPending;

/// Try to claim a record (status Pending → Processing).
/// Returns `true` if the claim succeeded (i.e. the row was still
/// Pending), `false` if another worker claimed it first.
#[derive(Oper)]
#[oper(output = bool)]
pub struct ClaimPending<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to claim.
    id: &'a str,
    /// Lease observed by the poller.
    lease: i64,
}

impl<'a> ClaimPending<'a> {
    /// Builds an operation that claims the observed message attempt.
    pub const fn new(id: &'a str, lease: i64) -> Self {
        Self { id, lease }
    }
}

/// Mark a record as successfully completed.
#[derive(Oper)]
#[oper(output = ())]
pub struct CompleteMessage<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to mark complete.
    id: &'a str,
    /// Lease owned by the worker attempt.
    lease: i64,
}

impl<'a> CompleteMessage<'a> {
    /// Builds an operation that completes the identified worker attempt.
    pub const fn new(id: &'a str, lease: i64) -> Self {
        Self { id, lease }
    }
}

/// Mark a record as dead with an error message.
#[derive(Oper)]
#[oper(output = ())]
pub struct FailMessage<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to mark as failed.
    id: &'a str,
    /// Lease owned by the worker attempt.
    lease: i64,
    /// Error description attached to the failure record.
    error: &'a str,
}

impl<'a> FailMessage<'a> {
    /// Builds an operation that permanently fails the message identified by `id`.
    pub const fn new(id: &'a str, lease: i64, err_msg: &'a str) -> Self {
        //
        Self {
            id,
            lease,
            error: err_msg,
        }
    }
}

/// Reset one failed processing attempt back to pending for a later retry.
#[derive(Oper)]
#[oper(output = ())]
pub struct RetryMessage<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to retry.
    id: &'a str,
    /// Lease owned by the worker attempt.
    lease: i64,
    /// Error description logged from the previous attempt.
    error: &'a str,
    /// Timestamp after which the retry becomes visible for processing.
    visible_at: &'a OffsetDateTime,
    /// Amount consumed from the failure retry budget.
    retry_delta: i64,
}

impl<'a> RetryMessage<'a> {
    /// Builds an operation that schedules the message identified by `id` for retry.
    pub const fn new(
        id: &'a str,
        lease: i64,
        err_msg: &'a str,
        visible_at: &'a OffsetDateTime,
        retry_delta: i64,
    ) -> Self {
        //
        Self {
            id,
            lease,
            error: err_msg,
            visible_at,
            retry_delta,
        }
    }
}

/// Reset processing records stuck before a cutoff timestamp.
#[derive(Oper)]
#[oper(output = ())]
pub struct ResetStuck<'a> {
    /// Cutoff timestamp; any record stuck in Processing before this is reset.
    before: &'a OffsetDateTime,
}

impl<'a> ResetStuck<'a> {
    /// Builds an operation that resets messages stuck before the cutoff.
    pub const fn new(before: &'a OffsetDateTime) -> Self {
        Self { before }
    }
}

/// Deletes completed and dead records after their independent retention cutoffs.
#[derive(Oper)]
#[oper(output = usize)]
pub struct PurgeCompleted<'a> {
    //
    // Internal state field `completed_before`.
    /// Cutoff timestamp for completed records to purge.
    completed_before: &'a OffsetDateTime,
    /// Cutoff timestamp for dead records to purge.
    dead_before: &'a OffsetDateTime,
}

impl<'a> PurgeCompleted<'a> {
    /// Builds terminal-message purge cutoffs.
    pub const fn new(
        completed_before: &'a OffsetDateTime,
        dead_before: &'a OffsetDateTime,
    ) -> Self {
        //
        Self {
            completed_before,
            dead_before,
        }
    }
}

// Implements poll pending.
#[instrument(level = "info", skip_all)]
async fn poll_pending(conn: &mut RdbConn) -> BaseRest<Vec<LocalMessageRow>> {
    //
    let processing_message =
        diesel::alias!(t_local_message as processing_message);

    let processing_topic = processing_message
        .filter(
            processing_message
                .field(t_local_message::f_status)
                .eq(LocalMessageStatus::Processing.as_str()),
        )
        .filter(
            processing_message
                .field(t_local_message::f_topic)
                .eq(t_local_message::f_topic),
        );

    let local_message_rows = t_local_message::table
        .filter(
            t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
        )
        .filter(t_local_message::f_visible_at.le(OffsetDateTime::now_utc()))
        .filter(diesel::dsl::not(diesel::dsl::exists(processing_topic)))
        .distinct_on(t_local_message::f_topic)
        .order_by((
            t_local_message::f_topic.asc(),
            t_local_message::f_created_at.asc(),
            t_local_message::f_id.asc(),
        ))
        .select((
            t_local_message::f_id,
            t_local_message::f_topic,
            t_local_message::f_payload,
            t_local_message::f_retried_count,
            t_local_message::f_lease,
        ))
        .load::<LocalMessageRow>(conn)
        .await
        .map_err(diesel)?;

    accept(local_message_rows)
}

// Implements claim pending.
#[instrument(level = "info", skip_all)]
async fn claim_pending(
    conn: &mut RdbConn,
    id: &str,
    lease: i64,
) -> BaseRest<bool> {
    //
    let updated = diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq(id))
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Pending.as_str()),
            )
            .filter(t_local_message::f_lease.eq(lease)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Processing.as_str()),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(updated > 0)
}

// Implements complete message.
#[instrument(level = "info", skip_all)]
async fn complete_message(
    conn: &mut RdbConn,
    id: &str,
    lease: i64,
) -> BaseRest<()> {
    //
    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq(id))
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_lease.eq(lease)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Completed.as_str()),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Implements fail message.
#[instrument(level = "info", skip_all)]
async fn fail_message(
    conn: &mut RdbConn,
    id: &str,
    lease: i64,
    error: &str,
) -> BaseRest<()> {
    //
    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq(id))
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_lease.eq(lease)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
        t_local_message::f_last_error.eq(Some(error)),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Implements retry message.
#[instrument(level = "info", skip_all)]
async fn retry_message(
    conn: &mut RdbConn,
    id: &str,
    lease: i64,
    error: &str,
    visible_at: &OffsetDateTime,
    retry_delta: i64,
) -> BaseRest<()> {
    //
    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq(id))
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_lease.eq(lease)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
        t_local_message::f_last_error.eq(Some(error)),
        t_local_message::f_retried_count
            .eq(t_local_message::f_retried_count + retry_delta),
        t_local_message::f_visible_at.eq(*visible_at),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Implements reset stuck.
#[instrument(level = "info", skip_all)]
async fn reset_stuck(
    conn: &mut RdbConn,
    before: &OffsetDateTime,
) -> BaseRest<()> {
    //
    diesel::update(
        t_local_message::table
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_updated_at.le(*before))
            .filter(t_local_message::f_lease.ge(3)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
        t_local_message::f_last_error.eq(Some("processing timeout exceeded")),
        t_local_message::f_lease.eq(t_local_message::f_lease + 1),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::update(
        t_local_message::table
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_updated_at.le(*before))
            .filter(t_local_message::f_lease.lt(3)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
        t_local_message::f_lease.eq(t_local_message::f_lease + 1),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Implements purge completed.
#[instrument(level = "info", skip_all)]
async fn purge_completed(
    conn: &mut RdbConn,
    completed_before: &OffsetDateTime,
    dead_before: &OffsetDateTime,
) -> BaseRest<usize> {
    //
    let (expired_completed, expired_dead) = (
        t_local_message::f_status
            .eq(LocalMessageStatus::Completed.as_str())
            .and(t_local_message::f_updated_at.lt(*completed_before)),
        t_local_message::f_status
            .eq(LocalMessageStatus::Dead.as_str())
            .and(t_local_message::f_updated_at.lt(*dead_before)),
    );

    let purged_count = diesel::delete(
        t_local_message::table.filter(expired_completed.or(expired_dead)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(purged_count)
}

/// Queue repository used by the prom background actor.
///
/// Provides polling, claiming, completion, failure, retry, and recovery
/// operations for records in `t_local_message`.
///
/// [`RdbPromActor`]: super::actor::base::RdbPromActor
#[derive(Default)]
pub struct RdbPromRepo;

impl RdbPromRepo {
    /// Builds the local-message queue repository.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<L> Step<PollPending, RdbContext<L>> for RdbPromRepo
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
        _oper: &PollPending,
    ) -> BaseRest<Vec<LocalMessageRow>> {
        poll_pending(context.conn()).await
    }
}

impl<'a, L> Step<ClaimPending<'a>, RdbContext<L>> for RdbPromRepo
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
        oper: &ClaimPending<'a>,
    ) -> BaseRest<bool> {
        claim_pending(context.conn(), oper.id, oper.lease).await
    }
}

impl<'a, L> Step<CompleteMessage<'a>, RdbContext<L>> for RdbPromRepo
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
        oper: &CompleteMessage<'a>,
    ) -> BaseRest<()> {
        complete_message(context.conn(), oper.id, oper.lease).await
    }
}

impl<'a, L> Step<FailMessage<'a>, RdbContext<L>> for RdbPromRepo
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
        oper: &FailMessage<'a>,
    ) -> BaseRest<()> {
        fail_message(context.conn(), oper.id, oper.lease, oper.error).await
    }
}

impl<'a, L> Step<RetryMessage<'a>, RdbContext<L>> for RdbPromRepo
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
        oper: &RetryMessage<'a>,
    ) -> BaseRest<()> {
        //
        retry_message(
            context.conn(),
            oper.id,
            oper.lease,
            oper.error,
            oper.visible_at,
            oper.retry_delta,
        )
        .await
    }
}

impl<'a, L> Step<ResetStuck<'a>, RdbContext<L>> for RdbPromRepo
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
        oper: &ResetStuck<'a>,
    ) -> BaseRest<()> {
        reset_stuck(context.conn(), oper.before).await
    }
}

impl<'a, L> Step<PurgeCompleted<'a>, RdbContext<L>> for RdbPromRepo
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
        oper: &PurgeCompleted<'a>,
    ) -> BaseRest<usize> {
        //
        purge_completed(context.conn(), oper.completed_before, oper.dead_before)
            .await
    }
}
