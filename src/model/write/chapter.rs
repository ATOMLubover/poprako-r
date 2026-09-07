//! Domain models for chapters inside comics — workflow state, progress counters,
//! and display metadata tracked per chapter.
//!
//! See [`StagePhase`] and [`WorkflowStage`](crate::value::chapter::WorkflowStage)
//! for the six production stages and their phase transitions.
//! Convert to [`ChapterInfoView`] for presentation.
//!
//! [`ChapterInfoView`]: crate::data::view::chapter::ChapterInfoView
//! [`StagePhase`]: crate::value::chapter::stage::StagePhase

use crate::value::chapter::mask::StageMask;

/// The data needed to insert a new chapter row.
///
/// Supplied at chapter-creation time by the use case layer. The `id` is
/// typically generated via [`ChapterComplex::gen_id`] before constructing
/// this form. The `is_pinned` initial value is determined by the caller
/// (usually `true` for the first chapter in a comic, or the new
/// highest-index chapter).
///
/// [`ChapterComplex::gen_id`]: crate::complex::chapter::ChapterComplex::gen_id
#[cfg_attr(test, derive(Clone))]
pub struct ChapterEntry {
    //
    /// Unique identifier to insert for the new chapter.
    pub id: String,
    /// Foreign key identifying the parent comic.
    pub comic_id: String,

    /// Whether the new chapter should become the active chapter immediately.
    pub is_pinned: bool,
    /// Ordinal position assigned to the new chapter within the comic.
    pub index: usize,
    /// Chapter subtitle or number for display.
    pub subtitle: String,

    /// Foreign key identifying the user creating the chapter.
    pub creator_id: String,
}

/// Mutable non-workflow fields for a chapter.
///
/// The profile update endpoint changes only `subtitle`. Chapter pinning is a
/// separate action, while internal orchestration can use `pin` to preserve
/// the single-pinned-chapter invariant. Workflow phase transitions are
/// handled via [`ChapterStageRepl`] instead.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterPatch {
    //
    /// Unique identifier of the chapter whose profile fields are being changed.
    pub id: String,

    /// New subtitle value, or `None` to leave unchanged.
    pub subtitle: Option<String>,
    /// New pinned state for a dedicated pinning operation.
    pub pin: Option<bool>,
}

/// Mutable workflow stage mask for a chapter.
///
/// The use case layer validates transition legality before building this update.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterStageRepl {
    //
    /// Unique identifier of the chapter whose stages are being transitioned.
    pub id: String,

    /// Updated workflow stage mask after applying the transition.
    pub stages: StageMask,
}
