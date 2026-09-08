use poprako_orchestra::Oper;

use crate::model::read::proj::team::TeamInfo;
use crate::model::read::spec::team::TeamListSpec;
use crate::model::write::team::{TeamEntry, TeamRepl};

/// Creates a team.
#[derive(Oper)]
#[oper(output = TeamInfo)]
pub struct CreateTeam<'a> {
    /// The team entry to insert.
    pub entry: &'a TeamEntry,
}

/// Looks up a team by identifier.
#[derive(Oper)]
#[oper(output = TeamInfo)]
pub struct GetTeamInfo<'a> {
    /// The team identifier.
    pub id: &'a str,
}

/// Resolves the owning team from a nested domain resource.
#[derive(Oper)]
#[oper(output = String)]
pub enum ResolveTeamId<'a> {
    //
    /// Resolve from a comic identifier.
    Comic {
        /// The comic identifier.
        id: &'a str,
    },

    /// Resolve from a chapter identifier.
    Chapter {
        /// The chapter identifier.
        id: &'a str,
    },
}

/// Lists team infos matching a filter spec.
#[derive(Oper)]
#[oper(output = Vec<TeamInfo>)]
pub struct ListTeamInfos<'a> {
    /// The specification for filtering listed teams.
    pub spec: &'a TeamListSpec,
}

/// Updates a team.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateTeam<'a> {
    /// The replacement payload.
    pub repl: &'a TeamRepl,
}

/// Looks up a team by identifier, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = TeamInfo)]
pub struct GetTeamInfoExcluded<'a> {
    /// The team identifier.
    pub id: &'a str,
}

/// Locks a team row.
#[derive(Oper)]
#[oper(output = ())]
pub struct LockTeam<'a> {
    /// The team id.
    pub id: &'a str,
}

/// Allocates a sequential index for a new workset under this team.
#[derive(Oper)]
#[oper(output = usize)]
pub struct AllocTeamWorksetIndex<'a> {
    /// The team id.
    pub id: &'a str,
}
