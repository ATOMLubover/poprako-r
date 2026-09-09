//! Periodic relational hierarchy sweeping.

#[cfg(test)]
mod tests;

use std::future::Future;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use poprako_obj_dept::ObjDept;

use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
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
pub fn spawn<R, O>(
    nucl: RdbNucl,
    repo: R,
    obj_dept: O,
    token: CancellationToken,
) -> tokio::task::JoinHandle<()>
where
    R: SubtreeRepo<RdbContext> + Send + Sync + 'static,
    O: ObjDept<ChapterArtwork, RdbContext>
        + ObjDept<PageImage, RdbContext>
        + ObjDept<ComicCover, RdbContext>
        + ObjDept<TeamAvatar, RdbContext>
        + Send
        + Sync
        + 'static,
{
    tokio::spawn(async move {
        //
        run(
            &token,
            |level| {
                usecase::subtree_delete::sweep((&nucl, &repo, &obj_dept), level)
            },
            || wait(&token),
        )
        .await;
    })
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
