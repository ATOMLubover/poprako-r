//! Chapter artwork upload limits.

use anyhow::bail;
use serde::Deserialize;

/// Runtime MiB limit for a chapter's artwork file.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct ArtworkConfig {
    /// Maximum size of one direct upload, in MiB.
    pub chapter_artwork_limit: u64,
}

impl ArtworkConfig {
    /// Rejects empty limits and limits that cannot be represented by PUT signing.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        if self.chapter_artwork_limit == 0
            || self.chapter_artwork_limit > (i64::MAX as u64) / (1024 * 1024)
        {
            bail!(
                "chapter_artwork_limit must be positive and fit a signed byte length"
            );
        }

        Ok(())
    }
}

impl Default for ArtworkConfig {
    // Uses the safe default for direct chapter artwork uploads.
    fn default() -> Self {
        //
        Self {
            chapter_artwork_limit: 512,
        }
    }
}
