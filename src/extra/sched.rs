//! Explicitly composed periodic background jobs.

// Hierarchy mark-and-sweep job.
mod subtree_delete;

use tokio_util::sync::CancellationToken;

use poprako_obj_dept::ObjDept;

use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::shared::RdbContext;

// Fixed worker count for the relational hierarchy sweep.
const SUBTREE_DELETE_SWEEP_WORKERS: usize = 2;

/// Owns cancellation and completion of one background supervisor.
pub struct SchedDesc {
    //
    //
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
    /// Task whose completion includes its worker shutdown.
    task: tokio::task::JoinHandle<()>,
}

impl SchedDesc {
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

impl Drop for SchedDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Periodic jobs with explicitly injected business ports.
pub struct Sched<N, R, O> {
    //
    //
    /// Transaction coordinator.
    nucl: N,
    /// Shared business repository.
    repo: R,
    /// Object lifecycle port.
    obj_dept: O,
}

impl<N, R, O> Sched<N, R, O> {
    /// Constructs the scheduler without starting workers.
    pub const fn new(nucl: N, repo: R, obj_dept: O) -> Self {
        //
        Self {
            //
            nucl,
            repo,
            obj_dept,
        }
    }
}

impl<R, O> Sched<RdbNucl, R, O> {
    /// Starts the fixed worker group and returns its runtime owner.
    #[must_use]
    pub fn run_detach(self) -> SchedDesc
    where
        R: SubtreeRepo<RdbContext> + Clone + Send + Sync + 'static,
        O: ObjDept<ChapterArtwork, RdbContext>
            + ObjDept<PageImage, RdbContext>
            + ObjDept<ComicCover, RdbContext>
            + ObjDept<TeamAvatar, RdbContext>
            + Clone
            + Send
            + Sync
            + 'static,
    {
        let token = CancellationToken::new();

        let worker_token = token.clone();

        let task = tokio::spawn(async move {
            //
            let workers = (0..SUBTREE_DELETE_SWEEP_WORKERS)
                .map(|_| {
                    //
                    subtree_delete::spawn(
                        self.nucl.clone(),
                        self.repo.clone(),
                        self.obj_dept.clone(),
                        worker_token.clone(),
                    )
                })
                .collect::<Vec<_>>();

            for worker in workers {
                //
                if let Err(error) = worker.await {
                    tracing::error!(err = ?error, operation = "join_subtree_worker", "scheduler worker failed");
                }
            }
        });

        SchedDesc { token, task }
    }
}
