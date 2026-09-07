//! View DTOs for chapter translation port import and export.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::page_port::PageTranslationPortView;

/// JSON document exchanged by the `PopRaKo` translation port.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterTranslationPortView {
    //
    /// Chapter identifier from the exporting chapter.
    pub chapter_id: String,
    /// Ordinal index of the chapter within its comic.
    pub chapter_index: usize,
    /// Optional subtitle from the exporting chapter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_subtitle: Option<String>,

    /// Comic identifier from the exporting comic.
    pub comic_id: String,
    /// Comic title from the exporting comic.
    pub comic_title: String,

    /// Pages and their translation units.
    pub pages: Vec<PageTranslationPortView>,
}

/// A direct artwork upload capability.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterArtworkUploadSlotView {
    //
    /// Presigned PUT URL.
    pub put_url: String,
    /// Headers bound into the upload signature.
    pub headers: BTreeMap<String, String>,
}
