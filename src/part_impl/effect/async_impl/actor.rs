//! Background event actor — receives events from the channel and dispatches
//! them to the appropriate domain actor.

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

/// Unique receive capability for a bounded event queue.
pub struct EffectRecv {
    /// Receiver transferred to the event actor.
    recv: Receiver<Event>,
}

impl EffectRecv {
    /// Creates a receiver capability for the event actor.
    pub const fn new(recv: Receiver<Event>) -> Self {
        Self { recv }
    }

    /// Transfers the receiver to the actor.
    #[must_use]
    pub fn into_recv(self) -> Receiver<Event> {
        self.recv
    }
}

/// Owns cancellation and completion of one background supervisor.
pub struct EffectActorDesc {
    //
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
    /// Task whose completion includes its worker shutdown.
    task: tokio::task::JoinHandle<()>,
}

impl EffectActorDesc {
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

impl Drop for EffectActorDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Background event consumer that receives events from the channel and
/// dispatches them to the appropriate domain actor.
pub struct EffectActor<R> {
    //
    /// Shared repository access for event processing.
    repo: R,
    /// Channel receiver that yields queued events.
    recv: Receiver<Event>,
    /// Cancellation token to stop the background loop.
    token: CancellationToken,
}

impl<R> EffectActor<R> {
    /// Constructs a consumer with its injected repository and unique receiver.
    pub fn new(repo: R, effect_recv: EffectRecv) -> Self {
        //
        Self {
            repo,
            recv: effect_recv.into_recv(),
            token: CancellationToken::new(),
        }
    }

    /// Starts consumption and returns its unique runtime owner.
    #[must_use]
    pub fn run_detach<C>(self) -> EffectActorDesc
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
        let (token, task) = (self.token.clone(), tokio::spawn(self.run::<C>()));

        EffectActorDesc { token, task }
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

        self.recv.close();

        while let Ok(event) = self.recv.try_recv() {
            dispatch::<C, R>(&self.repo, event).await;
        }
    }
}
