//! Single object-task actor lifecycle.

#[cfg(feature = "rdb_impl")]
/// RDB-backed handler support.
pub mod rdb_impl;

#[cfg(test)]
mod tests;

use std::future::Future;

use tokio_util::sync::CancellationToken;

use crate::model::task::{ObjDeptPromTask, ObjTaskAction, validate_task};
use crate::prom::ObjDeptProm;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Delay between idle polls.
pub const POLL_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(30);

/// Maximum duration of one claimed attempt.
pub const ATTEMPT_TIMEOUT: std::time::Duration =
    std::time::Duration::from_mins(1);

// Maps one adapter failure to its mechanical durable-task action.
fn action_from_err(err: ObjDeptError) -> ObjTaskAction {
    //
    match err {
        //
        ObjDeptError::Retryable { message }
        | ObjDeptError::Unavailable { message }
        | ObjDeptError::Conflict { message } => {
            ObjTaskAction::Retry { message }
        }

        ObjDeptError::Invalid { message }
        | ObjDeptError::Unrecoverable { message } => {
            ObjTaskAction::Operator { message }
        }
    }
}

// Persists one actor decision through the durable-task adapter.
async fn persist_action<P>(
    prom: &P,
    task: &ObjDeptPromTask,
    action: &ObjTaskAction,
) -> ObjDeptRest<usize>
where
    P: ObjDeptProm,
{
    //
    match action {
        //
        ObjTaskAction::Complete => prom.complete_task(task).await,

        ObjTaskAction::Retry { message } => {
            prom.retry_task(task, message).await
        }

        ObjTaskAction::Operator { message } => {
            prom.mark_task_operator(task, message).await
        }
    }
}

// Waits until another poll is due or cancellation is requested.
async fn wait_poll(token: &CancellationToken) -> bool {
    //
    tokio::select! {
        () = token.cancelled() => false,
        () = tokio::time::sleep(POLL_INTERVAL) => true,
    }
}

// Runs typed dispatch and persists its fenced task mutation.
async fn run_attempt<P, H, F>(
    prom: &P,
    handler: &H,
    task: &ObjDeptPromTask,
) -> ObjDeptRest<usize>
where
    P: ObjDeptProm,
    H: Fn(ObjDeptPromTask) -> F,
    F: Future<Output = ObjDeptRest<ObjTaskAction>>,
{
    //
    let action = match validate_task(task) {
        //
        Ok(()) => match handler(task.clone()).await {
            //
            Ok(action) => action,

            Err(err) => action_from_err(err),
        },

        Err(err) => action_from_err(err),
    };

    if let ObjTaskAction::Operator { message } = &action {
        //
        tracing::error!(
            task_id = task.id,
            err_message = %message,
            "ObjDept task requires operator repair",
        );
    }

    persist_action(prom, task, &action).await
}

// Keeps the actor helper call graph in the repository's required order.
// Claims immediately, drains visible work, and waits only when no task is available.
async fn run_claim_loop<P, H, F>(prom: P, handler: H, token: CancellationToken)
where
    P: ObjDeptProm,
    H: Fn(ObjDeptPromTask) -> F,
    F: Future<Output = ObjDeptRest<ObjTaskAction>>,
{
    loop {
        //
        let claimed = tokio::select! {
            () = token.cancelled() => break,
            claimed = tokio::time::timeout(
                ATTEMPT_TIMEOUT,
                prom.claim_task(),
            ) => claimed,
        };

        let task = match claimed {
            //
            Ok(Ok(Some(task))) => task,

            Ok(Ok(None)) => {
                //
                if !wait_poll(&token).await {
                    break;
                }

                continue;
            }

            Ok(Err(err)) => {
                //
                tracing::error!(
                    operation = "claim_obj_task",
                    sdk_err = ?err,
                    "ObjDept task claim failed",
                );

                if !wait_poll(&token).await {
                    break;
                }

                continue;
            }

            Err(_) => {
                //
                tracing::error!(
                    operation = "claim_obj_task",
                    "ObjDept task claim timed out",
                );

                continue;
            }
        };

        let attempt = run_attempt(&prom, &handler, &task);

        let changed = tokio::select! {
            () = token.cancelled() => break,
            changed = tokio::time::timeout(ATTEMPT_TIMEOUT, attempt) => changed,
        };

        match changed {
            //
            Ok(Ok(1)) => {}

            Ok(Ok(0)) => tracing::warn!(
                task_id = task.id,
                lease = task.lease,
                "ObjDept task lost its lease",
            ),

            Ok(Ok(updated_count)) => tracing::error!(
                task_id = task.id,
                updated_count,
                "ObjDept task changed multiple rows",
            ),

            Ok(Err(err)) => tracing::error!(
                task_id = task.id,
                sdk_err = ?err,
                "ObjDept task update failed",
            ),

            Err(_) => tracing::error!(
                task_id = task.id,
                "ObjDept task attempt timed out",
            ),
        }
    }
}

