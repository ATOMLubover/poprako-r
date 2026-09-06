//! Chapter translation and artwork port handlers.

// Shared translation export response helpers.
mod export;

#[cfg(test)]
mod tests;

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;
#[cfg(feature = "swagger")]
use crate::data::val::chapter_port::ExportChapterTranslationsVal;

use crate::api::http::result::{
    Accept as _, HttpError, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::chapter_port::{
    AllocChapterArtworkInstr, ExportChapterTranslationInstr,
    ImportChapterTranslationInstr, MarkChapterArtworkUploadedInstr,
};
use crate::data::val::chapter_port::{
    AllocChapterArtworkVal, ExportChapterArtworkVal,
    ImportChapterTranslationVal,
};
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;

/// `POST /api/v1/chapters/{chapter_id}/translations/import` — import translations.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/translations/import",
    tag = "chapter-port",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = ImportChapterTranslationInstr,
    responses(
        (status = 200, description = "Translations imported", body = HttpBody<ImportChapterTranslationVal>),
        (status = 403, description = "No perm to import into this chapter"),
        (status = 422, description = "Invalid import content for the selected format"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn import_translation(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<ImportChapterTranslationInstr>,
) -> HttpResult<ImportChapterTranslationVal> {
    //
    usecase::chapter_port::import_translation::import_translation::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
    >(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
        chapter_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/translations/export` — export response body.
///
/// `format` selects one or both generated documents as a comma-separated value.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/translations/export",
    tag = "chapter-port",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("format" = String, Query, description = "Comma-separated export formats: poprako,label_plus"),
    ),
    responses(
        (status = 200, description = "Selected translation exports", body = ExportChapterTranslationsVal, content_type = "application/json"),
        (status = 403, description = "No perm to export this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export_translation(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ExportChapterTranslationInstr>,
) -> Result<Response, HttpError> {
    //
    let payload =
        export::export_payload(&harn, user_token, chapter_id, instr.format)
            .await?;

    export::body_response(payload)
}

/// `GET /api/v1/chapters/{chapter_id}/translations/export/download` — export as file download.
///
/// The downloaded JSON contains every document selected by `format`.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/translations/export/download",
    tag = "chapter-port",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("format" = String, Query, description = "Comma-separated export formats: poprako,label_plus"),
    ),
    responses(
        (status = 200, description = "Selected translation exports download", body = ExportChapterTranslationsVal, content_type = "application/json"),
        (status = 403, description = "No perm to export this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export_translation_download(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ExportChapterTranslationInstr>,
) -> Result<Response, HttpError> {
    //
    let filename = format!("chapter_{}", chapter_id);

    let payload =
        export::export_payload(&harn, user_token, chapter_id, instr.format)
            .await?;

    export::download_response(&filename, payload)
}

/// Allocates a direct chapter artwork upload; an available duplicate has no slot.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/chapters/{chapter_id}/artwork/alloc", tag = "chapter-port",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = AllocChapterArtworkInstr,
    responses(
        (status = 200, description = "Current artwork version and optional PUT capability", body = HttpBody<AllocChapterArtworkVal>),
        (status = 403, description = "Chapter artwork role required"),
        (status = 422, description = "Invalid allocation or frozen chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_artwork(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<AllocChapterArtworkInstr>,
) -> HttpResult<AllocChapterArtworkVal> {
    //
    usecase::chapter_port::artwork::alloc_artwork::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
        _,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            &harn.config().artwork,
        ),
        user_token,
        chapter_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// Optimistically confirms artwork and atomically completes typesetting/redraw.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/chapters/{chapter_id}/artwork/mark-uploaded", tag = "chapter-port",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = MarkChapterArtworkUploadedInstr,
    responses(
        (status = 204, description = "Current artwork confirmed and typesetting completed"),
        (status = 403, description = "Chapter artwork role required"),
        (status = 422, description = "Missing or stale artwork version, or frozen chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_artwork_uploaded(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkChapterArtworkUploadedInstr>,
) -> HttpNoContent {
    //
    usecase::chapter_port::artwork::mark_artwork_uploaded::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
        _,
        _,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            harn.develop(),
        ),
        user_token,
        chapter_id,
        instr,
    )
    .await?;

    no_content()
}

/// Returns the original object URL of the current available artwork.
#[cfg_attr(feature = "swagger", utoipa::path(
    get, path = "/api/v1/chapters/{chapter_id}/artwork/export", tag = "chapter-port",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Artwork identity and original download URL", body = HttpBody<ExportChapterArtworkVal>),
        (status = 403, description = "Chapter export access required"),
        (status = 422, description = "Chapter artwork is unavailable"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export_artwork(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ExportChapterArtworkVal> {
    //
    usecase::chapter_port::artwork::export_artwork::<
        RdbContext<ReptRead>,
        HybRepo,
        _,
    >((harn.repo(), harn.obj_dept()), user_token, chapter_id)
    .await?
    .accept(StatusCode::OK)
}
