//! View DTOs for the member domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::data::view::team::TeamInfoView;
use crate::data::view::user::UserInfoView;
use crate::model::read::proj::member::MemberInfo;
use crate::value::role::RoleMask;

/// Presentation-ready membership information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MemberInfoView {
    //
    /// Unique member identifier.
    pub id: String,

    /// Owning user identifier.
    pub user_id: String,
    /// Display nickname of the member.
    pub nickname: String,
    /// Unix timestamp of the last activity, in milliseconds.
    pub last_active_at: i64,

    /// Team identifier this membership belongs to.
    pub team_id: String,

    /// Resolved user detail, present when the `user` include option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfoView>,
    /// Resolved team detail, present when the `team` include option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoView>,

    /// Bitmask of roles assigned to this member.
    pub roles: RoleMask,
}

impl MemberInfoView {
    /// Converts a member model into a presentation-ready value,
    /// resolving included user/team instr when present.
    pub fn from_model(
        model: MemberInfo,
        user: Option<UserInfoView>,
        team: Option<TeamInfoView>,
    ) -> Self {
        //
        Self {
            id: model.id,
            user_id: model.user_id,
            nickname: model.user_nickname,
            last_active_at: model.user_last_active_at.to_unix_milli(),
            team_id: model.team_id,
            user,
            team,
            roles: model.roles,
        }
    }
}

impl From<MemberInfo> for MemberInfoView {
    // Convert persisted membership model into response DTO without include expansion.
    fn from(value: MemberInfo) -> Self {
        Self::from_model(value, None, None)
    }
}
