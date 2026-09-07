//! Domain models for chapter assignments — role masks tracking which
//! workflow roles a user holds for a chapter.
//!
//! Each assignment carries a [`RoleMask`] bitmap. The individual
//! workflow stages a user can act on are derived from the mask.
//!
//! Convert to [`AssignmentInfoView`] for presentation.
//!
//! [`RoleMask`]: crate::value::role::RoleMask
//! [`AssignmentInfoView`]: crate::data::view::chapter::AssignmentInfoView

use time::OffsetDateTime;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::user::UserInfo;
use crate::value::role::RoleMask;

/// A chapter assignment record linking a user to a chapter with a collection of
/// workflow roles.
///
/// The `roles` mask specifies which workflow roles the user holds for this
/// chapter (translator, proofreader, etc.).
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentInfo {
    //
    /// Unique identifier for the assignment record.
    pub id: String,

    /// Foreign key to the chapter the user is assigned to.
    pub chapter_id: String,
    /// Foreign key to the assigned user.
    pub user_id: String,

    /// Optional joined user data included when the query specifies user expansion.
    pub user: Option<UserInfo>,
    /// Optional joined chapter data included when the query specifies chapter expansion.
    pub chapter: Option<ChapterInfo>,

    /// Bitmask of workflow roles the user holds for this chapter.
    pub roles: RoleMask,

    /// Timestamp when the assignment was created.
    pub created_at: OffsetDateTime,
    /// Timestamp of the last modification to the assignment.
    pub updated_at: OffsetDateTime,
}
