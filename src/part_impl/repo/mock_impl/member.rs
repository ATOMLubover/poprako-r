//! Mock member repository operations for in-memory testing.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::spec::member::MemberListSpec;
use crate::model::write::member::MemberEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, DeleteUserMemberships, FindMemberInfo,
    GetMemberInfo, ListMemberInfos, LockTeamMemberInfos, UpdateMember,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::member::MemberInclOpt;

// Internal implementation of `find_user`.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

// Internal implementation of `find_team`.
fn find_team(state: &MockState, team_id: &str) -> Option<TeamInfo> {
    //
    state
        .teams
        .iter()
        .find(|team_info| {
            team_info.id == team_id && !state.deleted_team_ids.contains(team_id)
        })
        .cloned()
}

// Resolve one member by primary key and return expected error when missing.
fn get_member_by_id(state: &MockState, id: &str) -> BaseRest<MemberInfo> {
    //
    state
        .members
        .iter()
        .find(|member| {
            member.id == id && !state.deleted_team_ids.contains(&member.team_id)
        })
        .cloned()
        .ok_or_else(|| expected("error-member-not-found"))
}

// Internal implementation of `apply_user_incl`.
fn apply_user_incl(
    state: &MockState,
    member_info: &mut MemberInfo,
    include_user: bool,
) {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    member_info.user = None;

    if include_user {
        member_info.user = find_user(state, &member_info.user_id);
    }
}

// Internal implementation of `apply_team_incl`.
fn apply_team_incl(
    state: &MockState,
    member_info: &mut MemberInfo,
    include_team: bool,
) {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    member_info.team = None;

    if include_team {
        member_info.team = find_team(state, &member_info.team_id);
    }
}

// Insert a new member record after duplicate checks on id and (user, team).
fn create_member(
    state: &mut MockState,
    entry: &MemberEntry,
) -> BaseRest<MemberInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state.deleted_team_ids.contains(&entry.team_id) {
        return Err(expected("error-team-not-found"));
    }

    if state.members.iter().any(|member| member.id == entry.id) {
        return Err(expected("error-already-exists"));
    }

    if state.members.iter().any(|member| {
        member.user_id == entry.user_id && member.team_id == entry.team_id
    }) {
        return Err(expected("error-already-exists"));
    }

    let member = MemberInfo {
        id: entry.id.clone(),
        user_id: entry.user_id.clone(),
        user_nickname: entry.user_nickname.clone(),
        user_last_active_at: now(),
        team_id: entry.team_id.clone(),
        user: None,
        team: None,
        roles: entry.roles,
    };

    state.members.push(member.clone());

    accept(member)
}

// Internal implementation of `find_member_by_user_id_and_team_id`.
fn find_member_by_user_id_and_team_id(
    state: &MockState,
    user_id: &str,
    team_id: &str,
) -> Option<MemberInfo> {
    //
    if state.deleted_team_ids.contains(team_id) {
        return None;
    }

    state
        .members
        .iter()
        .find(|member| member.user_id == user_id && member.team_id == team_id)
        .cloned()
}

impl<'a> Run<FindMemberInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &FindMemberInfo<'a>,
    ) -> BaseRest<Option<MemberInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(find_member_by_user_id_and_team_id(
            &state,
            oper.user_id,
            oper.team_id,
        ))
    }
}

// Internal implementation of `get_member_info`.
fn get_member_info(
    state: &MockState,
    id: &str,
    incls: &[MemberInclOpt],
) -> BaseRest<MemberInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut member_info = get_member_by_id(state, id)?;

    let include_user = incls.contains(&MemberInclOpt::User);

    let include_team = incls.contains(&MemberInclOpt::Team);

    apply_user_incl(state, &mut member_info, include_user);

    apply_team_incl(state, &mut member_info, include_team);

    accept(member_info)
}

