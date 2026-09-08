//! Domain event types emitted during use case execution.

/// Chapter-related event payload types.
pub mod chapter;
/// User-related event payload types.
pub mod user;

use crate::part::effect::event::chapter::{
    ChapterPublishedEvent, ChapterWorkflowCompletedEvent,
};
use crate::part::effect::event::user::UserSignedUpEvent;

/// Domain events produced by use cases and dispatched through [`EffectDevelop`].
///
/// Each variant carries a payload struct with the data relevant to that event.
///
/// [`EffectDevelop`]: crate::part::effect::EffectDevelop
pub enum Event {
    //
    /// Emitted when a new user signs up via an invitation.
    UserSignedUp {
        /// Signup event details.
        payload: UserSignedUpEvent,
    },

    /// Emitted when a chapter reaches publish completion.
    ChapterPublished {
        /// Chapter publication event details.
        payload: ChapterPublishedEvent,
    },

    /// Emitted when one chapter workflow stage reaches completion.
    ChapterWorkflowCompleted {
        /// Workflow completion event details.
        payload: ChapterWorkflowCompletedEvent,
    },
}
