//! Periodic relational hierarchy sweeping.

#[cfg(test)]
mod tests;

use std::future::Future;
use std::time::Duration;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use poprako_obj_dept::ObjDept;
use poprako_rdb_core::RdbCore;

use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::result::BaseRest;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::subtree_delete::SubtreeSweepLevel;

// Delay between empty or failed sweep attempts.
const RETRY_DELAY: Duration = Duration::from_secs(5);

// Hierarchy levels are polled from leaves to roots.
const SWEEP_LEVELS: [SubtreeSweepLevel; 4] = [
    SubtreeSweepLevel::Chapter,
    SubtreeSweepLevel::Comic,
    SubtreeSweepLevel::Workset,
    SubtreeSweepLevel::Team,
];

/// Wait for cancellation or the next retry interval.
pub async fn wait(token: &CancellationToken) -> bool {
    //
    tokio::select! {
        () = token.cancelled() => true,
        () = tokio::time::sleep(RETRY_DELAY) => false,
    }
}

/// Spawns one hierarchy sweep worker.
pub fn spawn<O>(
    core: RdbCore,
    obj_dept: O,
    token: CancellationToken,
) -> watch::Receiver<bool>
where
    O: ObjDept<ChapterArtwork, RdbContext<ReptRead>>
        + ObjDept<PageImage, RdbContext<ReptRead>>
        + ObjDept<ComicCover, RdbContext<ReptRead>>
        + ObjDept<TeamAvatar, RdbContext<ReptRead>>
        + Send
        + Sync
        + 'static,
{
    let (done_send, done_recv) = watch::channel(false);

    tokio::spawn(async move {
        //
        let nucl = RdbNucl::<ReptRead>::new(core.clone());

        let repo = HybRepo::new(core);

        run(
            &token,
            |level| {
                usecase::subtree_delete::sweep((&nucl, &repo, &obj_dept), level)
            },
            || wait(&token),
        )
        .await;

        done_send.send_replace(true);
    });

    done_recv
}

// Runs hierarchy sweep rounds until cancellation or a cancelled retry wait.
async fn run<F, Fut, W, WaitFut>(
    token: &CancellationToken,
    mut sweep_level: F,
    mut wait_retry: W,
) where
    F: FnMut(SubtreeSweepLevel) -> Fut,
    Fut: Future<Output = BaseRest<bool>>,
    W: FnMut() -> WaitFut,
    WaitFut: Future<Output = bool>,
{
    loop {
        //
        if token.is_cancelled() {
            break;
        }

        let swept = sweep(token, &mut sweep_level).await;

        if !swept && wait_retry().await {
            break;
        }
    }
}

// Poll one hierarchy sweep round in dependency order.
async fn sweep<F, Fut>(token: &CancellationToken, mut sweep_level: F) -> bool
where
    F: FnMut(SubtreeSweepLevel) -> Fut,
    Fut: Future<Output = BaseRest<bool>>,
{
    //
    for sweep_level_value in SWEEP_LEVELS {
        //
        if token.is_cancelled() {
            return false;
        }

        match sweep_level(sweep_level_value).await {
            //
            Ok(true) => return true,

            Ok(false) => {}

            Err(error) => {
                //
                tracing::error!(
                    err = ?error,
                    sweep_level = ?sweep_level_value,
                    operation = "sweep_subtree_delete",
                    "hierarchy sweep level failed and polling will continue",
                );
            }
        }
    }

    false
}
