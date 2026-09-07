//! Instr DTOs for the page domain.

//! Data transfer objects for page use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::write::page::PageImageSpec;
use crate::value::image::{ImageExt, ImageHash};

/// Input parameters for reserving all page images of a chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPagesInstr {
    //
    /// Target chapter identifier.
    pub chapter_id: String,
    /// Page images to allocate for the chapter.
    pub pages: Vec<PageImageInstr>,
}

/// One page image in a complete chapter manifest.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageImageInstr {
    //
    /// Complete original filename. Omission or null replaces it with no name.
    #[serde(default)]
    pub raw_ident: Option<String>,

    //
    /// Existing page identifier, if updating an existing page.
    pub page_id: Option<String>,
    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// Size used to constrain a newly requested upload slot.
    ///
    /// Existing manifest entries may omit this when no upload slot is needed.
    pub new_byte_len: Option<u64>,
    /// File format.
    pub ext: ImageExt,
}

impl From<PageImageInstr> for PageImageSpec {
    // Map page image parameters directly to the domain spec.
    fn from(instr: PageImageInstr) -> Self {
        //
        Self {
            raw_ident: instr.raw_ident,
            page_id: instr.page_id,
            image_hash: instr.image_hash,
            new_byte_len: instr.new_byte_len,
            ext: instr.ext,
        }
    }
}

/// Input parameters for reserving one page image.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocPageImageInstr {
    //
    /// Complete original filename. Omission or null replaces it with no name.
    #[serde(default)]
    pub raw_ident: Option<String>,

    //
    /// Content hash of the page image to allocate.
    pub image_hash: ImageHash,
    /// Size of the page image in bytes.
    pub new_byte_len: u64,
    /// File format.
    pub ext: ImageExt,
}

/// Input parameters for confirming a page image upload completed.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkPageImageUploadedInstr {
    /// Image version from the allocation, used as an idempotency guard.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
}

/// Input parameters for listing all pages under one chapter.
#[derive(Debug)]
pub struct ListPageInfosInstr {
    /// Chapter whose pages to list.
    pub chapter_id: String,
}

/// Input parameters for listing Pages with proofread text diffs.
#[derive(Debug)]
pub struct ListEdittedDiffPageIdsInstr {
    /// Chapter whose Pages are checked for Unit text diffs.
    pub chapter_id: String,
}
