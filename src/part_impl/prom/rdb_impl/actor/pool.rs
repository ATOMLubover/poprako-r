//! Fixed-size worker pool for persisted prom tasks.

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `tests` module.
mod tests;

use std::sync::Arc;
use std::time::Duration as StdDuration;

use poprako_orchestra::{Nucl as _, OperStep as _};
use time::{Duration, OffsetDateTime};
use tokio::sync::{Notify, mpsc};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;

use crate::part::effect::Develop;
use crate::part::obj_dept::PageImage;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::actor::base::RdbPromActor;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageRow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, CompleteMessage, FailMessage, PollPending, PurgeCompleted,
    ResetStuck, RetryMessage,
};
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

// Constant definition for `WORKER_COUNT`.
const WORKER_COUNT: usize = 4;

// Constant definition for `POLL_INTERVAL`.
const POLL_INTERVAL: StdDuration = StdDuration::from_mins(1);

// Constant definition for `STUCK_RESET_INTERVAL`.
const STUCK_RESET_INTERVAL: Duration = Duration::minutes(1);

// Constant definition for `RETRY_DELAY`.
const RETRY_DELAY: Duration = Duration::minutes(5);

// Constant definition for `PROCESSING_TIMEOUT`.
const PROCESSING_TIMEOUT: Duration = Duration::minutes(15);

// Constant definition for `COMPLETED_RETENTION`.
const COMPLETED_RETENTION: Duration = Duration::days(7);

// Constant definition for `DEAD_RETENTION`.
const DEAD_RETENTION: Duration = Duration::days(30);

// Constant definition for `COMPLETED_PURGE_INTERVAL`.
const COMPLETED_PURGE_INTERVAL: Duration = Duration::hours(1);

// Constant definition for `FNV_OFFSET_BASIS`.
const FNV_OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;

// Constant definition for `FNV_PRIME`.
const FNV_PRIME: u64 = 1_099_511_628_211;

// Internal type alias for `WorkerSend`.
type WorkerSend = mpsc::UnboundedSender<LocalMessageRow>;

/// Enforces the retry limit for a task flow.
///
/// When the task has been retried 3 or more times, transitions from
/// [`TaskFlow::Retry`] to [`TaskFlow::Dead`] so the message is not
/// requeued indefinitely.
pub fn enforce_retry_limit(
    task_flow: TaskFlow,
    retried_count: i64,
) -> TaskFlow {
    //
    match (task_flow, retried_count >= 3) {
        //
        (TaskFlow::Retry { err_message: error }, true) => {
            TaskFlow::Dead { err_message: error }
        }

        (task_flow, _) => task_flow,
    }
}

