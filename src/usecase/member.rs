//! Member use cases: create, join, list, role update, and deletion.

/// Member presentation assembly.
pub mod view;

#[cfg(test)]
// Unit tests for team membership and invitation boundary conditions.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::instr::member::{
    CreateMemberInstr, JoinTeamInstr, ListMemberInfosInstr,
    UpdateMemberRolesInstr,
};
use crate::data::val::member::CreateMemberVal;
use crate::data::view::member::MemberInfoView;
use crate::model::read::spec::member::MemberListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::member::{MemberEntry, MemberRoleRepl};
use crate::part::nucl::{ReptRead, Serial};
use crate::part::obj_dept::{TeamAvatar, UserAvatar};
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    LockTeamMemberInfos, UpdateMember,
};
use crate::part::repo::oper::member_invitation::{
    GetMemberInvitationInfoExcluded, UpdateMemberInvitation,
};
use crate::part::repo::oper::team::LockTeam;
use crate::part::repo::oper::user::GetUserInfoExcluded;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::member::view::{member_info_view, member_info_views};

/// Creates one member under a team.
///
/// The caller must be a team admin. The target user and team are locked in
/// the transaction before inserting the membership.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateMemberInstr,
) -> BaseRest<CreateMemberVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: MemberRepo<C> + TeamRepo<C> + UserRepo<C> + Send + Sync,
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
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    MemberPermComplex::ensure_user_can_create(&member_info)?;

    let member_id = nucl
        .coord(async move |context| {
            //

            let user_info = GetUserInfoExcluded { id: &instr.user_id }
                .step_on(repo, context)
                .await?;

            LockTeam { id: &instr.team_id }
                .step_on(repo, context)
                .await?;

            let existing_member_info = FindMemberInfo {
                user_id: &instr.user_id,
                team_id: &instr.team_id,
            }
            .step_on(repo, context)
            .await?;

            if existing_member_info.is_some() {
                //
                let err_message = trl("error-already-team-member");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    team_id = %instr.team_id,
                    user_id = %token.user_id,
                    affected_user_id = %instr.user_id,
                    roles = ?roles,
                    "expected error: user is already a team member",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: instr.user_id,
                user_nickname: user_info.nickname,
                team_id: instr.team_id,
                roles,
            };

            let member_info = CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(member_info.id)
        })
        .await?;

    accept(CreateMemberVal { id: member_id })
}

/// Joins the current user to a team with a pending invitation code.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, token, instr),
    fields(
        actor_user_id = %token.user_id,
        code = "[REDACTED]",
    )
)]
pub async fn join_team<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    instr: JoinTeamInstr,
) -> BaseRest<MemberInfoView>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: MemberRepo<C>
        + MemberInvitationRepo<C>
        + TeamRepo<C>
        + UserRepo<C>
        + Send
        + Sync,
    O: ObjDeptView<UserAvatar, C> + ObjDeptView<TeamAvatar, C> + Sync,
{
    let current_user_id = token.user_id;

    let member_info = nucl
        .coord(async move |context| {
            //

            let current_user_info = GetUserInfoExcluded {
                id: &current_user_id,
            }
            .step_on(repo, context)
            .await?;

            let member_invitation_info =
                GetMemberInvitationInfoExcluded { code: &instr.code }
                    .step_on(repo, context)
                    .await?;

            LockTeam {
                id: &member_invitation_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            if member_invitation_info.invitee_qid != current_user_info.qid {
                //
                let err_message = trl("error-no-pending-invitation");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    user_id = %current_user_id,
                    invitee_qid = %current_user_info.qid,
                    invitation_invitee_qid = %member_invitation_info.invitee_qid,
                    team_id = %member_invitation_info.team_id,
                    "expected error: invitation does not belong to current user",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            let existing_member_info = FindMemberInfo {
                user_id: &current_user_id,
                team_id: &member_invitation_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            if existing_member_info.is_some() {
                //
                let err_message = trl("error-already-team-member");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    user_id = %current_user_id,
                    team_id = %member_invitation_info.team_id,
                    "expected error: user is already a team member",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: current_user_id,
                user_nickname: current_user_info.nickname,
                team_id: member_invitation_info.team_id.clone(),
                roles: member_invitation_info.roles,
            };

            let member_info = CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            UpdateMemberInvitation::MarkUsed {
                id: &member_invitation_info.id,
            }
            .step_on(repo, context)
            .await?;

            accept(member_info)
        })
        .await?;

    member_info_view(obj_dept, member_info).await
}

/// Lists members under one team.
///
/// The caller must already be a member of the target team.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListMemberInfosInstr,
) -> BaseRest<Vec<MemberInfoView>>
where
    C: Context,
    R: MemberRepo<C> + Sync,
    O: ObjDeptView<UserAvatar, C> + ObjDeptView<TeamAvatar, C> + Sync,
{
    let member_list_spec = instr.try_into()?;

    if let MemberListSpec::Team { team_id, .. } = &member_list_spec {
        //
        let member_info = FindMemberInfo {
            user_id: &token.user_id,
            team_id,
        }
        .run_on(repo)
        .await?;

        let Some(member_info) = member_info else {
            //
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: trl("error-team-member-required"),
            });
        };

        MemberPermComplex::ensure_user_can_list_infos(&member_info)?;
    }

    let member_infos = ListMemberInfos::Spec {
        spec: &member_list_spec,
    }
    .run_on(repo)
    .await?;

    let member_info_vals = member_info_views(obj_dept, member_infos).await?;

    accept(member_info_vals)
}