// Internal implementation of `list_member_infos`.
fn list_member_infos(
    state: &MockState,
    spec: &MemberListSpec,
) -> Vec<MemberInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let (offset, limit, incls, mut member_infos) = match spec {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        MemberListSpec::User {
            owner_id,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            limit.get(),
            incl_opt,
            state
                .members
                .iter()
                .filter(|member_info| member_info.user_id == *owner_id)
                .filter(|member_info| {
                    !state.deleted_team_ids.contains(&member_info.team_id)
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),

        MemberListSpec::Team {
            team_id,
            fuzzy_nickname,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            limit.get(),
            incl_opt,
            state
                .members
                .iter()
                .filter(|member_info| member_info.team_id == *team_id)
                .filter(|member_info| {
                    !state.deleted_team_ids.contains(&member_info.team_id)
                })
                .filter(|member_info| {
                    //
                    fuzzy_nickname.as_ref().is_none_or(|keyword| {
                        member_info.user_nickname.contains(keyword.as_str())
                    })
                })
                .filter(|member_info| {
                    //
                    role.is_none_or(|role| {
                        member_info.roles.has_any_role(&[role])
                    })
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),
    };

    let include_user = incls.contains(&MemberInclOpt::User);

    let include_team = incls.contains(&MemberInclOpt::Team);

    for member_info in &mut member_infos {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        apply_user_incl(state, member_info, include_user);

        apply_team_incl(state, member_info, include_team);
    }

    member_infos.sort_by(|left, right| {
        //
        right
            .user_last_active_at
            .cmp(&left.user_last_active_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    let offset = offset as usize;

    let limit = limit as usize;

    if offset >= member_infos.len() {
        Vec::new()
    } else {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let end = std::cmp::min(offset + limit, member_infos.len());

        member_infos[offset..end].to_vec()
    }
}

// Internal implementation of `list_member_infos_by_user`.
fn list_member_infos_by_user(
    state: &MockState,
    user_id: &str,
) -> Vec<MemberInfo> {
    //
    state
        .members
        .iter()
        .filter(|member_info| member_info.user_id == user_id)
        .filter(|member_info| {
            !state.deleted_team_ids.contains(&member_info.team_id)
        })
        .cloned()
        .collect()
}

impl<'a> Run<ListMemberInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListMemberInfos<'a>,
    ) -> BaseRest<Vec<MemberInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        match oper {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            ListMemberInfos::Spec { spec } => {
                accept(list_member_infos(&state, spec))
            }

            ListMemberInfos::User { user_id } => {
                accept(list_member_infos_by_user(&state, user_id))
            }
        }
    }
}

impl<'a, 'b> Run<GetMemberInfo<'a, 'b>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &GetMemberInfo<'a, 'b>) -> BaseRest<MemberInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_member_info(&state, oper.id, oper.incls)
    }
}

impl<'a> Step<CreateMember<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateMember<'a>,
    ) -> BaseRest<MemberInfo> {
        create_member(&mut context.state, oper.entry)
    }
}

impl<'a> Step<UpdateMember<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateMember<'a>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            UpdateMember::UserNickname { repl } => {
                //
                // Internal implementation detail.
                // Internal implementation detail.
                context
                    .state
                    .members
                    .iter_mut()
                    .filter(|member_info| member_info.user_id == repl.user_id)
                    .for_each(|member_info| {
                        member_info.user_nickname = repl.user_nickname.clone();
                    });

                accept(())
            }

            UpdateMember::Role { update } => {
                //
                // Internal implementation detail.
                // Internal implementation detail.
                let member_info = context
                    .state
                    .members
                    .iter_mut()
                    .find(|member_info| member_info.id == update.id)
                    .ok_or_else(|| expected("error-member-not-found"))?;

                member_info.roles = update.roles;

                accept(())
            }
        }
    }
}

impl<'a> Step<ListMemberInfos<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListMemberInfos<'a>,
    ) -> BaseRest<Vec<MemberInfo>> {
        //
        match oper {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            ListMemberInfos::Spec { spec } => {
                accept(list_member_infos(&context.state, spec))
            }

            ListMemberInfos::User { user_id } => {
                accept(list_member_infos_by_user(&context.state, user_id))
            }
        }
    }
}

impl<'a> Step<FindMemberInfo<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindMemberInfo<'a>,
    ) -> BaseRest<Option<MemberInfo>> {
        //
        accept(find_member_by_user_id_and_team_id(
            &context.state,
            oper.user_id,
            oper.team_id,
        ))
    }
}

impl<'a, 'b> Step<GetMemberInfo<'a, 'b>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetMemberInfo<'a, 'b>,
    ) -> BaseRest<MemberInfo> {
        get_member_info(&context.state, oper.id, oper.incls)
    }
}

impl<'a> Step<LockTeamMemberInfos<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &LockTeamMemberInfos<'a>,
    ) -> BaseRest<Vec<MemberInfo>> {
        //
        accept(
            context
                .state
                .members
                .iter()
                .filter(|member_info| member_info.team_id == oper.team_id)
                .cloned()
                .collect(),
        )
    }
}

impl<'a> Step<DeleteMember<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteMember<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let position = context
            .state
            .members
            .iter()
            .position(|member_info| member_info.id == oper.id)
            .ok_or_else(|| expected("error-member-not-found"))?;

        context.state.members.remove(position);

        accept(())
    }
}

impl<'a> Step<DeleteUserMemberships<'a>, MockContext> for Mock {
    // Required Orchestra execution level.
    type Level = ReptRead;

    // Shared repository error type.
    type Error = BaseError;

    // Execute the repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteUserMemberships<'a>,
    ) -> BaseRest<()> {
        //
        context
            .state
            .members
            .retain(|member_info| member_info.user_id != oper.user_id);

        accept(())
    }
}