impl<R, V, D> RdbPromActor<RdbNucl, R, V, D>
where
    R: AssignmentInvitationRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync
        + 'static,
    V: ObjDeptView<PageImage, RdbContext> + Send + Sync + 'static,
    D: Develop + Send + Sync + 'static,
{
    /// Runs the polling supervisor and drains in-flight worker tasks on shutdown.
    #[instrument(level = "info", skip_all)]
    pub async fn run(self) {
        //
        let (actor, completed) = (Arc::new(self), Arc::new(Notify::new()));

        let (worker_sends, worker_handles) = actor.spawn_workers(&completed);

        actor
            .run_supervisor(&worker_sends, completed.as_ref())
            .await;

        drop(worker_sends);

        for worker_handle in worker_handles {
            //
            if let Err(error) = worker_handle.await {
                //
                tracing::error!(
                    err = ?error,
                    "[RdbPromActor::run] worker task failed",
                );
            }
        }
    }

    // Internal implementation of `spawn_workers`.
    fn spawn_workers(
        self: &Arc<Self>,
        completed: &Arc<Notify>,
    ) -> (Vec<WorkerSend>, Vec<JoinHandle<()>>) {
        //
        let (mut worker_sends, mut worker_handles) = (
            Vec::with_capacity(WORKER_COUNT),
            Vec::with_capacity(WORKER_COUNT),
        );

        for worker_index in 0..WORKER_COUNT {
            //
            let (worker_send, mut worker_recv) = mpsc::unbounded_channel();

            let (actor, completed) = (self.clone(), completed.clone());

            let worker_handle = tokio::spawn(async move {
                //
                while let Some(row) = worker_recv.recv().await {
                    //
                    actor.process_row(&row).await;

                    completed.notify_one();
                }

                tracing::debug!(
                    worker_index,
                    "[RdbPromActor::worker] worker stopped",
                );
            });

            worker_sends.push(worker_send);

            worker_handles.push(worker_handle);
        }

        (worker_sends, worker_handles)
    }

    // Internal implementation of `run_supervisor`.
    async fn run_supervisor(
        &self,
        worker_sends: &[WorkerSend],
        completed: &Notify,
    ) {
        //
        let (mut next_stuck_reset_at, mut next_completed_purge_at) =
            (OffsetDateTime::now_utc(), OffsetDateTime::now_utc());

        loop {
            //
            if self.token().is_cancelled() {
                break;
            }

            let now = OffsetDateTime::now_utc();

            if now >= next_stuck_reset_at {
                //
                self.log_reset_stuck().await;

                next_stuck_reset_at = schedule_at(now, STUCK_RESET_INTERVAL);
            }

            if now >= next_completed_purge_at {
                //
                self.log_purge_completed().await;

                next_completed_purge_at =
                    schedule_at(now, COMPLETED_PURGE_INTERVAL);
            }

            let dispatched = match self.poll().await {
                //
                Ok(rows) => {
                    //
                    match self.dispatch_rows(worker_sends, rows).await {
                        //
                        Ok(dispatched) => dispatched,

                        Err(error) => {
                            //
                            tracing::error!(
                                err = ?error,
                                "[RdbPromActor::run] worker index calculation failed",
                            );

                            false
                        }
                    }
                }

                Err(error) => {
                    //
                    tracing::error!(
                        err = ?error,
                        "[RdbPromActor::run] poll failed",
                    );

                    false
                }
            };

            if dispatched {
                continue;
            }

            tokio::select! {
                //
                biased;

                () = self.token().cancelled() => break,

                () = completed.notified() => {}

                () = sleep(POLL_INTERVAL) => {}
            }
        }
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `process_row`.
    async fn process_row(&self, row: &LocalMessageRow) {
        //
        let task_flow =
            self.dispatch_payload(&row.f_topic, &row.f_payload).await;

        let task_flow = enforce_retry_limit(task_flow, row.f_retried_count);

        match task_flow {
            //
            TaskFlow::Complete => {
                //
                if let Err(error) = self.complete(&row.f_id, row.f_lease).await
                {
                    tracing::error!(
                        id = %row.f_id,
                        err = ?error,
                        "[RdbPromActor::process_row] complete failed",
                    );
                }
            }

            TaskFlow::Retry { err_message: error } => {
                self.log_reschedule(row, &error, 1).await;
            }

            TaskFlow::Wait { err_message: error } => {
                self.log_reschedule(row, &error, 0).await;
            }

            TaskFlow::Dead { err_message: error } => {
                //
                tracing::error!(
                    id = %row.f_id,
                    topic = %row.f_topic,
                    err = %error,
                    "[RdbPromActor::process_row] task failed",
                );

                if let Err(mark_error) =
                    self.fail(&row.f_id, row.f_lease, &error).await
                {
                    tracing::error!(
                        id = %row.f_id,
                        original_err = %error,
                        err = ?mark_error,
                        "[RdbPromActor::process_row] fail mark failed",
                    );
                }
            }
        }
    }

    // Internal implementation of `log_reset_stuck`.
    async fn log_reset_stuck(&self) {
        //
        if let Err(error) = self.reset_stuck().await {
            //
            tracing::error!(
                err = ?error,
                "[RdbPromActor::run] reset stuck failed",
            );
        }
    }

    // Internal implementation of `log_purge_completed`.
    async fn log_purge_completed(&self) {
        //
        match self.purge_completed().await {
            //
            Ok(purged_count) => {
                //
                if purged_count > 0 {
                    //
                    tracing::info!(
                        purged_count,
                        "[RdbPromActor::run] purged expired completed messages",
                    );
                }
            }

            Err(error) => {
                //
                tracing::error!(
                    err = ?error,
                    "[RdbPromActor::run] purge completed failed",
                );
            }
        }
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `poll`.
    async fn poll(&self) -> BaseRest<Vec<LocalMessageRow>> {
        //
        let rows = self
            .prom_nucl()
            .coord(async |context| {
                PollPending.step_on(self.prom_repo(), context).await
            })
            .await?;

        Ok(rows)
    }

    // Internal implementation of `dispatch_rows`.
    async fn dispatch_rows(
        &self,
        worker_sends: &[WorkerSend],
        rows: Vec<LocalMessageRow>,
    ) -> BaseRest<bool> {
        //
        let mut dispatched = false;

        for row in rows {
            //
            let worker_index = topic_worker_index(&row.f_topic)?;

            let Some(worker_send) = worker_sends.get(worker_index) else {
                //
                tracing::error!(
                    id = %row.f_id,
                    worker_index,
                    worker_count = worker_sends.len(),
                    "internal invariant violated: prom worker is missing",
                );

                continue;
            };

            let claimed = match self.claim(&row.f_id, row.f_lease).await {
                //
                Ok(claimed) => claimed,

                Err(error) => {
                    //
                    tracing::error!(
                        id = %row.f_id,
                        err = ?error,
                        "[RdbPromActor::dispatch_rows] claim failed",
                    );

                    continue;
                }
            };

            if !claimed {
                continue;
            }

            match worker_send.send(row) {
                //
                Ok(()) => dispatched = true,

                Err(error) => {
                    //
                    tracing::error!(
                        id = %error.0.f_id,
                        worker_index,
                        "[RdbPromActor::dispatch_rows] worker channel closed",
                    );
                }
            }
        }

        Ok(dispatched)
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `complete`.
    async fn complete(&self, id: &str, lease: i64) -> BaseRest<()> {
        //
        self.prom_nucl()
            .coord(async |context| {
                //
                CompleteMessage::new(id, lease)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(())
    }

    // Logs a failed attempt to return one task to pending.
    async fn log_reschedule(
        &self,
        row: &LocalMessageRow,
        message: &str,
        retry_delta: i64,
    ) {
        //
        let rest = async {
            //
            let visible_at = OffsetDateTime::now_utc()
                .checked_add(RETRY_DELAY)
                .ok_or_else(|| BaseError::Unrecoverable {
                    message:
                        "prom retry timestamp is outside the supported range"
                            .into(),
                })?;

            self.prom_nucl()
                .coord(async |context| {
                    //
                    RetryMessage::new(
                        &row.f_id,
                        row.f_lease,
                        message,
                        &visible_at,
                        retry_delta,
                    )
                    .step_on(self.prom_repo(), context)
                    .await
                })
                .await?;

            Ok::<(), BaseError>(())
        }
        .await;

        if let Err(mark_error) = rest {
            //
            tracing::error!(
                id = %row.f_id,
                original_err = %message,
                err = ?mark_error,
                "[RdbPromActor::process_row] reschedule failed",
            );
        }
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `fail`.
    async fn fail(&self, id: &str, lease: i64, message: &str) -> BaseRest<()> {
        //
        self.prom_nucl()
            .coord(async |context| {
                //
                FailMessage::new(id, lease, message)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(())
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `reset_stuck`.
    async fn reset_stuck(&self) -> BaseRest<()> {
        //
        let before = OffsetDateTime::now_utc()
            .checked_sub(PROCESSING_TIMEOUT)
            .ok_or_else(|| BaseError::Unrecoverable {
                message:
                    "prom processing cutoff is outside the supported range"
                        .into(),
            })?;

        self.prom_nucl()
            .coord(async |context| {
                //
                ResetStuck::new(&before)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(())
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `purge_completed`.
    async fn purge_completed(&self) -> BaseRest<usize> {
        //
        let completed_before = OffsetDateTime::now_utc()
            .checked_sub(COMPLETED_RETENTION)
            .ok_or_else(|| BaseError::Unrecoverable {
                message:
                    "prom completion cutoff is outside the supported range"
                        .into(),
            })?;

        let dead_before = OffsetDateTime::now_utc()
            .checked_sub(DEAD_RETENTION)
            .ok_or_else(|| BaseError::Unrecoverable {
                message:
                    "prom dead-message cutoff is outside the supported range"
                        .into(),
            })?;

        let purged_count = self
            .prom_nucl()
            .coord(async |context| {
                //
                PurgeCompleted::new(&completed_before, &dead_before)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(purged_count)
    }

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `claim`.
    async fn claim(&self, id: &str, lease: i64) -> BaseRest<bool> {
        //
        let claimed = self
            .prom_nucl()
            .coord(async |context| {
                //
                ClaimPending::new(id, lease)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(claimed)
    }
}

// Internal implementation of `topic_worker_index`.
fn topic_worker_index(topic: &str) -> BaseRest<usize> {
    //
    let hash = topic.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    });

    let worker_count =
        u64::try_from(WORKER_COUNT).map_err(|_| BaseError::Unrecoverable {
            message: "worker count exceeds u64 range".into(),
        })?;

    usize::try_from(hash % worker_count).map_err(|_| BaseError::Unrecoverable {
        message: "worker index exceeds usize range".into(),
    })
}

// Computes the next maintenance deadline without panicking at time bounds.
const fn schedule_at(
    now: OffsetDateTime,
    interval: Duration,
) -> OffsetDateTime {
    now.saturating_add(interval)
}
