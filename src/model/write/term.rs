//! Domain models for terms stored inside terminology bases.

/// The data needed to create a terminology entry.
#[cfg_attr(test, derive(Clone))]
pub struct TermEntry {
    //
    /// The unique identifier for the new terminology entry.
    pub id: String,

    /// Foreign key of the terminology base to insert this term into.
    pub termbase_id: String,

    /// The source-language term or phrase to add.
    pub source: String,
    /// One or more target-language translations for the new term.
    pub targets: Vec<String>,
    /// Optional annotation or usage note for the new term.
    pub comment: Option<String>,

    /// Foreign key of the user creating this term entry.
    pub creator_id: String,
}

/// Portable terminology-entry content before persistence identity is assigned.
#[cfg_attr(test, derive(Clone))]
pub struct TermImport {
    //
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation or usage note.
    pub comment: Option<String>,
}

/// Mutable terminology-entry fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermRepl {
    //
    /// The unique identifier of the term to update.
    pub id: String,

    /// Updated source-language term or phrase.
    pub source: String,
    /// Updated target-language translations for the term.
    pub targets: Vec<String>,
    /// Updated annotation or usage note for the term.
    pub comment: Option<String>,
}

/// Persistence changes produced by one terminology-base import.
pub struct TermUpsertPlan {
    //
    /// New terminology entries to insert.
    pub entries: Vec<TermEntry>,

    /// Existing terminology entries to update.
    pub updates: Vec<TermRepl>,
}
