use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::util::Patch;

/// One literal text transform applied against an original Unit field value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitTextTransform {
    //
    /// Literal text selected in the original value.
    pub origin: String,
    /// Literal text inserted in place of each match.
    pub target: String,
}

/// Text transforms requested for one permanent Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitTransform {
    //
    /// Permanent target Unit ID.
    pub unit_id: String,
    /// Literal transforms evaluated against the same original text.
    pub transforms: Vec<UnitTextTransform>,
}

/// One normalized Unit mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitEdit {
    //
    /// Creates one Unit with complete structural fields.
    Create {
        /// Permanent Unit ID.
        id: String,
        /// Unit before which this Unit is inserted, or the tail.
        next_id: Option<String>,

        /// Whether this Unit identifies a speech bubble.
        is_bubble: bool,
        /// Initial page-relative coordinate.
        coord: UnitCoord,

        /// Optional initial translation.
        translation: Option<UnitTranslation>,
        /// Optional initial revision.
        revision: Option<UnitRevision>,
    },

    /// Restores or patches one persisted Unit.
    Save {
        /// Permanent target Unit ID.
        id: String,
        /// Three-state successor pointer patch.
        next_id: Patch<String>,

        /// Optional updated speech-bubble flag.
        is_bubble: Option<bool>,
        /// Optional updated coordinate.
        coord: Option<UnitCoord>,

        /// Three-state translation patch.
        translation: Patch<UnitTranslation>,
        /// Three-state revision patch.
        revision: Patch<UnitRevision>,
    },

    /// Hide one persisted Unit while retaining its chain node and content.
    Delete {
        /// Permanent target Unit ID.
        id: String,
    },
}
