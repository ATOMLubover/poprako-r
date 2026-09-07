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

use crate::value::assignment::AssignmentInclOpt;
use crate::value::pagination::PubListLimit;
use crate::value::role::RoleField;

/// Filtering and pagination parameters for listing chapter assignments.
pub enum AssignmentListSpec {
    //
    /// List assignments on a specific chapter, optionally filtered by role.
    Chapter {
        /// Foreign key scoping the listing to a single chapter.
        chapter_id: String,
        /// Optional role filter; only assignments with this role in their mask
        /// are returned when provided.
        role: Option<RoleField>,
        /// Flags controlling which optional associations are joined into results.
        incl_opt: Vec<AssignmentInclOpt>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: PubListLimit,
    },

    /// List assignments owned by a specific user, optionally filtered by role.
    User {
        /// User identifier scoping the listing to assignments owned by this user.
        owner_id: String,
        /// Optional role filter; only assignments with this role in their mask
        /// are returned when provided.
        role: Option<RoleField>,
        /// Flags controlling which optional associations are joined into results.
        incl_opt: Vec<AssignmentInclOpt>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: PubListLimit,
    },
}
