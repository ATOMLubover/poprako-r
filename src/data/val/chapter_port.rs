//! Val DTOs for the chapter port domain.

//! Data transfer objects for chapter import/export port use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::chapter_port::{
    ChapterArtworkUploadSlotView, ChapterTranslationPortView,
};
use crate::value::artwork::ArtworkHash;

/// Translation documents generated together by one chapter export.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportChapterTranslationsVal {
    //
    /// `LabelPlus` text, absent when that format was not selected.
    pub label_plus: Option<String>,
    /// Native `PopRaKo` document, absent when that format was not selected.
    pub poprako: Option<ChapterTranslationPortView>,
    /// Page-to-original-filename mappings, absent unless requested.
    pub raw_idents: Option<Vec<ChapterPageRawIdentVal>>,
}

/// One exported Page's original filename mapping.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterPageRawIdentVal {
    //
    /// Stable Page identifier.
    pub page_id: String,
    /// Complete original image filename.
    pub raw_ident: String,
}

/// Summary returned after importing chapter translations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationVal {
    //
    /// Number of pages whose visible Unit content changed.
    pub imported_page_count: usize,
    /// Number of Units created from the imported source.
    pub imported_unit_count: usize,
}

/// Chapter artwork allocation result, including the version on a deduplication hit.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterArtworkVal {
    //
    /// Current artwork generation to confirm.
    #[serde(rename = "artwork_version")]
    pub artwork_ver: u32,
    /// Absent when the requested content is already available.
    pub slot: Option<ChapterArtworkUploadSlotView>,
}

/// Current available artwork and its direct object-storage download address.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportChapterArtworkVal {
    //
    /// Current available object generation.
    #[serde(rename = "artwork_version")]
    pub artwork_ver: u32,
    /// Canonical Base64 SHA-256 identity supplied by the uploader.
    pub artwork_hash: ArtworkHash,
    /// Original file suffix.
    pub ext: String,
    /// Original object URL; file bytes never pass through the API server.
    pub download_url: String,
}
