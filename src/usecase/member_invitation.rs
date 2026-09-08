//! Member invitation use cases.

/// Member-invitation presentation assembly.
pub mod view;

#[cfg(test)]
// Unit tests for member invitation creation and cancellation semantics.
mod tests;

use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::member_invitation::{
    MemberInvitationComplex, MemberInvitationPermComplex,
};
use crate::data::instr::member_invitation::{
    CreateMemberInvitationInstr, ListMemberInvitationInfosInstr,
    UpdateMemberInvitationRolesInstr,
};
use crate::data::val::member_invitation::CreateMemberInvitationVal;
use crate::data::view::member_invitation::MemberInvitationInfoView;
use crate::model::read::spec::member_invitation::MemberInvitationListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::member_invitation::{
    MemberInvitationEntry, MemberInvitationRoleRepl,
};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::UserAvatar;
use crate::part::prom::Prom;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::prom::task::Task;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, ListMemberInvitationInfos,
    PurgeExpiredMemberInvitation, UpdateMemberInvitation,
};
use crate::part::repo::oper::team::LockTeam;
use crate::part::repo::oper::user::FindUserInfo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::usecase::member_invitation::view::member_invitation_info_views;
use crate::util::next_snowflake_id;

// Default invitation validity window for member invite tokens.
const EXPIRY_DELAY: Duration = Duration::from_hours(120);

/// Removes one member invitation when it has expired.
#[instrument(level = "info", skip(repo))]
pub async fn purge_expired<C, R>(
    (repo,): (&R,),
    invitation_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: MemberInvitationRepo<C>,
{
    PurgeExpiredMemberInvitation { id: invitation_id }
        .run_on(repo)
        .await?;

    accept(())
}

/// Creates a pending invitation for a team.
#[instrument(level = "info", skip(nucl, repo, prom, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    instr: CreateMemberInvitationInstr,
) -> BaseRest<CreateMemberInvitationVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: MemberInvitationRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + UserRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    let roles = instr.roles;

    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            team_id = %instr.team_id,
            user_id = %token.user_id,
            "expected error: invitation creator membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    MemberInvitationPermComplex::ensure_user_can_create(&member_info)?;

    let (member_invitation_id, code) = nucl
        .coord(async move |context| {
            //

            LockTeam { id: &instr.team_id }
                .step_on(repo, context)
                .await?;

            let invitee_user_info = FindUserInfo {
                qid: &instr.invitee_qid,
            }
            .step_on(repo, context)
            .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                //

                let invitee_member_info = FindMemberInfo {
                    user_id: &invitee_user_info.id,
                    team_id: &instr.team_id,
                }
                .step_on(repo, context)
                .await?;

                if invitee_member_info.is_some() {
                    //
                    let err_message = trl("error-already-team-member");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        team_id = %instr.team_id,
                        user_id = %token.user_id,
                        invitee_user_id = %invitee_user_info.id,
                        invitee_qid = %instr.invitee_qid,
                        "expected error: invitee is already a team member",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    });
                }
            }

            let (member_invitation_id, code) = (
                MemberInvitationComplex::gen_id(),
                MemberInvitationComplex::gen_code(),
            );

            let member_invitation_entry = MemberInvitationEntry {
                id: member_invitation_id,
                team_id: instr.team_id,
                invitor_id: token.user_id,
                invitee_qid: instr.invitee_qid,
                code,
                roles,
            };

            let member_invitation_info = CreateMemberInvitation {
                entry: &member_invitation_entry,
            }
            .step_on(repo, context)
            .await?;

            let purge_event = InvitationPayload::Member {
                invitation_id: member_invitation_info.id.clone(),
            };

            let (purge_payload, purge_task_id) = (
                TaskPayload::Invitation {
                    payload: purge_event,
                },
                next_snowflake_id(),
            );

            let purge_task = Task {
                id: &purge_task_id,
                payload: &purge_payload,
                delay: Some(EXPIRY_DELAY),
            };

            Defer::new(purge_task).step_on(prom, context).await?;

            accept((member_invitation_info.id, member_invitation_info.code))
        })
        .await?;

    accept(CreateMemberInvitationVal {
        id: member_invitation_id,
        code,
    })
}

/// Lists invitations for a team.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListMemberInvitationInfosInstr,
) -> BaseRest<Vec<MemberInvitationInfoView>>
where
    C: Context,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Sync,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-member-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            team_id = %instr.team_id,
            user_id = %token.user_id,
            "expected error: invitation list membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    MemberInvitationPermComplex::ensure_user_can_list_infos(&member_info)?;

    let member_invitation_list_spec = MemberInvitationListSpec {
        team_id: instr.team_id,
        is_pending: instr.is_pending,
        incl_opt: instr.incl_opt,
        offset: instr.offset,
        limit: instr.limit,
    };

    let member_invitation_infos = ListMemberInvitationInfos {
        spec: &member_invitation_list_spec,
    }
    .run_on(repo)
    .await?;

    let member_invitation_info_vals =
        member_invitation_info_views(obj_dept, member_invitation_infos).await?;

    accept(member_invitation_info_vals)
}

/// Updates the roles of an invitation.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_roles<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateMemberInvitationRolesInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Send + Sync,
{
    let member_info = MemberLoader::load_info_from_member_invitation(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &instr.id,
    )
    .await?;

    MemberInvitationPermComplex::ensure_user_can_update_info(&member_info)?;

    nucl.coord(async move |context| {
        //
        let member_invitation_update = MemberInvitationRoleRepl {
            id: instr.id,
            roles: instr.roles,
        };

        UpdateMemberInvitation::Info {
            update: &member_invitation_update,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

/// Deletes an invitation.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Send + Sync,
{
    let member_info = MemberLoader::load_info_from_member_invitation(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &id,
    )
    .await?;

    MemberInvitationPermComplex::ensure_user_can_delete(&member_info)?;

    nucl.coord(async move |context| {
        //
        DeleteMemberInvitation { id: &id }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}
