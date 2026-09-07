//! Domain models for team membership.

use crate::value::role::RoleMask;

/// The data needed to insert a new membership row.
///
/// Includes a [`RoleMask`] specifying the member's perms within the team.
#[cfg_attr(test, derive(Clone))]
pub struct MemberEntry {
    //
    /// Unique identifier for the new membership record.
    pub id: String,

    /// The user who will be granted membership.
    pub user_id: String,
    /// Display nickname to cache from the user record at insertion time.
    pub user_nickname: String,

    /// The team the user is joining.
    pub team_id: String,

    /// Bitmask of roles and perms assigned to this member.
    pub roles: RoleMask,
}

/// Mutable fields for a membership record.
pub struct MemberRoleRepl {
    //
    /// Identifies which membership record to update.
    pub id: String,
    /// Updated bitmask of roles and perms for the member.
    pub roles: RoleMask,
}

/// An updated cached member nickname.
pub struct MemberNicknameRepl {
    //
    /// The user whose memberships are being updated.
    pub user_id: String,

    /// The nickname to cache on each membership.
    pub user_nickname: String,
}
