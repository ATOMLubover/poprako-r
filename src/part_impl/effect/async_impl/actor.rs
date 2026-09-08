//! Background event actor — receives events from the channel and dispatches
//! them to the appropriate domain actor.

use std::sync::Arc;

use poprako_orchestra::Context;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::effect::event::Event;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::part_impl::effect::async_impl::dispatch::dispatch;

/// Background event consumer that receives events from the channel and
/// dispatches them to the appropriate domain actor.
pub struct EffectActor<R> {
    //
    /// Shared repository access for event processing.
    repo: Arc<R>,
    /// Channel receiver that yields queued events.
    recv: Receiver<Event>,
    /// Cancellation token to stop the background loop.
    token: CancellationToken,
}

impl<R> EffectActor<R> {
    /// Builds a background actor from its queue and cancellation token.
    pub const fn new(
        repo: Arc<R>,
        recv: Receiver<Event>,
        token: CancellationToken,
    ) -> Self {
        Self { repo, recv, token }
    }

    #[instrument(level = "info", skip_all)]
    /// Runs the event consumer loop, dispatching events until a shutdown signal is received.
    pub async fn run<C>(mut self)
    where
        C: Context + Send,
        R: AssignmentRepo<C>
            + ChapterRepo<C>
            + TeamRepo<C>
            + SystemMailRepo
            + Send
            + Sync,
    {
        loop {
            //
            tokio::select! {
                //
                event = self.recv.recv() => {
                    //
                    match event {
                        //
                        Some(event) => {
                            dispatch::<C, R>(&self.repo, event).await;
                        }

                        None => break,
                    }
                }

                () = self.token.cancelled() => break,
            }
        }

        while let Ok(event) = self.recv.try_recv() {
            dispatch::<C, R>(&self.repo, event).await;
        }
    }
}
