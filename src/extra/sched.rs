//! Fixed production composition for periodic background jobs.

// Hierarchy mark-and-sweep job.
mod subtree_delete;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use poprako_obj_dept::ObjDept;
use poprako_rdb_core::RdbCore;

use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar,
};
use crate::shared::RdbContext;

// Fixed worker count for the relational hierarchy sweep.
const SUBTREE_DELETE_SWEEP_WORKERS: usize = 2;

/// Owns the lifecycle of the fixed production background-job composition.
pub struct Sched {
    //
    /// Shared cancellation signal for every explicitly composed job.
    token: CancellationToken,

    /// Completion receivers used during graceful shutdown.
    done_recvs: Vec<watch::Receiver<bool>>,
}

impl Sched {
    /// Starts the fixed pair of hierarchy sweep workers.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "workers must own cloned production ports for 'static tasks"
    )]
    pub fn new<O>(core: RdbCore, obj_dept: O) -> Self
    where
        O: ObjDept<ChapterArtwork, RdbContext<ReptRead>>
            + ObjDept<PageImage, RdbContext<ReptRead>>
            + ObjDept<ComicCover, RdbContext<ReptRead>>
            + ObjDept<TeamAvatar, RdbContext<ReptRead>>
            + Clone
            + Send
            + Sync
            + 'static,
    {
        let token = CancellationToken::new();

        let done_recvs = (0..SUBTREE_DELETE_SWEEP_WORKERS)
            .map(|_| {
                //
                subtree_delete::spawn(
                    core.clone(),
                    obj_dept.clone(),
                    token.clone(),
                )
            })
            .collect();

        Self { token, done_recvs }
    }

    /// Stops acquiring cleanup work and waits for in-flight transactions.
    pub async fn close(&self) {
        //
        self.token.cancel();

        for done_recv in &self.done_recvs {
            //
            let mut done_recv = done_recv.clone();

            if let Err(error) = done_recv.wait_for(|done| *done).await {
                //
                tracing::error!(
                    err = %error,
                    operation = "close_subtree_sweep_worker",
                    "scheduler worker ended without completion",
                );
            }
        }
    }
}

impl Drop for Sched {
    // Cancel workers when the scheduler is dropped.
    fn drop(&mut self) {
        self.token.cancel();
    }
}
