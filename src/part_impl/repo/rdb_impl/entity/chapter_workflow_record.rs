//! Diesel entity types for immutable chapter workflow records.

#[cfg(test)]
mod tests;

use diesel::{Insertable, Queryable, Selectable};
use serde_json::json;
use time::OffsetDateTime;

use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part_impl::repo::rdb_impl::schema::t_chapter_workflow_record;
use crate::result::BaseError;
use crate::value::chapter_port::{ExportFormatSpec, TranslationFormat};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordKind, ChapterWorkflowRecordPayload,
};

// Converts a legacy singular export format into the current format spec.
fn normalize_legacy_export_payload(
    kind: ChapterWorkflowRecordKind,
    payload_fields: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), serde_json::Error> {
    //
    match kind {
        //
        ChapterWorkflowRecordKind::TranslationExported
            if payload_fields.len() == 1
                && payload_fields.contains_key("format") =>
        {
            //
            let Some(format) = payload_fields.remove("format") else {
                return Ok(());
            };

            let format = serde_json::from_value::<TranslationFormat>(format)?;

            let formats = ExportFormatSpec::from(format);

            payload_fields
                .insert("formats".into(), serde_json::to_value(formats)?);
        }

        _ => {}
    }

    Ok(())
}

// Encodes one typed workflow payload for the repository JSONB column.
fn encode_payload(payload: &ChapterWorkflowRecordPayload) -> serde_json::Value {
    //
    match payload {
        //
        ChapterWorkflowRecordPayload::ChapterCreated
        | ChapterWorkflowRecordPayload::ChapterPinned
        | ChapterWorkflowRecordPayload::ChapterUnpinned => json!({}),

        ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
            previous_subtitle,
            next_subtitle,
        } => {
            //
            json!({
                "previous_subtitle": previous_subtitle,
                "next_subtitle": next_subtitle,
            })
        }

        ChapterWorkflowRecordPayload::AssignmentCreated {
            subject_user_id,
            roles,
        } => json!({ "subject_user_id": subject_user_id, "roles": roles }),

        ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
            subject_user_id,
            previous_roles,
            next_roles,
        } => {
            //
            json!({
                "subject_user_id": subject_user_id,
                "previous_roles": previous_roles,
                "next_roles": next_roles,
            })
        }

        ChapterWorkflowRecordPayload::AssignmentDeleted {
            subject_user_id,
            previous_roles,
        } => {
            //
            json!({
                "subject_user_id": subject_user_id,
                "previous_roles": previous_roles,
            })
        }

        ChapterWorkflowRecordPayload::TranslationImported {
            format,
            imported_page_count,
            imported_unit_count,
        } => {
            //
            json!({
                "format": format,
                "imported_page_count": imported_page_count,
                "imported_unit_count": imported_unit_count,
            })
        }

        ChapterWorkflowRecordPayload::TranslationExported { formats } => {
            json!({ "formats": formats })
        }

        ChapterWorkflowRecordPayload::ArtworkExported { artwork_ver } => {
            json!({ "artwork_version": artwork_ver })
        }

        ChapterWorkflowRecordPayload::StageTransitioned {
            stage,
            previous_phase,
            next_phase,
            origin,
        } => {
            //
            json!({
                "stage": stage,
                "previous_phase": previous_phase,
                "next_phase": next_phase,
                "origin": origin,
            })
        }
    }
}

