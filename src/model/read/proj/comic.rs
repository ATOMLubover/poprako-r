//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoView`] for presentation outside the domain layer.
//!
//! [`ComicInfoView`]: crate::data::view::comic::ComicInfoView

use time::OffsetDateTime;

use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;

/// A comicrecord as stored in the database.
///
/// Each comic belongs to exactly one workset. Cover uploads follow a multi-step
/// flow: a key is allocated via
/// [`AllocComicCoverInstr`], the client uploads to that key, then
/// the upload is confirmed via [`MarkComicCoverUploaded`].
///
/// [`AllocComicCoverInstr`]: crate::data::instr::comic::AllocComicCoverInstr
/// [`MarkComicCoverUploaded`]: crate::part::repo::oper::comic::MarkComicCoverUploaded
#[derive(Clone)]
pub struct ComicInfo {
    //
    /// Unique identifier for the comic record.
    pub id: String,

    /// The workset this comic belongs to.
    pub workset_id: String,
    /// Sorting position of the comic within its workset.
    pub index: usize,

    /// Display title of the comic series or volume.
    pub title: String,
    /// Name of the author or artist responsible for this comic.
    pub author: String,
    /// Optional longer synopsis or editorial notes about the comic.
    pub description: Option<String>,

    /// Denormalised count of chapters attached to this comic.
    pub chapter_count: usize,

    /// The user who created this comic record.
    pub creator_id: String,

    /// The resolved workset record, populated when the include option is requested.
    pub workset: Option<WorksetInfo>,
    /// The resolved team record for the owning workset, populated when requested.
    pub team: Option<TeamInfo>,
    /// The resolved creator user record, populated when the include option is requested.
    pub creator: Option<UserInfo>,

    /// Timestamp of the most recent activity on any chapter under this comic.
    pub last_active_at: OffsetDateTime,
    /// Timestamp when this comic was archived, if it is no longer writable.
    pub archived_at: Option<OffsetDateTime>,

    /// When this comic record was first inserted.
    pub created_at: OffsetDateTime,
    /// When this comic record was last updated.
    pub updated_at: OffsetDateTime,
}
