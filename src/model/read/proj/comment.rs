//! Domain models for team board comments.

use time::OffsetDateTime;

use crate::model::read::proj::user::UserInfo;

/// A team board comment as stored in the database, with optional included user data.
#[cfg_attr(test, derive(Clone))]
pub struct CommentInfo {
    //
    /// Unique identifier for the comment.
    pub id: String,

    /// The team board this comment belongs to.
    pub team_id: String,
    /// The user who authored the comment.
    pub user_id: String,
    /// The resolved user record, populated when the include option is requested.
    pub user: Option<UserInfo>,

    /// The text body of the comment.
    pub content: String,

    /// When this comment was first created.
    pub created_at: OffsetDateTime,
}
