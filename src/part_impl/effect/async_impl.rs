//! Async background dispatcher for side-effect events.

// Background event actor runner.
mod actor;
// Maps delivered events to domain use cases.
mod dispatch;

#[cfg(test)]
// Mock and integration tests for async dispatcher behavior.
mod tests;

use std::num::NonZeroUsize;
use std::sync::Arc;

use poprako_orchestra::Context;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::effect::Develop;
use crate::part::effect::event::Event;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;

/// Async side-effect dispatcher backed by a bounded channel.
///
/// Spawns a background actor on construction that drains the queue and
/// dispatches each event to the appropriate domain actor. Use
/// [`close`](AsyncEffectDevelop::close) before dropping to drain pending
/// events gracefully.
pub struct AsyncEffectDevelop {
    //
    // Internal state field `send`.
    /// Bounded channel sender for enqueueing events.
    send: Sender<Event>,
    /// Cancellation token to signal graceful shutdown.
    token: CancellationToken,
    /// Watch receiver that signals when background processing completes.
    done: watch::Receiver<bool>,
}

impl AsyncEffectDevelop {
    /// Creates a dispatcher and launches its background task.
    pub fn new<C, R>(repo: Arc<R>, buf_size: NonZeroUsize) -> Self
    where
        C: Context + Send + 'static,
        R: AssignmentRepo<C>
            + ChapterRepo<C>
            + TeamRepo<C>
            + SystemMailRepo
            + Send
            + Sync
            + 'static,
    {
        let (send, recv) = tokio::sync::mpsc::channel(buf_size.get());

        let token = CancellationToken::new();

        let (done_send, done) = watch::channel(false);

        let actor = actor::EffectActor::<R>::new(repo, recv, token.clone());

        tokio::spawn(async move {
            //
            // Internal implementation detail.
            actor.run().await;

            done_send.send_replace(true);
        });

        Self { send, token, done }
    }

    /// Stops accepting new events and waits for queued events to finish.
    #[instrument(level = "info", skip_all)]
    pub async fn close(&self) {
        //
        // Internal implementation detail.
        self.token.cancel();

        let mut done = self.done.clone();

        if let Err(error) = done.wait_for(|done| *done).await {
            //
            tracing::error!(
                err = %error,
                "[AsyncEffectDevelop::close] background task ended without completion",
            );
        }
    }
}

impl Clone for AsyncEffectDevelop {
    // Internal implementation of `clone`.
    fn clone(&self) -> Self {
        //
        Self {
            send: self.send.clone(),
            token: self.token.clone(),
            done: self.done.clone(),
        }
    }
}

impl Develop for AsyncEffectDevelop {
    // Internal implementation of `develop`.
    #[instrument(level = "info", skip_all)]
    async fn develop(&self, events: Vec<Event>) {
        //
        if self.token.is_cancelled() {
            return;
        }

        for event in events {
            //
            if let Err(e) = self.send.try_send(event) {
                //
                match e {
                    //
                    // Internal implementation detail.
                    TrySendError::Full(_) | TrySendError::Closed(_)
                        if self.token.is_cancelled() =>
                    {
                        break;
                    }

                    // Internal implementation detail.
                    TrySendError::Full(event) => {
                        //
                        tracing::warn!(
                            event = event_name(&event),
                            "[AsyncEffectDevelop::develop] event queue is full, dropping event",
                        );
                    }

                    TrySendError::Closed(event) => {
                        //
                        // Internal implementation detail.
                        tracing::warn!(
                            event = event_name(&event),
                            "[AsyncEffectDevelop::develop] event queue is closed, dropping event",
                        );

                        break;
                    }
                }
            }
        }
    }
}

impl Drop for AsyncEffectDevelop {
    // Internal implementation of `drop`.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Returns a human-readable label for a domain event variant.
// Used by queue diagnostics when logging full/closed queue drop events.
const fn event_name(event: &Event) -> &'static str {
    //
    match event {
        //
        // Internal state field Event.
        Event::UserSignedUp { payload: _ } => "user_signed_up",

        Event::ChapterPublished { payload: _ } => "chapter_published",

        Event::ChapterWorkflowCompleted { payload: _ } => {
            "chapter_workflow_completed"
        }
    }
}
