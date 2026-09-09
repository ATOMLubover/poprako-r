//! Shared types and task dispatch logic for the prom actor submodules.
//!
//! Defined here so that both the parent [`actor`] and its child modules
//! (notably [`pool`]) can import without creating an upward ancestor
//! dependency.
//!
//! [`actor`]: crate::part_impl::prom::rdb_impl::actor
//! [`pool`]: crate::part_impl::prom::rdb_impl::actor::pool

use poprako_orchestra::Nucl;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;

use crate::part::effect::Develop;
use crate::part::obj_dept::PageImage;
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::dispatch;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::result::BaseError;
use crate::shared::RdbContext;

/// Owns cancellation and completion of one background supervisor.
pub struct RdbPromActorDesc {
    //
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
    /// Task whose completion includes its worker shutdown.
    task: tokio::task::JoinHandle<()>,
}

impl RdbPromActorDesc {
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

impl Drop for RdbPromActorDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Persisted-task consumer with explicitly injected queue and business ports.
pub struct RdbPromActor<N, R, V, D> {
    //
    /// Queue transaction coordinator.
    prom_nucl: RdbNucl,
    /// Queue lifecycle repository.
    prom_repo: RdbPromRepo,

    /// Business transaction coordinator.
    nucl: N,
    /// Business repository.
    repo: R,
    /// Object query port.
    obj_dept_view: V,
    /// Event producer.
    develop: D,

    /// Supervisor cancellation signal.
    token: CancellationToken,
}

impl<N, R, V, D> RdbPromActor<N, R, V, D> {
    /// Constructs a consumer without starting any background work.
    pub fn new(
        (prom_nucl, prom_repo): (RdbNucl, RdbPromRepo),
        (nucl, repo, obj_dept_view, develop): (N, R, V, D),
    ) -> Self {
        //
        Self {
            prom_nucl,
            prom_repo,
            nucl,
            repo,
            obj_dept_view,
            develop,
            token: CancellationToken::new(),
        }
    }

    /// Returns the injected queue transaction coordinator.
    pub const fn prom_nucl(&self) -> &RdbNucl {
        &self.prom_nucl
    }

    /// Returns the injected queue repository.
    pub const fn prom_repo(&self) -> &RdbPromRepo {
        &self.prom_repo
    }

    /// Returns the supervisor cancellation signal.
    pub const fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl<N, R, V, D> RdbPromActor<N, R, V, D>
where
    N: Nucl<Error = BaseError> + Send + Sync,
    N::Context: Send,
    R: AssignmentInvitationRepo<N::Context>
        + ChapterRepo<N::Context>
        + ChapterWorkflowRecordRepo<N::Context>
        + MemberInvitationRepo<N::Context>
        + PageRepo<N::Context>
        + Send
        + Sync,
    V: ObjDeptView<PageImage, N::Context> + Send + Sync,
    D: Develop + Send + Sync,
{
    /// Decodes and dispatches one persisted prom payload.
    #[instrument(level = "info", skip_all)]
    pub async fn dispatch_payload(
        &self,
        topic: &str,
        payload: &serde_json::Value,
    ) -> TaskFlow {
        //
        let task = match serde_json::from_value::<TaskPayload>(payload.clone())
        {
            //
            Ok(task) => task,

            Err(error) => {
                //
                tracing::error!(
                    operation = "deserialize_prom_payload",
                    sdk_err = ?error,
                    "JSON SDK deserialization error",
                );

                return TaskFlow::Dead {
                    err_message: format!(
                        "failed to deserialize prom payload: {}",
                        error,
                    ),
                };
            }
        };

        if task.topic() != topic {
            //
            return TaskFlow::Dead {
                err_message: format!(
                    "prom topic {} does not match payload topic {}",
                    topic,
                    task.topic()
                ),
            };
        }

        dispatch::dispatch::<N::Context, _, _, _, _>(
            (&self.nucl, &self.repo, &self.obj_dept_view, &self.develop),
            task,
        )
        .await
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
        + Sync,
    V: ObjDeptView<PageImage, RdbContext> + Send + Sync,
    D: Develop + Send + Sync,
{
    /// Starts the consumer and transfers shutdown ownership to its descriptor.
    #[must_use]
    pub fn run_detach(self) -> RdbPromActorDesc
    where
        R: 'static,
        V: 'static,
        D: 'static,
    {
        let (token, task) = (self.token.clone(), tokio::spawn(self.run()));

        RdbPromActorDesc { token, task }
    }
}
