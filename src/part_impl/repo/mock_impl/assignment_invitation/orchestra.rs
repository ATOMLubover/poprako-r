use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
    PurgeExpiredAssignmentInvitation,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Internal implementation of `get_info`.
fn get_info(
    state: &MockState,
    oper: &GetAssignmentInvitationInfo<'_>,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    state
        .assignment_invitations
        .iter()
        .find(|info| info.id == oper.id)
        .cloned()
        .ok_or_else(|| expected("error-invitation-not-found"))
}

// Internal implementation of `list_infos`.
fn list_infos(
    state: &MockState,
    oper: &ListAssignmentInvitationInfos<'_>,
) -> Vec<AssignmentInvitationInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut infos = state
        .assignment_invitations
        .iter()
        .filter(|info| {
            //
            info.chapter_id == oper.spec.chapter_id
                && oper
                    .spec
                    .is_pending
                    .is_none_or(|is_pending| info.is_pending == is_pending)
        })
        .cloned()
        .collect::<Vec<_>>();

    infos.sort_by(|left, right| {
        //
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    let offset = oper.spec.offset as usize;

    let end =
        std::cmp::min(offset + oper.spec.limit.get() as usize, infos.len());

    if offset >= infos.len() {
        Vec::new()
    } else {
        infos[offset..end].to_vec()
    }
}

impl<'a> Run<ListAssignmentInvitationInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListAssignmentInvitationInfos<'a>,
    ) -> BaseRest<Vec<AssignmentInvitationInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_infos(&state, oper))
    }
}

impl<'a> Run<GetAssignmentInvitationInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &GetAssignmentInvitationInfo<'a>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_info(&state, oper)
    }
}

impl<'a> Step<CreateAssignmentInvitation<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateAssignmentInvitation<'a>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        //
        if context.state.assignment_invitations.iter().any(|info| {
            //
            info.id == oper.entry.id
                || (info.chapter_id == oper.entry.chapter_id
                    && info.invitee_qid == oper.entry.invitee_qid
                    && info.is_pending)
        }) {
            Err(expected("error-already-exists"))
        } else {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            let time = now();

            let info = AssignmentInvitationInfo {
                id: oper.entry.id.clone(),
                chapter_id: oper.entry.chapter_id.clone(),
                inviter_id: oper.entry.inviter_id.clone(),
                invitee_qid: oper.entry.invitee_qid.clone(),
                code: oper.entry.code.clone(),
                is_pending: true,
                roles: oper.entry.roles,
                created_at: time,
                updated_at: time,
            };

            context.state.assignment_invitations.push(info.clone());

            accept(info)
        }
    }
}

impl<'a> Step<GetAssignmentInvitationInfo<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetAssignmentInvitationInfo<'a>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        get_info(&context.state, oper)
    }
}

impl<'a> Step<GetAssignmentInvitationInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetAssignmentInvitationInfoExcluded<'a>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        //
        context
            .state
            .assignment_invitations
            .iter()
            .find(|info| info.code == oper.code && info.is_pending)
            .cloned()
            .ok_or_else(|| expected("error-no-pending-invitation"))
    }
}

impl<'a> Step<MarkAssignmentInvitationUsed<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &MarkAssignmentInvitationUsed<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let info = context
            .state
            .assignment_invitations
            .iter_mut()
            .find(|info| info.id == oper.id && info.is_pending)
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        info.is_pending = false;

        info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<DeleteAssignmentInvitations<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteAssignmentInvitations<'a>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            DeleteAssignmentInvitations::Id { id } => {
                //
                // Internal implementation detail.
                // Internal implementation detail.
                let position = context
                    .state
                    .assignment_invitations
                    .iter()
                    .position(|info| info.id == *id)
                    .ok_or_else(|| expected("error-invitation-not-found"))?;

                context.state.assignment_invitations.remove(position);

                accept(())
            }

            DeleteAssignmentInvitations::Chapter { chapter_id } => {
                //
                // Internal implementation detail.
                // Internal implementation detail.
                context
                    .state
                    .assignment_invitations
                    .retain(|info| info.chapter_id != *chapter_id);

                accept(())
            }
        }
    }
}

impl<'a> Step<PurgeExpiredAssignmentInvitation<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &PurgeExpiredAssignmentInvitation<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        context
            .state
            .assignment_invitations
            .retain(|info| info.id != oper.id || !info.is_pending);

        accept(())
    }
}

impl<'a> Run<PurgeExpiredAssignmentInvitation<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &PurgeExpiredAssignmentInvitation<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        state
            .assignment_invitations
            .retain(|info| info.id != oper.id || !info.is_pending);

        accept(())
    }
}
