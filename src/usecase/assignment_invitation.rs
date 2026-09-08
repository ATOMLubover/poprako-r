//! Assignment invitation use cases.
// Assignment invitation identifier and code generation helpers.
mod code;

/// Joining an assignment through an invitation.
pub mod join;

#[cfg(test)]
mod tests;

use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::data::instr::assignment_invitation::{
    CreateAssignmentInvitationInstr, ListAssignmentInvitationInfosInstr,
};
use crate::data::val::assignment_invitation::CreateAssignmentInvitationVal;
use crate::data::view::assignment_invitation::AssignmentInvitationInfoView;
use crate::model::read::spec::assignment_invitation::AssignmentInvitationListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment_invitation::AssignmentInvitationEntry;
use crate::part::nucl::ReptRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::prom::task::Task;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, ListAssignmentInvitationInfos,
    PurgeExpiredAssignmentInvitation,
};
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::user::FindUserInfo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::assignment_invitation::code::{
    gen_assignment_invitation_id, gen_code,
};
use crate::util::next_snowflake_id;
use crate::value::role::{RoleField, RoleMask};

/// Removes one assignment invitation when it has expired.
#[instrument(level = "info", skip(repo))]
pub async fn purge_expired<C, R>(
    (repo,): (&R,),
    invitation_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentInvitationRepo<C>,
{
    PurgeExpiredAssignmentInvitation { id: invitation_id }
        .run_on(repo)
        .await?;

    accept(())
}

/// Lists assignment invitations under one chapter.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListAssignmentInvitationInfosInstr,
) -> BaseRest<Vec<AssignmentInvitationInfoView>>
where
    C: Context,
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Sync,
{
    ensure_user_admin(repo, &token.user_id, &instr.chapter_id).await?;

    let assignment_invitation_list_spec = AssignmentInvitationListSpec {
        chapter_id: instr.chapter_id,
        is_pending: instr.is_pending,
        offset: instr.offset,
        limit: instr.limit,
    };

    let assignment_invitation_infos = ListAssignmentInvitationInfos {
        spec: &assignment_invitation_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(
        assignment_invitation_infos
            .into_iter()
            .map(AssignmentInvitationInfoView::from)
            .collect(),
    )
}

/// Creates a pending assignment invitation.
#[instrument(level = "info", skip(nucl, repo, prom, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    instr: CreateAssignmentInvitationInstr,
) -> BaseRest<CreateAssignmentInvitationVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + ChapterRepo<C>
        + UserRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    validate_roles(instr.roles, &token.user_id, &instr.chapter_id)?;

    ensure_user_admin(repo, &token.user_id, &instr.chapter_id).await?;

    let (assignment_invitation_id, code) = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &instr.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let invitee_user_info = FindUserInfo {
                qid: &instr.invitee_qid,
            }
            .step_on(repo, context)
            .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                //
                let existing_assignment_info =
                    FindAssignmentInfo::ChapterUser {
                        chapter_id: &instr.chapter_id,
                        user_id: &invitee_user_info.id,
                    }
                    .step_on(repo, context)
                    .await?;

                if existing_assignment_info.is_some() {
                    //
                    let err_message = trl("error-assignment-already-exists");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        chapter_id = %instr.chapter_id,
                        user_id = %token.user_id,
                        invitee_user_id = %invitee_user_info.id,
                        invitee_qid = %instr.invitee_qid,
                        roles = ?instr.roles,
                        "expected error: invitee already has a chapter assignment",
                    );

                    return Err(expected(ExpectedVariant::Args, err_message));
                }
            }

            let (assignment_invitation_id, code) =
                (gen_assignment_invitation_id(), gen_code());

            let assignment_invitation_entry = AssignmentInvitationEntry {
                id: assignment_invitation_id,
                chapter_id: instr.chapter_id,
                inviter_id: token.user_id,
                invitee_qid: instr.invitee_qid,
                code,
                roles: instr.roles,
            };

            let assignment_invitation_info = CreateAssignmentInvitation {
                entry: &assignment_invitation_entry,
            }
            .step_on(repo, context)
            .await?;

            let purge_event = InvitationPayload::Assignment {
                invitation_id: assignment_invitation_info.id.clone(),
            };

            let (purge_payload, purge_task_id) =
                (TaskPayload::Invitation { payload: purge_event }, next_snowflake_id());

            let purge_task = Task {
                id: &purge_task_id,
                payload: &purge_payload,
                delay: Some(EXPIRY_DELAY),
            };

            Defer::new(purge_task).step_on(prom, context).await?;

            accept((
                assignment_invitation_info.id,
                assignment_invitation_info.code,
            ))
        })
        .await?;

    accept(CreateAssignmentInvitationVal {
        id: assignment_invitation_id,
        code,
    })
}

/// Deletes an assignment invitation.
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
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + ChapterRepo<C>
        + Send
        + Sync,
{
    let assignment_invitation_info =
        GetAssignmentInvitationInfo { id: &id }.run_on(repo).await?;

    ensure_user_admin(
        repo,
        &token.user_id,
        &assignment_invitation_info.chapter_id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &assignment_invitation_info.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ChapterComplex::ensure_chapter_writable(&chapter_info)?;

        DeleteAssignmentInvitations::Id { id: &id }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}

// Assignment invitation code expiration window.
const EXPIRY_DELAY: Duration = Duration::from_hours(72);

// Verifies that the current user is assigned as a chapter administrator.
#[instrument(level = "info", skip(repo))]
async fn ensure_user_admin<C, R>(
    repo: &R,
    current_user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: current_user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %current_user_id,
            "expected error: chapter administrator perm required",
        );

        return Err(expected(ExpectedVariant::Perm, err_message));
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %current_user_id,
            roles = ?assignment_info.roles,
            "expected error: chapter administrator perm required",
        );

        return Err(expected(ExpectedVariant::Perm, err_message));
    }

    accept(())
}

// Validates that the roles mask is non-empty and does not contain ADMIN.
fn validate_roles(roles: RoleMask, user: &str, chapter: &str) -> BaseRest<()> {
    //
    if u32::from(roles) == 0 || roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            chapter_id = %chapter,
            user_id = %user,
            roles = ?roles,
            "expected error: chapter roles are not assignable",
        );

        return Err(expected(ExpectedVariant::Args, err_message));
    }

    accept(())
}

// Builds an expected application error with the supplied classification.
const fn expected(variant: ExpectedVariant, message: String) -> BaseError {
    BaseError::Expected { variant, message }
}
