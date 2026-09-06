//! Presentation view for immutable chapter workflow records.

#[cfg(test)]
mod tests;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_port::{ExportFormatSpec, TranslationFormat};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};
use crate::value::role::RoleMask;

/// API representation of one immutable chapter workflow record.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterWorkflowRecordInfoView {
    //
    /// Unique workflow record identifier.
    pub id: String,

    /// Chapter that owns this record.
    pub chapter_id: String,
    /// User that caused the record, absent for system work.
    pub actor_user_id: Option<String>,

    /// Strongly typed, language-neutral event data for client-side rendering.
    pub event: ChapterWorkflowRecordEventView,

    /// Record creation time in Unix milliseconds.
    pub created_at: i64,
}

impl From<ChapterWorkflowRecordInfo> for ChapterWorkflowRecordInfoView {
    // Converts a read projection at the presentation boundary.
    fn from(model: ChapterWorkflowRecordInfo) -> Self {
        //
        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            actor_user_id: model.actor_user_id,
            event: model.payload.into(),
            created_at: model.created_at.to_unix_milli(),
        }
    }
}

/// Strongly typed workflow event exposed to clients as a discriminated union.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ChapterWorkflowRecordEventView {
    //
    /// A chapter and its initial creator assignment were created.
    ChapterCreated,

    /// A chapter subtitle changed.
    ChapterSubtitleUpdated {
        /// Subtitle before the update.
        previous_subtitle: String,
        /// Subtitle after the update.
        next_subtitle: String,
    },

    /// A chapter became the pinned chapter for its comic.
    ChapterPinned,

    /// A chapter stopped being the pinned chapter for its comic.
    ChapterUnpinned,

    /// A user received a chapter assignment.
    AssignmentCreated {
        /// User receiving the assignment.
        subject_user_id: String,
        /// Initial assignment roles.
        roles: RoleMask,
    },

    /// An existing chapter assignment changed roles.
    AssignmentRolesUpdated {
        /// User whose roles changed.
        subject_user_id: String,
        /// Roles before the update.
        previous_roles: RoleMask,
        /// Roles after the update.
        next_roles: RoleMask,
    },

    /// A chapter assignment was deleted.
    AssignmentDeleted {
        /// User whose assignment was removed.
        subject_user_id: String,
        /// Roles before deletion.
        previous_roles: RoleMask,
    },

    /// Translation content was imported into a chapter.
    TranslationImported {
        /// Imported content format.
        format: ChapterWorkflowRecordTranslationFormatView,
        /// Number of pages whose visible Unit content changed.
        imported_page_count: usize,
        /// Number of Units created from the imported source.
        imported_unit_count: usize,
    },

    /// Translation content was successfully exported from a chapter.
    TranslationExported {
        /// Generated content formats.
        formats: ExportFormatSpec,
    },

    /// A chapter workflow stage changed phase.
    StageTransitioned {
        /// Changed workflow stage.
        stage: ChapterWorkflowRecordStageView,
        /// Phase before the transition.
        previous_phase: ChapterWorkflowRecordStagePhaseView,
        /// Phase after the transition.
        next_phase: ChapterWorkflowRecordStagePhaseView,
        /// Action that originated the transition.
        origin: ChapterWorkflowRecordOriginView,
    },
}

/// Translation format used by a workflow record event.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordTranslationFormatView {
    //
    /// `LabelPlus` translation format.
    LabelPlus,

    /// `PopRaKo` native translation format.
    #[serde(rename = "poprako")]
    PopRaKo,
}

impl From<TranslationFormat> for ChapterWorkflowRecordTranslationFormatView {
    // Converts the domain format into the workflow-record view.
    fn from(format: TranslationFormat) -> Self {
        //
        match format {
            //
            TranslationFormat::LabelPlus => Self::LabelPlus,

            TranslationFormat::PopRaKo => Self::PopRaKo,
        }
    }
}

/// Workflow stage used by a workflow record event.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordStageView {
    //
    /// Raw-provision stage.
    RawProvide,

    /// Translation stage.
    Translate,

    /// Proofreading stage.
    Proofread,

    /// Typesetting and redraw stage.
    TypesetRedraw,

    /// Review stage.
    Review,

    /// Publishing stage.
    Publish,
}

