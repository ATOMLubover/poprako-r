//! Mock member-invitation repository operations.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::spec::member_invitation::MemberInvitationListSpec;
use crate::model::write::member_invitation::MemberInvitationEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo,
    GetMemberInvitationInfoExcluded, ListMemberInvitationInfos,
    PurgeExpiredMemberInvitation, UpdateMemberInvitation,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::member_invitation::MemberInvitationInclOpt;

// Internal implementation of `find_user`.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

// Internal implementation of `apply_invitor_incl`.
fn apply_invitor_incl(
    state: &MockState,
    member_invitation_info: &mut MemberInvitationInfo,
    include_invitor: bool,
) {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    member_invitation_info.invitor = None;

    if include_invitor {
        //
        member_invitation_info.invitor =
            find_user(state, &member_invitation_info.invitor_id);
    }
}

// Internal implementation of `list_member_invitation_infos`.
fn list_member_invitation_infos(
    state: &MockState,
    spec: &MemberInvitationListSpec,
) -> Vec<MemberInvitationInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut member_invitation_infos = state
        .member_invitations
        .iter()
        .filter(|member_invitation_info| {
            //
            member_invitation_info.team_id == spec.team_id
                && spec.is_pending.is_none_or(|is_pending| {
                    member_invitation_info.is_pending == is_pending
                })
        })
        .cloned()
        .collect::<Vec<_>>();

    member_invitation_infos.sort_by(|left, right| left.id.cmp(&right.id));

    let include_invitor =
        spec.incl_opt.contains(&MemberInvitationInclOpt::Invitor);

    for member_invitation_info in &mut member_invitation_infos {
        apply_invitor_incl(state, member_invitation_info, include_invitor);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit.get() as usize;

    if offset >= member_invitation_infos.len() {
        Vec::new()
    } else {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let end = std::cmp::min(offset + limit, member_invitation_infos.len());

        member_invitation_infos[offset..end].to_vec()
    }
}

// Internal implementation of `get_member_invitation_info`.
fn get_member_invitation_info(
    state: &MockState,
    oper: &GetMemberInvitationInfo<'_, '_>,
) -> BaseRest<MemberInvitationInfo> {
    //
    match oper {
        //
        // Internal state field `GetMemberInvitationInfo`.
        // Internal implementation detail.
        GetMemberInvitationInfo::Id { id, incls } => {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            let mut member_invitation_info = state
                .member_invitations
                .iter()
                .find(|member_invitation_info| member_invitation_info.id == *id)
                .cloned()
                .ok_or_else(|| expected("error-invitation-not-found"))?;

            let include_invitor =
                incls.contains(&MemberInvitationInclOpt::Invitor);

            apply_invitor_incl(
                state,
                &mut member_invitation_info,
                include_invitor,
            );

            accept(member_invitation_info)
        }

        GetMemberInvitationInfo::Code { code } => state
            .member_invitations
            .iter()
            .find(|member_invitation_info| {
                //
                member_invitation_info.code == *code
                    && member_invitation_info.is_pending
            })
            .cloned()
            .ok_or_else(|| expected("error-no-pending-invitation")),
    }
}

// Internal implementation of `create_member_invitation`.
fn create_member_invitation(
    state: &mut MockState,
    entry: &MemberInvitationEntry,
) -> BaseRest<MemberInvitationInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state
        .member_invitations
        .iter()
        .any(|member_invitation_info| member_invitation_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    if state
        .member_invitations
        .iter()
        .any(|member_invitation_info| {
            //
            member_invitation_info.team_id == entry.team_id
                && member_invitation_info.invitee_qid == entry.invitee_qid
                && member_invitation_info.is_pending
        })
    {
        return Err(expected("error-already-exists"));
    }

    let member_invitation_info = MemberInvitationInfo {
        id: entry.id.clone(),
        team_id: entry.team_id.clone(),
        invitor: None,
        invitor_id: entry.invitor_id.clone(),
        invitee_qid: entry.invitee_qid.clone(),
        code: entry.code.clone(),
        is_pending: true,
        roles: entry.roles,
    };

    state
        .member_invitations
        .push(member_invitation_info.clone());

    accept(member_invitation_info)
}

// Internal implementation of `update_member_invitation`.
fn update_member_invitation(
    state: &mut MockState,
    oper: &UpdateMemberInvitation<'_>,
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    match oper {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        UpdateMemberInvitation::Info { update } => {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            let member_invitation_info = state
                .member_invitations
                .iter_mut()
                .find(|member_invitation_info| {
                    member_invitation_info.id == update.id
                })
                .ok_or_else(|| expected("error-invitation-not-found"))?;

            member_invitation_info.roles = update.roles;
        }

        UpdateMemberInvitation::MarkUsed { id } => {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            let member_invitation_info = state
                .member_invitations
                .iter_mut()
                .find(|member_invitation_info| {
                    //
                    member_invitation_info.id == *id
                        && member_invitation_info.is_pending
                })
                .ok_or_else(|| expected("error-invitation-not-found"))?;

            member_invitation_info.is_pending = false;
        }
    }

    accept(())
}

impl<'a> Run<ListMemberInvitationInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListMemberInvitationInfos<'a>,
    ) -> BaseRest<Vec<MemberInvitationInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_member_invitation_infos(&state, oper.spec))
    }
}

impl<'a, 'b> Run<GetMemberInvitationInfo<'a, 'b>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &GetMemberInvitationInfo<'a, 'b>,
    ) -> BaseRest<MemberInvitationInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_member_invitation_info(&state, oper)
    }
}

impl<'a> Step<CreateMemberInvitation<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateMemberInvitation<'a>,
    ) -> BaseRest<MemberInvitationInfo> {
        create_member_invitation(&mut context.state, oper.entry)
    }
}

impl<'a, 'b> Step<GetMemberInvitationInfo<'a, 'b>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetMemberInvitationInfo<'a, 'b>,
    ) -> BaseRest<MemberInvitationInfo> {
        get_member_invitation_info(&context.state, oper)
    }
}

impl<'a> Step<UpdateMemberInvitation<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateMemberInvitation<'a>,
    ) -> BaseRest<()> {
        update_member_invitation(&mut context.state, oper)
    }
}

impl<'a> Step<GetMemberInvitationInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetMemberInvitationInfoExcluded<'a>,
    ) -> BaseRest<MemberInvitationInfo> {
        //
        get_member_invitation_info(
            &context.state,
            &GetMemberInvitationInfo::Code { code: oper.code },
        )
    }
}

impl<'a> Step<DeleteMemberInvitation<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteMemberInvitation<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let position = context
            .state
            .member_invitations
            .iter()
            .position(|member_invitation_info| {
                member_invitation_info.id == oper.id
            })
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        context.state.member_invitations.remove(position);

        accept(())
    }
}

impl<'a> Step<PurgeExpiredMemberInvitation<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &PurgeExpiredMemberInvitation<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        context
            .state
            .member_invitations
            .retain(|member_invitation_info| {
                //
                member_invitation_info.id != oper.id
                    || !member_invitation_info.is_pending
            });

        accept(())
    }
}

impl<'a> Run<PurgeExpiredMemberInvitation<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &PurgeExpiredMemberInvitation<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        state.member_invitations.retain(|member_invitation_info| {
            //
            member_invitation_info.id != oper.id
                || !member_invitation_info.is_pending
        });

        accept(())
    }
}
