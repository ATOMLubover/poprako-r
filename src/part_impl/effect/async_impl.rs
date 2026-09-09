//! Bounded event producer, independent of its consumer lifecycle.

// Event delivery dispatcher for the asynchronous effect actor.
mod dispatch;

/// Background event actor implementation.
pub mod actor;

#[cfg(test)]
mod tests;

use std::num::NonZeroUsize;

use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tracing::instrument;

use crate::part::effect::Develop;
use crate::part::effect::event::Event;

/// Unique receive capability for a bounded event queue.
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

/// Cloneable best-effort event producer.
#[derive(Clone)]
pub struct AsyncEffectDevelop {
    /// Bounded queue sender.
    send: mpsc::Sender<Event>,
}

impl AsyncEffectDevelop {
    /// Constructs a sender and its unique receiver without starting a consumer.
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> (Self, actor::EffectRecv) {
        //
        let (send, recv) = mpsc::channel(capacity.get());

        (Self { send }, actor::EffectRecv::new(recv))
    }
}

impl Develop for AsyncEffectDevelop {
    // Enqueues best-effort events without waiting for queue capacity.
    #[instrument(level = "info", skip_all)]
    async fn develop(&self, events: Vec<Event>) {
        //
        for event in events {
            //
            match self.send.try_send(event) {
                //
                Ok(()) => {}

                Err(TrySendError::Full(event)) => {
                    //
                    tracing::warn!(
                        event = event_name(&event),
                        "event queue is full, dropping event"
                    );
                }

                Err(TrySendError::Closed(_)) => {
                    break;
                }
            }
        }
    }
}
