use super::*;

use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_port::{ExportFormatSpec, TranslationFormat};
use crate::value::chapter_workflow_record::ChapterWorkflowRecordOrigin;
use crate::value::role::{RoleField, RoleMask};

// payloads_round_trip_through_rdb_storage_forms(ChapterWorkflowRecordPayload)(positive): every typed payload decodes from its separate kind and exact JSONB object fields.
#[test]
fn payloads_round_trip_through_rdb_storage_forms() {
    let translator_role = RoleMask::from(RoleField::TRANSLATOR);

    let payloads = vec![
        ChapterWorkflowRecordPayload::ChapterCreated,
        ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
            previous_subtitle: "before".into(),
            next_subtitle: "after".into(),
        },
        ChapterWorkflowRecordPayload::ChapterPinned,
        ChapterWorkflowRecordPayload::ChapterUnpinned,
        ChapterWorkflowRecordPayload::AssignmentCreated {
            subject_user_id: "user-1".into(),
            roles: translator_role,
        },
        ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
            subject_user_id: "user-1".into(),
            previous_roles: translator_role,
            next_roles: RoleMask::from(RoleField::PROOFREADER),
        },
        ChapterWorkflowRecordPayload::AssignmentDeleted {
            subject_user_id: "user-1".into(),
            previous_roles: translator_role,
        },
        ChapterWorkflowRecordPayload::TranslationImported {
            format: TranslationFormat::PopRaKo,
            imported_page_count: 3,
            imported_unit_count: 8,
        },
        ChapterWorkflowRecordPayload::TranslationExported {
            formats: ExportFormatSpec::BOTH,
        },
        ChapterWorkflowRecordPayload::ArtworkExported { artwork_ver: 7 },
        ChapterWorkflowRecordPayload::StageTransitioned {
            stage: Stage::Translate,
            previous_phase: StagePhase::Pending,
            next_phase: StagePhase::Active,
            origin: ChapterWorkflowRecordOrigin::UnitEdit,
        },
    ];

    for payload in payloads {
        let storage_json = encode_payload(&payload);

        assert!(storage_json.is_object());

        assert!(storage_json.get("type").is_none());

        let storage_round_trip =
            decode_payload(payload.kind(), storage_json).unwrap();

        assert_eq!(storage_round_trip, payload);
    }
}

// legacy_export_payload_decodes_as_one_hot_spec(ChapterWorkflowRecordPayload)(compatibility): existing singular-format rows remain readable after aggregate exports are introduced.
#[test]
fn legacy_export_payload_decodes_as_one_hot_spec() {
    let payload = decode_payload(
        ChapterWorkflowRecordKind::TranslationExported,
        serde_json::json!({ "format": "label-plus" }),
    )
    .unwrap();

    assert_eq!(
        payload,
        ChapterWorkflowRecordPayload::TranslationExported {
            formats: ExportFormatSpec::LABEL_PLUS,
        }
    );
}

// storage_payload_rejects_kind_mismatch_and_extra_fields(ChapterWorkflowRecordPayload)(negative): persisted JSONB payloads must exactly match their separate kind.
#[test]
fn storage_payload_rejects_kind_mismatch_and_extra_fields() {
    let mismatch = decode_payload(
        ChapterWorkflowRecordKind::ChapterCreated,
        serde_json::json!({ "unexpected": true }),
    );

    assert!(mismatch.is_err());

    let missing_field = decode_payload(
        ChapterWorkflowRecordKind::StageTransitioned,
        serde_json::json!({ "stage": "translate" }),
    );

    assert!(missing_field.is_err());

    let empty_export = decode_payload(
        ChapterWorkflowRecordKind::TranslationExported,
        serde_json::json!({
            "formats": {
                "label_plus": false,
                "poprako": false,
            },
        }),
    );

    assert!(empty_export.is_err());
}
