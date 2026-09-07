use super::*;

use serde_json::json;
use time::OffsetDateTime;

use crate::value::chapter_port::ExportFormatSpec;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordOrigin;
use crate::value::role::RoleField;

// workflow_record_view_preserves_each_typed_event(ChapterWorkflowRecordInfoView)(positive): every domain payload becomes a structured client event without storage JSON or rendered text.
#[test]
fn workflow_record_view_preserves_each_typed_event() {
    let cases = vec![
        (
            ChapterWorkflowRecordPayload::ChapterCreated,
            json!({ "kind": "chapter_created" }),
        ),
        (
            ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
                previous_subtitle: "before".into(),
                next_subtitle: "after".into(),
            },
            json!({
                "kind": "chapter_subtitle_updated",
                "data": {
                    "previous_subtitle": "before",
                    "next_subtitle": "after",
                },
            }),
        ),
        (
            ChapterWorkflowRecordPayload::ChapterPinned,
            json!({ "kind": "chapter_pinned" }),
        ),
        (
            ChapterWorkflowRecordPayload::ChapterUnpinned,
            json!({ "kind": "chapter_unpinned" }),
        ),
        (
            ChapterWorkflowRecordPayload::AssignmentCreated {
                subject_user_id: "subject-user".into(),
                roles: RoleMask::from(RoleField::TRANSLATOR),
            },
            json!({
                "kind": "assignment_created",
                "data": {
                    "subject_user_id": "subject-user",
                    "roles": 2,
                },
            }),
        ),
        (
            ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
                subject_user_id: "subject-user".into(),
                previous_roles: RoleMask::from(RoleField::TRANSLATOR),
                next_roles: RoleMask::from(RoleField::PROOFREADER),
            },
            json!({
                "kind": "assignment_roles_updated",
                "data": {
                    "subject_user_id": "subject-user",
                    "previous_roles": 2,
                    "next_roles": 4,
                },
            }),
        ),
        (
            ChapterWorkflowRecordPayload::AssignmentDeleted {
                subject_user_id: "subject-user".into(),
                previous_roles: RoleMask::from(RoleField::TRANSLATOR),
            },
            json!({
                "kind": "assignment_deleted",
                "data": {
                    "subject_user_id": "subject-user",
                    "previous_roles": 2,
                },
            }),
        ),
        (
            ChapterWorkflowRecordPayload::TranslationImported {
                format: TranslationFormat::PopRaKo,
                imported_page_count: 2,
                imported_unit_count: 4,
            },
            json!({
                "kind": "translation_imported",
                "data": {
                    "format": "poprako",
                    "imported_page_count": 2,
                    "imported_unit_count": 4,
                },
            }),
        ),
        (
            ChapterWorkflowRecordPayload::TranslationExported {
                formats: ExportFormatSpec::BOTH,
            },
            json!({
                "kind": "translation_exported",
                "data": {
                    "formats": {
                        "label_plus": true,
                        "poprako": true,
                    },
                },
            }),
        ),
        (
            ChapterWorkflowRecordPayload::ArtworkExported { artwork_ver: 7 },
            json!({
                "kind": "artwork_exported",
                "data": {
                    "artwork_version": 7,
                },
            }),
        ),
        (
            ChapterWorkflowRecordPayload::StageTransitioned {
                stage: Stage::Translate,
                previous_phase: StagePhase::Pending,
                next_phase: StagePhase::Active,
                origin: ChapterWorkflowRecordOrigin::Manual,
            },
            json!({
                "kind": "stage_transitioned",
                "data": {
                    "stage": "translate",
                    "previous_phase": "pending",
                    "next_phase": "active",
                    "origin": "manual",
                },
            }),
        ),
    ];

    for (payload, expected_event) in cases {
        let workflow_record_info = ChapterWorkflowRecordInfo {
            id: "record-1".into(),
            chapter_id: "chapter-1".into(),
            actor_user_id: Some("actor-user".into()),
            kind: payload.kind(),
            payload,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

        let workflow_record_view =
            ChapterWorkflowRecordInfoView::from(workflow_record_info);

        let serialized = serde_json::to_value(&workflow_record_view).unwrap();

        assert_eq!(serialized["event"], expected_event);

        assert!(serialized.get("kind").is_none());

        assert!(serialized.get("payload").is_none());

        assert!(serialized.get("text").is_none());
    }
}