/// Updates one member's roles.
///
/// The caller must be a team admin of the target member's team.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_roles<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateMemberRolesInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<Serial>,
    R: MemberRepo<C> + Send + Sync,
{
    let () = nucl
        .coord(async move |context| {
            //
            let member_info = GetMemberInfo {
                id: &instr.id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let caller_member_info = FindMemberInfo {
                user_id: &token.user_id,
                team_id: &member_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            let Some(caller_member_info) = caller_member_info else {
                //
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: trl("error-team-admin-required"),
                });
            };

            MemberPermComplex::ensure_user_can_update_info(
                &caller_member_info,
            )?;

            let member_infos = LockTeamMemberInfos {
                team_id: &member_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            if !MemberComplex::team_has_admin_after_role_update(
                &member_infos,
                &member_info,
                instr.roles,
            ) {
                //
                let err_message = trl("error-forbidden");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    team_id = %member_info.team_id,
                    user_id = %token.user_id,
                    affected_user_id = %member_info.user_id,
                    member_id = %member_info.id,
                    roles = ?instr.roles,
                    operation = "remove last team administrator role",
                    "expected error: team administrator retention required",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                });
            }

            let member_role_update = MemberRoleRepl {
                id: member_info.id,
                roles: instr.roles,
            };

            UpdateMember::Role {
                update: &member_role_update,
            }
            .step_on(repo, context)
            .await?;

            accept(())
        })
        .await?;

    accept(())
}

/// Deletes one member.
///
/// The caller must be a team admin of the target member's team.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<Serial>,
    R: MemberRepo<C> + Send + Sync,
{
    let () = nucl
        .coord(async move |context| {
            //
            let member_info = GetMemberInfo {
                id: &id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let caller_member_info = FindMemberInfo {
                user_id: &token.user_id,
                team_id: &member_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            let Some(caller_member_info) = caller_member_info else {
                //
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: trl("error-team-admin-required"),
                });
            };

            MemberPermComplex::ensure_user_can_delete(&caller_member_info)?;

            let member_infos = LockTeamMemberInfos {
                team_id: &member_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            if !MemberComplex::team_has_admin_after_delete(
                &member_infos,
                &member_info,
            ) {
                //
                let err_message = trl("error-forbidden");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    team_id = %member_info.team_id,
                    user_id = %token.user_id,
                    affected_user_id = %member_info.user_id,
                    member_id = %member_info.id,
                    operation = "delete last team administrator member",
                    "expected error: team administrator retention required",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                });
            }

            DeleteMember {
                id: &member_info.id,
            }
            .step_on(repo, context)
            .await?;

            accept(())
        })
        .await?;

    accept(())
}
