//! Assignment role-update orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::{
    AssignmentComplex, AssignmentPermComplex, AssignmentRoleUpdateAccess,
};
use crate::complex::chapter::ChapterComplex;
use crate::data::instr::assignment::UpdateAssignmentRolesInstr;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::{AssignmentEntry, AssignmentRoleRepl};
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, FindAssignmentInfo, ListAssignmentInfosExcluded,
    UpdateAssignmentRoles,
};
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;
use crate::value::role::RoleField;

/// Updates assignment roles.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_roles<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateAssignmentRolesInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: AssignmentRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ChapterRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
{
    ensure_user_can_update_roles::<C, R>(repo, &token, &instr).await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &instr.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ChapterComplex::ensure_chapter_writable(&chapter_info)?;

        let assignment_infos = ListAssignmentInfosExcluded {
            chapter_id: &instr.chapter_id,
        }
        .step_on(repo, context)
        .await?;

        let existing_assignment_info = assignment_infos
            .iter()
            .find(|assignment_info| assignment_info.user_id == instr.user_id);

        match existing_assignment_info {
            //
            Some(assignment_info) => {
                //
                update_existing_assignment(
                    repo,
                    context,
                    &token,
                    &instr,
                    &chapter_info,
                    &assignment_infos,
                    assignment_info,
                )
                .await?;
            }

            None => {
                //
                create_assignment(
                    repo,
                    context,
                    &token,
                    instr,
                    &chapter_info,
                    &assignment_infos,
                )
                .await?;
            }
        }

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

// Persist an existing assignment role change and its workflow record.
async fn update_existing_assignment<C, R>(
    repo: &R,
    context: &mut C,
    token: &UserToken,
    instr: &UpdateAssignmentRolesInstr,
    chapter_info: &ChapterInfo,
    assignment_infos: &[AssignmentInfo],
    assignment_info: &AssignmentInfo,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + ChapterWorkflowRecordRepo<C>,
{
    ensure_existing_update_keeps_admin(
        token,
        instr,
        assignment_infos,
        assignment_info,
    )?;

    if assignment_info.roles == instr.roles {
        return accept(());
    }

    let assignment_role_update = AssignmentRoleRepl {
        id: assignment_info.id.clone(),
        roles: instr.roles,
    };

    UpdateAssignmentRoles {
        update: &assignment_role_update,
    }
    .step_on(repo, context)
    .await?;

    let workflow_record_entry = ChapterWorkflowRecordEntry::new(
        chapter_info.id.clone(),
        Some(token.user_id.clone()),
        ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
            subject_user_id: assignment_info.user_id.clone(),
            previous_roles: assignment_info.roles,
            next_roles: instr.roles,
        },
    );

    CreateChapterWorkflowRecords {
        entries: std::slice::from_ref(&workflow_record_entry),
    }
    .step_on(repo, context)
    .await?;

    accept(())
}

// Create an assignment and record its creation in the chapter workflow.
async fn create_assignment<C, R>(
    repo: &R,
    context: &mut C,
    token: &UserToken,
    instr: UpdateAssignmentRolesInstr,
    chapter_info: &ChapterInfo,
    assignment_infos: &[AssignmentInfo],
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + ChapterWorkflowRecordRepo<C>,
{
    if !AssignmentComplex::chapter_has_admin_after_role_update(
        assignment_infos,
        &instr.user_id,
        instr.roles,
    ) {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %instr.chapter_id,
            user_id = %token.user_id,
            affected_user_id = %instr.user_id,
            roles = ?instr.roles,
            operation = "assign administrator role",
            "expected error: chapter administrator perm required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    let assignment_entry = AssignmentEntry {
        id: AssignmentComplex::gen_id(),
        chapter_id: instr.chapter_id,
        user_id: instr.user_id.clone(),
        roles: instr.roles,
    };

    CreateAssignment {
        entry: &assignment_entry,
    }
    .step_on(repo, context)
    .await?;

    let workflow_record_entry = ChapterWorkflowRecordEntry::new(
        chapter_info.id.clone(),
        Some(token.user_id.clone()),
        ChapterWorkflowRecordPayload::AssignmentCreated {
            subject_user_id: instr.user_id,
            roles: instr.roles,
        },
    );

    CreateChapterWorkflowRecords {
        entries: std::slice::from_ref(&workflow_record_entry),
    }
    .step_on(repo, context)
    .await?;

    accept(())
}

// Preserve an administrator when updating an existing assignment.
fn ensure_existing_update_keeps_admin(
    token: &UserToken,
    instr: &UpdateAssignmentRolesInstr,
    assignment_infos: &[AssignmentInfo],
    assignment_info: &AssignmentInfo,
) -> BaseRest<()> {
    //
    if AssignmentComplex::is_self_admin_role_removal(
        &token.user_id,
        assignment_info,
        instr.roles,
    ) {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %instr.chapter_id,
            user_id = %token.user_id,
            affected_user_id = %instr.user_id,
            roles = ?instr.roles,
            operation = "remove own administrator role",
            "expected error: chapter administrator perm required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    if AssignmentComplex::chapter_has_admin_after_role_update(
        assignment_infos,
        &instr.user_id,
        instr.roles,
    ) {
        return accept(());
    }

    let err_message = trl("error-forbidden");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Perm,
        err_message = %err_message,
        chapter_id = %instr.chapter_id,
        user_id = %token.user_id,
        affected_user_id = %instr.user_id,
        roles = ?instr.roles,
        operation = "remove last chapter administrator role",
        "expected error: chapter administrator perm required",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: err_message,
    })
}

// Select the permission context for an administrator or self-reduction.
fn role_update_access(
    assignment_info: &AssignmentInfo,
) -> AssignmentRoleUpdateAccess<'_> {
    //
    if assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return AssignmentRoleUpdateAccess::Admin { assignment_info };
    }

    AssignmentRoleUpdateAccess::SelfReduce { assignment_info }
}

// Ensure the caller may apply the requested role change.
async fn ensure_user_can_update_roles<C, R>(
    repo: &R,
    token: &UserToken,
    instr: &UpdateAssignmentRolesInstr,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + MemberRepo<C> + TeamRepo<C> + Sync,
{
    let current_assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &instr.chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(current_assignment_info) = current_assignment_info else {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            chapter_id = %instr.chapter_id,
            "expected error: chapter assignment required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    let update_access = role_update_access(&current_assignment_info);

    let subject_member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::<C>::Run,
        &instr.user_id,
        &instr.chapter_id,
    )
    .await?;

    AssignmentPermComplex::ensure_user_can_update_roles(
        &update_access,
        &subject_member_info,
        instr.roles,
    )
}