// Decodes one repository JSONB object using its separately persisted kind.
fn decode_payload(
    kind: ChapterWorkflowRecordKind,
    payload: serde_json::Value,
) -> Result<ChapterWorkflowRecordPayload, serde_json::Error> {
    // Reconstitute the in-memory tag from the separately persisted kind.
    let mut payload_fields = match payload {
        //
        serde_json::Value::Object(payload_fields) => payload_fields,

        payload => return serde_json::from_value(payload),
    };

    normalize_legacy_export_payload(kind, &mut payload_fields)?;

    let expected_fields = match kind {
        //
        ChapterWorkflowRecordKind::ChapterCreated
        | ChapterWorkflowRecordKind::ChapterPinned
        | ChapterWorkflowRecordKind::ChapterUnpinned => &[][..],

        ChapterWorkflowRecordKind::ChapterSubtitleUpdated => {
            &["previous_subtitle", "next_subtitle"][..]
        }

        ChapterWorkflowRecordKind::AssignmentCreated => {
            &["subject_user_id", "roles"][..]
        }

        ChapterWorkflowRecordKind::AssignmentRolesUpdated => {
            &["subject_user_id", "previous_roles", "next_roles"][..]
        }

        ChapterWorkflowRecordKind::AssignmentDeleted => {
            &["subject_user_id", "previous_roles"][..]
        }

        ChapterWorkflowRecordKind::TranslationImported => {
            &["format", "imported_page_count", "imported_unit_count"][..]
        }

        ChapterWorkflowRecordKind::TranslationExported => &["formats"][..],

        ChapterWorkflowRecordKind::ArtworkExported => &["artwork_version"][..],

        ChapterWorkflowRecordKind::StageTransitioned => {
            &["stage", "previous_phase", "next_phase", "origin"][..]
        }
    };

    let has_expected_fields = payload_fields.len() == expected_fields.len()
        && expected_fields
            .iter()
            .all(|field_name| payload_fields.contains_key(*field_name));

    if !has_expected_fields {
        //
        return Err(serde_json::Error::io(std::io::Error::other(
            "persisted workflow record payload has invalid fields",
        )));
    }

    payload_fields.insert(
        "type".into(),
        serde_json::Value::String(kind.as_str().into()),
    );

    serde_json::from_value(serde_json::Value::Object(payload_fields))
}

/// Raw database row returned from `t_chapter_workflow_record`.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_chapter_workflow_record)]
pub struct ChapterWorkflowRecordInfoRow {
    //
    pub f_id: String,

    pub f_chapter_id: String,
    pub f_actor_user_id: Option<String>,
    pub f_kind: String,
    pub f_payload: serde_json::Value,
    pub f_created_at: OffsetDateTime,
}

impl TryFrom<ChapterWorkflowRecordInfoRow> for ChapterWorkflowRecordInfo {
    type Error = BaseError;

    fn try_from(
        row: ChapterWorkflowRecordInfoRow,
    ) -> Result<Self, Self::Error> {
        //
        let kind = serde_json::from_value::<ChapterWorkflowRecordKind>(
            serde_json::Value::String(row.f_kind),
        )
        .map_err(|error| {
            //
            tracing::error!(
                operation = "decode_chapter_workflow_record_kind",
                record_id = %row.f_id,
                err = ?error,
                "persisted chapter workflow record is corrupt",
            );

            BaseError::Unrecoverable {
                message: "persisted chapter workflow record kind is corrupt"
                    .into(),
            }
        })?;

        let payload = decode_payload(kind, row.f_payload).map_err(|error| {
            //
            tracing::error!(
                operation = "decode_chapter_workflow_record_payload",
                record_id = %row.f_id,
                kind = ?kind,
                err = ?error,
                "persisted chapter workflow record is corrupt",
            );

            BaseError::Unrecoverable {
                message: "persisted chapter workflow record payload is corrupt"
                    .into(),
            }
        })?;

        Ok(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            actor_user_id: row.f_actor_user_id,
            kind,
            payload,
            created_at: row.f_created_at,
        })
    }
}

/// Insertable immutable workflow record row.
#[derive(Insertable)]
#[diesel(table_name = t_chapter_workflow_record)]
pub struct ChapterWorkflowRecordEntryRow<'a> {
    //
    pub f_id: &'a str,

    pub f_chapter_id: &'a str,
    pub f_actor_user_id: Option<&'a str>,
    pub f_kind: String,
    pub f_payload: serde_json::Value,
    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a ChapterWorkflowRecordEntry>
    for ChapterWorkflowRecordEntryRow<'a>
{
    fn from(entry: &'a ChapterWorkflowRecordEntry) -> Self {
        //
        Self {
            f_id: &entry.id,
            f_chapter_id: &entry.chapter_id,
            f_actor_user_id: entry.actor_user_id.as_deref(),
            f_kind: entry.payload.kind().as_str().to_string(),
            f_payload: encode_payload(&entry.payload),
            f_created_at: entry.created_at,
        }
    }
}
