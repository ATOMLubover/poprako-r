//! Domain models for pages inside chapters — per-page unit tracking and
//! image-storage metadata.
//!
//! Each page belongs to exactly one chapter and carries denormalised progress
//! counters that aggregate to the parent chapter's overview totals.
//!
//! Convert to [`PageInfoView`] for presentation.
//!
//! [`PageInfoView`]: crate::data::view::page::PageInfoView

use crate::value::image::{ImageExt, ImageHash};

/// The data needed to insert one page row.
#[cfg_attr(test, derive(Clone))]
pub struct PageEntry {
    //
    /// The unique identifier for the new page record.
    pub id: String,

    /// Foreign key of the parent chapter to insert this page into.
    pub chapter_id: String,
    /// Zero-based ordinal position for this page within the chapter.
    pub index: usize,
}

/// One page-image identity supplied to manifest planning.
pub struct PageImageSpec {
    //
    /// Complete original image filename, or no original identifier.
    pub raw_ident: Option<String>,

    //
    /// Existing page identifier, if the manifest retains a known page.
    pub page_id: Option<String>,
    /// Content-addressable hash of the page image file.
    pub image_hash: ImageHash,
    /// File size when this manifest entry requests an upload slot.
    pub new_byte_len: Option<u64>,
    /// File format of the page image.
    pub ext: ImageExt,
}

/// Final persisted identity and position for one page-manifest item.
pub struct PageManifestEntry {
    //
    /// The unique identifier of the retained or newly created page.
    pub id: String,
    /// Foreign key of the parent chapter for this manifest item.
    pub chapter_id: String,
    /// Final ordinal position of the page within the chapter.
    pub index: usize,
}

/// Complete source filenames for the selected pages.
pub struct PageRawIdentsRepl<'a> {
    /// Paired page IDs and filenames; None means no original identifier.
    pub idents: &'a [(&'a str, Option<&'a str>)],
}