impl From<Stage> for ChapterWorkflowRecordStageView {
    // Converts the domain stage into the workflow-record view.
    fn from(stage: Stage) -> Self {
        //
        match stage {
            //
            Stage::RawProvide => Self::RawProvide,

            Stage::Translate => Self::Translate,

            Stage::Proofread => Self::Proofread,

            Stage::TypesetRedraw => Self::TypesetRedraw,

            Stage::Review => Self::Review,

            Stage::Publish => Self::Publish,
        }
    }
}

/// Workflow-stage phase used by a workflow record event.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordStagePhaseView {
    //
    /// The stage has not started.
    Pending,

    /// The stage is in progress.
    Active,

    /// The stage has finished.
    Completed,
}

impl From<StagePhase> for ChapterWorkflowRecordStagePhaseView {
    // Converts the domain phase into the workflow-record view.
    fn from(phase: StagePhase) -> Self {
        //
        match phase {
            //
            StagePhase::Pending => Self::Pending,

            StagePhase::Active => Self::Active,

            StagePhase::Completed => Self::Completed,
        }
    }
}

/// Operation source used by a workflow record event.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordOriginView {
    //
    /// Explicit stage operation.
    Manual,

    /// Unit mutation.
    UnitEdit,

    /// Translation import.
    TranslationImport,

    /// Translation export.
    TranslationExport,

    /// Raw-provision completeness check.
    RawProvideCheck,

    /// Client confirmation of the current artwork upload.
    ArtworkUpload,
}

impl From<ChapterWorkflowRecordOrigin> for ChapterWorkflowRecordOriginView {
    // Converts the domain origin into the workflow-record view.
    fn from(origin: ChapterWorkflowRecordOrigin) -> Self {
        //
        match origin {
            //
            ChapterWorkflowRecordOrigin::Manual => Self::Manual,

            ChapterWorkflowRecordOrigin::ArtworkUpload => Self::ArtworkUpload,

            ChapterWorkflowRecordOrigin::UnitEdit => Self::UnitEdit,

            ChapterWorkflowRecordOrigin::TranslationImport => {
                Self::TranslationImport
            }

            ChapterWorkflowRecordOrigin::TranslationExport => {
                Self::TranslationExport
            }

            ChapterWorkflowRecordOrigin::RawProvideCheck => {
                Self::RawProvideCheck
            }
        }
    }
}

impl From<ChapterWorkflowRecordPayload> for ChapterWorkflowRecordEventView {
    // Converts domain event details into the stable presentation union.
    fn from(payload: ChapterWorkflowRecordPayload) -> Self {
        //
        match payload {
            //
            ChapterWorkflowRecordPayload::ChapterCreated => {
                Self::ChapterCreated
            }

            ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            } => Self::ChapterSubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            },

            ChapterWorkflowRecordPayload::ChapterPinned => Self::ChapterPinned,

            ChapterWorkflowRecordPayload::ChapterUnpinned => {
                Self::ChapterUnpinned
            }

            ChapterWorkflowRecordPayload::AssignmentCreated {
                subject_user_id,
                roles,
            } => Self::AssignmentCreated {
                subject_user_id,
                roles,
            },

            ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            } => Self::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            },

            ChapterWorkflowRecordPayload::AssignmentDeleted {
                subject_user_id,
                previous_roles,
            } => Self::AssignmentDeleted {
                subject_user_id,
                previous_roles,
            },

            ChapterWorkflowRecordPayload::TranslationImported {
                format,
                imported_page_count,
                imported_unit_count,
            } => Self::TranslationImported {
                format: format.into(),
                imported_page_count,
                imported_unit_count,
            },

            ChapterWorkflowRecordPayload::TranslationExported { formats } => {
                Self::TranslationExported { formats }
            }

            ChapterWorkflowRecordPayload::StageTransitioned {
                stage,
                previous_phase,
                next_phase,
                origin,
            } => Self::StageTransitioned {
                stage: stage.into(),
                previous_phase: previous_phase.into(),
                next_phase: next_phase.into(),
                origin: origin.into(),
            },
        }
    }
}
