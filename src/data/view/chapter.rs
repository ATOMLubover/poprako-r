//! View DTOs for the chapter domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::data::view::comic::ComicInfoView;
use crate::data::view::user::UserInfoView;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::value::chapter::mask::StageMask;

/// Presentation-ready chapter information.
///
/// Mirrors [`ChapterInfo`] but converts timestamps to Unix milliseconds
/// and exposes the same grouped field layout as the API response.
///
/// Construct via [`From<ChapterInfo>`] — the conversion is infallible.
///
/// [`ChapterInfo`]: crate::model::read::proj::chapter::ChapterInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterInfoView {
    //
    /// Unique chapter identifier.
    pub id: String,

    /// Owning comic identifier.
    pub comic_id: String,

    /// Included comic information; present only when requested via inclusion
    /// options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comic: Option<ComicInfoView>,

    /// Whether the chapter is pinned to the top of its comic's chapter list.
    pub is_pinned: bool,

    /// Ordinal position within the parent comic.
    pub index: usize,
    /// Display subtitle for the chapter.
    pub subtitle: String,

    /// Total number of image pages in the chapter.
    pub page_count: usize,
    /// Total number of translation units across all pages.
    pub total_unit_count: usize,
    /// Number of units whose translation has been completed.
    pub translated_unit_count: usize,
    /// Number of units whose proofread has been completed.
    pub proofread_unit_count: usize,

    /// Bitmask encoding the current workflow-stage states.
    pub stages: StageMask,

    /// Creator user identifier.
    pub creator_id: String,

    /// Included creator information; present only when requested via inclusion
    /// options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<UserInfoView>,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl ChapterInfoView {
    /// Converts a chapter model into a presentation-ready value,
    /// resolving included creator avatar when present.
    pub fn from_model(
        model: ChapterInfo,
        comic: Option<ComicInfoView>,
        creator: Option<UserInfoView>,
    ) -> Self {
        //
        Self {
            id: model.id,
            comic_id: model.comic_id,
            comic,
            is_pinned: model.is_pinned,
            index: model.index,
            subtitle: model.subtitle,
            page_count: model.page_count,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            stages: model.stages,
            creator_id: model.creator_id,
            creator,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

impl From<ChapterInfo> for ChapterInfoView {
    // Copy persisted chapter fields into the API value shape.
    fn from(model: ChapterInfo) -> Self {
        Self::from_model(model, None, None)
    }
}
