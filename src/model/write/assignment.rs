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

use crate::value::role::RoleMask;

/// The data needed to insert a new assignment row.
///
/// The `roles` mask specifies the initial roles.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentEntry {
    //
    /// Unique identifier to insert for the new assignment row.
    pub id: String,

    /// Foreign key identifying the chapter whose workflow is joined.
    pub chapter_id: String,
    /// Foreign key identifying the user being added to the chapter workflow.
    pub user_id: String,

    /// Initial bitmask of workflow roles granted to this user.
    pub roles: RoleMask,
}

/// Mutable assignment role mask.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentRoleRepl {
    //
    /// Unique identifier of the assignment whose roles are being updated.
    pub id: String,

    /// Updated role mask.
    pub roles: RoleMask,
}
