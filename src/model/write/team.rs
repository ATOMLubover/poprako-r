//! Domain models for team profile storage.

/// The data needed to create a new team.
#[cfg_attr(test, derive(Clone))]
pub struct TeamEntry {
    //
    /// The unique identifier for the new team.
    pub id: String,

    /// Display name for the team being created.
    pub name: String,
    /// Free-text description for the team being created.
    pub description: String,
}

/// Mutable team profile fields updated together.
pub struct TeamRepl {
    //
    /// The team identifier.
    pub id: String,

    /// The display name.
    pub name: String,
    /// The team description.
    pub description: String,
}
