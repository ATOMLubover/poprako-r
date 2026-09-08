//! View DTOs for the assignment domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::user::UserInfoView;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::value::role::RoleMask;

/// Presentation-ready chapter assignment information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AssignmentInfoView {
    //
    /// Unique identifier of the assignment.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,
    /// Assigned user identifier.
    pub user_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolved user information, when included.
    pub user: Option<UserInfoView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Resolved chapter information, when included.
    pub chapter: Option<ChapterInfoView>,

    /// Role mask assigned to this user for the chapter.
    pub roles: RoleMask,

    /// Timestamp of creation in milliseconds.
    pub created_at: i64,
    /// Timestamp of last update in milliseconds.
    pub updated_at: i64,
}

impl AssignmentInfoView {
    /// Build a response value and eagerly resolve included user/chapter references.
    /// Converts an assignment model into a presentation-ready value,
    /// resolving included user avatar when present.
    pub fn from_model(
        model: AssignmentInfo,
        user: Option<UserInfoView>,
        chapter: Option<ChapterInfoView>,
    ) -> Self {
        //
        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            user_id: model.user_id,
            user,
            chapter,
            roles: model.roles,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

impl From<AssignmentInfo> for AssignmentInfoView {
    // Convert one persisted assignment into API value shape.
    fn from(model: AssignmentInfo) -> Self {
        Self::from_model(model, None, None)
    }
}
