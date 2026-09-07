//! Domain models for user authentication and profile storage.

/// The data needed to insert a new user row.
#[cfg_attr(test, derive(Clone))]
pub struct UserEntry {
    //
    /// Server-assigned unique user identifier.
    pub id: String,

    /// Third-party OAuth or QID provider identifier for this user.
    pub qid: String,
    /// Display name shown throughout the application.
    pub nickname: String,

    /// Bcrypt or similar hashed password for credential verification.
    pub password_hash: String,
}

/// Mutable user profile fields updated together.
pub struct UserInfoRepl {
    //
    /// The user identifier.
    pub id: String,

    /// The OAuth qualified identifier.
    pub qid: String,
    /// The display nickname.
    pub nickname: String,
}

/// A user's credential fields updated together.
pub struct UserCredsRepl {
    //
    /// The user identifier.
    pub id: String,

    /// The new password hash.
    pub password_hash: String,
}
