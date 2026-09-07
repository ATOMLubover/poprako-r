//! Domain models for member invitations.

use crate::model::read::proj::user::UserInfo;
use crate::value::role::RoleMask;

/// An invitation record for joining a team.
///
/// Carries an opaque invitation code, the inviter and invitee identifiers,
/// a pending flag indicating whether the invitation has been consumed,
/// and the [`RoleMask`] that will be assigned upon acceptance.
#[cfg_attr(test, derive(Clone))]
pub struct MemberInvitationInfo {
    //
    /// Unique identifier for the invitation record.
    pub id: String,

    /// The team the invitation grants access to.
    pub team_id: String,

    /// The resolved inviter user record, populated when the include option is requested.
    pub invitor: Option<UserInfo>,

    /// The user who created and sent the invitation.
    pub invitor_id: String,
    /// The qualified identifier of the user being invited.
    pub invitee_qid: String,

    /// Opaque token the invitee presents to accept the invitation.
    pub code: String,

    /// Whether this invitation has not yet been consumed.
    pub is_pending: bool,

    /// Role mask that will be assigned to the member upon acceptance.
    pub roles: RoleMask,
}
