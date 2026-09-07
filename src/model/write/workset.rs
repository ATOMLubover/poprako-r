//! Domain models for worksets — units of work within a team.

/// The data needed to create a new workset.
#[cfg_attr(test, derive(Clone))]
pub struct WorksetEntry {
    //
    /// Server-assigned unique workset identifier.
    pub id: String,

    /// Foreign key referencing the owning team.
    pub team_id: String,
    /// Display ordering index within the team.
    pub index: usize,

    /// Human-readable workset name.
    pub name: String,
    /// Optional longer description of this workset's purpose.
    pub description: Option<String>,
}

/// Mutable profile fields for a workset.
#[cfg_attr(test, derive(Clone))]
pub struct WorksetRepl {
    //
    /// Server-assigned identifier of the workset to update.
    pub id: String,
    /// Updated human-readable workset name.
    pub name: String,
    /// Updated description; `None` means no description.
    pub description: Option<String>,
}