// Runs maintenance immediately and then at an independent fixed cadence.
async fn run_maintenance_loop<P>(prom: P, token: CancellationToken)
where
    P: ObjDeptProm,
{
    loop {
        //
        let reset = tokio::select! {
            () = token.cancelled() => break,
            reset = tokio::time::timeout(
                ATTEMPT_TIMEOUT,
                prom.reset_tasks(),
            ) => reset,
        };

        match reset {
            //
            Ok(Ok(_)) => {}

            Ok(Err(err)) => tracing::error!(
                operation = "reset_obj_tasks",
                sdk_err = ?err,
                "ObjDept task reset failed",
            ),

            Err(_) => tracing::error!(
                operation = "reset_obj_tasks",
                "ObjDept task reset timed out",
            ),
        }

        if !wait_poll(&token).await {
            break;
        }
    }
}

// Runs claim and maintenance under one cancellation supervisor.
async fn run_actor<P, H, F>(prom: P, handler: H, token: CancellationToken)
where
    P: ObjDeptProm + Clone,
    H: Fn(ObjDeptPromTask) -> F,
    F: Future<Output = ObjDeptRest<ObjTaskAction>>,
{
    let claim_loop = run_claim_loop(prom.clone(), handler, token.clone());

    let maintenance_loop = run_maintenance_loop(prom, token);

    tokio::join!(claim_loop, maintenance_loop);
}

/// Owns cancellation and completion of one background supervisor.
pub struct ObjDeptActorDesc {
    //
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
    /// Task whose completion includes its worker shutdown.
    task: tokio::task::JoinHandle<()>,
}

impl ObjDeptActorDesc {
    /// Requests cancellation without waiting for completion.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Waits for completion and reports a supervisor panic or cancellation.
    ///
    /// # Errors
    /// Returns the supervisor task's join error.
    pub async fn join(mut self) -> Result<(), tokio::task::JoinError> {
        (&mut self.task).await
    }
}

impl Drop for ObjDeptActorDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Object consumer with explicitly injected task storage and dispatch.
pub struct ObjDeptActor<P, H> {
    //
    /// Durable task adapter.
    prom: P,
    /// Object task dispatcher.
    handler: H,
}

impl<P, H> ObjDeptActor<P, H> {
    /// Constructs an object consumer without starting background work.
    pub const fn new(prom: P, handler: H) -> Self {
        Self { prom, handler }
    }

    /// Starts the claim and maintenance loops under one supervisor.
    #[must_use]
    pub fn run_detach<F>(self) -> ObjDeptActorDesc
    where
        P: ObjDeptProm + Clone + Send + Sync + 'static,
        H: Fn(ObjDeptPromTask) -> F + Send + Sync + 'static,
        F: Future<Output = ObjDeptRest<ObjTaskAction>> + Send + 'static,
    {
        let token = CancellationToken::new();

        let task =
            tokio::spawn(run_actor(self.prom, self.handler, token.clone()));

        ObjDeptActorDesc { token, task }
    }
}
