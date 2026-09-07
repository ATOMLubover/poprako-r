//! Chapter artwork object identities and content hashes.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::image::ImageHash;

/// SHA-256 identity using the same canonical Base64 encoding as image uploads.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(feature = "swagger", schema(value_type = String))]
pub struct ArtworkHash(ImageHash);

impl ArtworkHash {
    /// Borrows the SHA-256 bytes used for object deduplication.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Builds a content identity from stored SHA-256 bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(ImageHash::new(bytes))
    }
}

/// Business identity of the chapter's single artwork slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterArtworkKey {
    //
    /// Stable chapter identifier.
    pub chapter_id: String,

    /// File suffix, validated before allocation.
    pub ext: String,
}
