//! Domain models for team membership.

use time::OffsetDateTime;

use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::value::role::RoleMask;

/// A membershiprecord linking a user to a team.
///
/// Lightweight projection — does not carry the full user or team record,
/// only the identifiers and a cached nickname for display purposes.
#[cfg_attr(test, derive(Clone))]
pub struct MemberInfo {
    //
    /// Unique identifier for the membership record.
    pub id: String,

    /// The user who holds this membership.
    pub user_id: String,
    /// Display nickname cached from the user record at membership time.
    pub user_nickname: String,
    /// Timestamp of the user's most recent activity, cached for quick sorting.
    pub user_last_active_at: OffsetDateTime,

    /// The team this membership belongs to.
    pub team_id: String,

    /// The resolved user record, populated when the include option is requested.
    pub user: Option<UserInfo>,
    /// The resolved team record, populated when the include option is requested.
    pub team: Option<TeamInfo>,

    /// Bitmask of roles and perms assigned to this member within the team.
    pub roles: RoleMask,
}
