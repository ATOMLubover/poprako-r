//! Strongly typed chapter workflow details retained in comic archives.

use serde::Serialize;

use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_port::{ExportFormatSpec, TranslationFormat};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};
use crate::value::role::RoleMask;

/// Strongly typed workflow details retained inside an archived chapter.
#[derive(Serialize)]
#[serde(untagged)]
pub enum ArchivedChapterWorkflowRecordDetail<'a> {
    //
    /// No details for chapter creation, pinning, or unpinning.
    Empty {},

    /// Previous and next chapter subtitles.
    SubtitleUpdated {
        /// Subtitle before the update.
        previous_subtitle: &'a str,
        /// Subtitle after the update.
        next_subtitle: &'a str,
    },

    /// Initial assignment details.
    AssignmentCreated {
        /// User receiving the assignment.
        subject_user_id: &'a str,
        /// Initial assignment roles.
        roles: RoleMask,
    },

    /// Assignment role-mask transition.
    AssignmentRolesUpdated {
        /// User whose assignment changed.
        subject_user_id: &'a str,
        /// Roles before the update.
        previous_roles: RoleMask,
        /// Roles after the update.
        next_roles: RoleMask,
    },

    /// Assignment details retained before deletion.
    AssignmentDeleted {
        /// User whose assignment was deleted.
        subject_user_id: &'a str,
        /// Roles before deletion.
        previous_roles: RoleMask,
    },

    /// Imported translation summary.
    TranslationImported {
        /// Imported content format.
        format: TranslationFormat,
        /// Number of pages whose visible Unit content changed.
        imported_page_count: usize,
        /// Number of Units created from the imported source.
        imported_unit_count: usize,
    },

    /// Exported translation formats.
    TranslationExported {
        /// Generated content formats.
        formats: ExportFormatSpec,
    },

    /// Exported artwork generation.
    ArtworkExported {
        /// Exported artwork generation.
        #[serde(rename = "artwork_version")]
        artwork_ver: u32,
    },

    /// Workflow-stage phase transition.
    StageTransitioned {
        /// Changed workflow stage.
        stage: Stage,
        /// Phase before the transition.
        previous_phase: StagePhase,
        /// Phase after the transition.
        next_phase: StagePhase,
        /// Operation that caused the transition.
        origin: ChapterWorkflowRecordOrigin,
    },
}

impl<'a> From<&'a ChapterWorkflowRecordPayload>
    for ArchivedChapterWorkflowRecordDetail<'a>
{
    // Converts domain workflow details into the stable archive shape.
    fn from(payload: &'a ChapterWorkflowRecordPayload) -> Self {
        //
        match payload {
            //
            ChapterWorkflowRecordPayload::ChapterCreated
            | ChapterWorkflowRecordPayload::ChapterPinned
            | ChapterWorkflowRecordPayload::ChapterUnpinned => Self::Empty {},

            ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            } => Self::SubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            },

            ChapterWorkflowRecordPayload::AssignmentCreated {
                subject_user_id,
                roles,
            } => Self::AssignmentCreated {
                subject_user_id,
                roles: *roles,
            },

            ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            } => Self::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles: *previous_roles,
                next_roles: *next_roles,
            },

            ChapterWorkflowRecordPayload::AssignmentDeleted {
                subject_user_id,
                previous_roles,
            } => Self::AssignmentDeleted {
                subject_user_id,
                previous_roles: *previous_roles,
            },

            ChapterWorkflowRecordPayload::TranslationImported {
                format,
                imported_page_count,
                imported_unit_count,
            } => Self::TranslationImported {
                format: *format,
                imported_page_count: *imported_page_count,
                imported_unit_count: *imported_unit_count,
            },

            ChapterWorkflowRecordPayload::TranslationExported { formats } => {
                Self::TranslationExported { formats: *formats }
            }

            ChapterWorkflowRecordPayload::ArtworkExported { artwork_ver } => {
                //
                Self::ArtworkExported {
                    artwork_ver: *artwork_ver,
                }
            }

            ChapterWorkflowRecordPayload::StageTransitioned {
                stage,
                previous_phase,
                next_phase,
                origin,
            } => Self::StageTransitioned {
                stage: *stage,
                previous_phase: *previous_phase,
                next_phase: *next_phase,
                origin: *origin,
            },
        }
    }
}
