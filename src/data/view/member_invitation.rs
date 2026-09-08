//! View DTOs for the member-invitation domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::user::UserInfoView;
use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::value::role::RoleMask;

/// Presentation-ready member invitation information.
///
/// Mirrors [`MemberInvitationInfo`] with timestamps omitted (the domain
/// model carries no timestamps).
///
/// [`MemberInvitationInfo`]: crate::model::read::proj::member_invitation::MemberInvitationInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MemberInvitationInfoView {
    //
    /// Unique identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,

    /// Identifier of the user who created the invitation.
    pub invitor_id: String,
    /// Resolved invitor user information, present only when requested via incl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitor: Option<UserInfoView>,

    /// QQ ID of the person being invited.
    pub invitee_qid: String,
    /// Opaque invitation code.
    pub code: String,

    /// Whether the invitation has not yet been consumed.
    pub is_pending: bool,

    /// Role mask assigned to the invitee upon acceptance.
    pub roles: RoleMask,
}

impl MemberInvitationInfoView {
    /// Converts an invitation model into a presentation-ready value,
    /// resolving included invitor avatar when present.
    pub fn from_model(
        model: MemberInvitationInfo,
        invitor: Option<UserInfoView>,
    ) -> Self {
        //
        Self {
            id: model.id,
            team_id: model.team_id,
            invitor_id: model.invitor_id,
            invitor,
            invitee_qid: model.invitee_qid,
            code: model.code,
            is_pending: model.is_pending,
            roles: model.roles,
        }
    }
}

impl From<MemberInvitationInfo> for MemberInvitationInfoView {
    // Convert invitation model into response value without preloaded invitor payload.
    fn from(value: MemberInvitationInfo) -> Self {
        Self::from_model(value, None)
    }
}
