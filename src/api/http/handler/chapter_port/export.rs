//! Shared chapter translation export response helpers.

use axum::body::{Body, Bytes};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;
use tracing::instrument;

use crate::api::http::result::HttpError;
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::chapter_port::ExportFormatSpec;

/// Internal payload carrying serialised export content and response metadata.
pub struct TranslationExportPayload {
    //
    // MIME type of the HTTP response body.
    /// MIME type of the HTTP response body.
    content_type: &'static str,
    // File extension for the downloaded filename suffix.
    /// File extension for the downloaded filename suffix.
    ext: &'static str,
    // Raw bytes of the serialised export payload.
    /// Raw bytes of the serialised export payload.
    body: Bytes,
}

/// Loads exported chapter data and builds the response payload.
#[instrument(level = "info", skip_all)]
pub async fn export_payload(
    harn: &AppHarn,
    user_token: UserToken,
    chapter_id: String,
    formats: ExportFormatSpec,
    with_raw_ident: bool,
) -> Result<TranslationExportPayload, HttpError> {
    //
    let val = usecase::chapter_port::export_translation::export_translation::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
        _,
    >(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        user_token,
        chapter_id,
        formats,
        with_raw_ident,
    )
    .await?;

    let body = serde_json::to_vec(&val).map_err(|err| {
        //
        tracing::error!(
            operation = "serialize_chapter_export",
            sdk_err = ?err,
            "JSON SDK serialization error",
        );

        HttpError::internal()
    })?;

    Ok(TranslationExportPayload {
        content_type: "application/json",
        ext: "json",
        body: Bytes::from(body),
    })
}

/// Builds a `200 OK` inline export response with the payload's MIME type.
pub fn body_response(
    payload: TranslationExportPayload,
) -> Result<Response, HttpError> {
    //
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, payload.content_type)
        .body(Body::from(payload.body))
        .map_err(|err| {
            //
            tracing::error!(
                operation = "build_inline_export_response",
                sdk_err = ?err,
                "HTTP SDK response build error",
            );

            HttpError::internal()
        })
}

/// Builds a `200 OK` attachment response with MIME type and filename header.
pub fn download_response(
    filename_base: &str,
    payload: TranslationExportPayload,
) -> Result<Response, HttpError> {
    //
    let filename = format!("{}.{}", filename_base, payload.ext);

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, payload.content_type)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(payload.body))
        .map_err(|err| {
            //
            tracing::error!(
                operation = "build_download_export_response",
                sdk_err = ?err,
                "HTTP SDK response build error",
            );

            HttpError::internal()
        })
}
