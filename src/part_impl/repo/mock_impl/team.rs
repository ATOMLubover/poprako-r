//! Mock team repository operations.

// In-memory team-ownership projections.
mod resolve;

use std::cmp::Reverse;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::team::TeamInfo;
use crate::model::write::team::TeamEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, GetTeamInfo, GetTeamInfoExcluded,
    ListTeamInfos, LockTeam, UpdateTeam,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Internal implementation of `create_team`.
fn create_team(state: &mut MockState, entry: &TeamEntry) -> BaseRest<TeamInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state.teams.iter().any(|team_info| team_info.id == entry.id) {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let team_info = TeamInfo {
        id: entry.id.clone(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        created_at: time,
        updated_at: time,
    };

    state.teams.push(team_info.clone());

    accept(team_info)
}

// Internal implementation of `get_team_info`.
fn get_team_info(state: &MockState, id: &str) -> BaseRest<TeamInfo> {
    //
    state
        .teams
        .iter()
        .find(|team_info| {
            team_info.id == id && !state.deleted_team_ids.contains(id)
        })
        .cloned()
        .ok_or_else(|| expected("error-team-not-found"))
}

// Internal implementation of `list_team_infos`.
fn list_team_infos(
    state: &MockState,
    oper: &ListTeamInfos<'_>,
) -> Vec<TeamInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut team_infos = match oper.spec.user_id.as_deref() {
        //
        Some(user_id) => state
            .teams
            .iter()
            .filter(|team_info| {
                //
                !state.deleted_team_ids.contains(&team_info.id)
                    && state.members.iter().any(|member_info| {
                        //
                        member_info.user_id == user_id
                            && member_info.team_id == team_info.id
                    })
            })
            .cloned()
            .collect::<Vec<_>>(),

        None => state
            .teams
            .iter()
            .filter(|team_info| !state.deleted_team_ids.contains(&team_info.id))
            .cloned()
            .collect::<Vec<_>>(),
    };

    team_infos.sort_by_key(|team_info| Reverse(team_info.created_at));

    let offset = oper.spec.offset as usize;

    let limit = oper.spec.limit.get() as usize;

    if offset >= team_infos.len() {
        Vec::new()
    } else {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let end = std::cmp::min(offset + limit, team_infos.len());

        team_infos[offset..end].to_vec()
    }
}

// Internal implementation of `update_team`.
fn update_team(state: &mut MockState, oper: &UpdateTeam<'_>) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state.deleted_team_ids.contains(&oper.repl.id) {
        return Err(expected("error-team-not-found"));
    }

    let team_info = state
        .teams
        .iter_mut()
        .find(|team_info| team_info.id == oper.repl.id)
        .ok_or_else(|| expected("error-team-not-found"))?;

    team_info.name = oper.repl.name.clone();

    team_info.description = oper.repl.description.clone();

    team_info.updated_at = now();

    accept(())
}

impl<'a> Run<CreateTeam<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &CreateTeam<'a>) -> BaseRest<TeamInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        create_team(&mut state, oper.entry)
    }
}

impl<'a> Run<GetTeamInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &GetTeamInfo<'a>) -> BaseRest<TeamInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_team_info(&state, oper.id)
    }
}

impl<'a> Run<ListTeamInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &ListTeamInfos<'a>) -> BaseRest<Vec<TeamInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_team_infos(&state, oper))
    }
}

impl<'a> Run<UpdateTeam<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &UpdateTeam<'a>) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        update_team(&mut state, oper)
    }
}

impl<'a> Step<CreateTeam<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateTeam<'a>,
    ) -> BaseRest<TeamInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        if context.create_team_failure {
            //
            return Err(BaseError::Unrecoverable {
                message: "mock team creation failed".into(),
            });
        }

        create_team(&mut context.state, oper.entry)
    }
}

impl<'a> Step<UpdateTeam<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTeam<'a>,
    ) -> BaseRest<()> {
        update_team(&mut context.state, oper)
    }
}

impl<'a> Step<GetTeamInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTeamInfoExcluded<'a>,
    ) -> BaseRest<TeamInfo> {
        get_team_info(&context.state, oper.id)
    }
}

impl<'a> Step<LockTeam<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &LockTeam<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        get_team_info(&context.state, oper.id)?;

        accept(())
    }
}

impl<'a> Step<AllocTeamWorksetIndex<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocTeamWorksetIndex<'a>,
    ) -> BaseRest<usize> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        // verify the team exists
        if context.state.deleted_team_ids.contains(oper.id) {
            return Err(expected("error-team-not-found"));
        }

        context
            .state
            .teams
            .iter()
            .find(|team| team.id == oper.id)
            .ok_or_else(|| expected("error-team-not-found"))?;

        let workset_index = context
            .state
            .worksets
            .iter()
            .filter(|workset_info| workset_info.team_id == oper.id)
            .count();

        accept(workset_index)
    }
}
