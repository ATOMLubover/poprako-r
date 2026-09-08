//! User domain events.

/// Event emitted when a new user signs up through an invitation.
///
/// [`UserSignedUp`]: crate::part::effect::event::Event::UserSignedUp
pub struct UserSignedUpEvent {
    //
    /// Identifier of the team the new user was invited to.
    pub team_id: String,
    /// Identifier of the user who sent the invitation.
    pub invitor_id: String,
    /// Qualified identifier of the newly signed-up user.
    pub invitee_qid: String,
}
