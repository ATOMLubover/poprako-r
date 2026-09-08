//! Joining assignments through pending invitation codes.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::data::instr::assignment_invitation::JoinAssignmentInvitationInstr;
use crate::data::view::assignment::AssignmentInfoView;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{ComicCover, TeamAvatar, UserAvatar};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, FindAssignmentInfo, UpdateAssignmentRoles,
};
use crate::part::repo::oper::assignment_invitation::{
    GetAssignmentInvitationInfoExcluded, MarkAssignmentInvitationUsed,
};
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::user::GetUserInfoExcluded;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::assignment::view::assignment_info_view;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;
use crate::value::role::{RoleField, RoleMask};

/// Joins a chapter assignment with a pending invitation code.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, token, instr),
    fields(
        actor_user_id = %token.user_id,
        code = "[REDACTED]",
    )
)]
pub async fn join<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    instr: JoinAssignmentInvitationInstr,
) -> BaseRest<AssignmentInfoView>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + UserRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + Send
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let current_user_id = token.user_id;

    let assignment_info = nucl
        .coord(async move |context| {
            //
            let current_user_info = GetUserInfoExcluded {
                id: &current_user_id,
            }
            .step_on(repo, context)
            .await?;

            let assignment_invitation_info =
                GetAssignmentInvitationInfoExcluded { code: &instr.code }
                    .step_on(repo, context)
                    .await?;

            ensure_invitation_matches(
                &current_user_info,
                &assignment_invitation_info,
            )?;

            let chapter_info = GetChapterInfoExcluded {
                id: &assignment_invitation_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let comic_info = GetComicInfo {
                id: &chapter_info.comic_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let workset_info = GetWorksetInfo {
                id: &comic_info.workset_id,
            }
            .step_on(repo, context)
            .await?;

            let member_info = FindMemberInfo {
                user_id: &current_user_id,
                team_id: &workset_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            ensure_member_roles(
                &assignment_invitation_info,
                member_info.as_ref(),
                &workset_info.team_id,
                &current_user_id,
            )?;

            let existing_assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &assignment_invitation_info.chapter_id,
                user_id: &current_user_id,
            }
            .step_on(repo, context)
            .await?;

            let (assignment_info, workflow_record_payload) = upsert_assignment(
                repo,
                context,
                existing_assignment_info,
                &assignment_invitation_info,
                &current_user_id,
            )
            .await?;

            MarkAssignmentInvitationUsed {
                id: &assignment_invitation_info.id,
            }
            .step_on(repo, context)
            .await?;

            if let Some(payload) = workflow_record_payload {
                //
                let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                    chapter_info.id,
                    Some(current_user_id),
                    payload,
                );

                CreateChapterWorkflowRecords {
                    entries: std::slice::from_ref(&workflow_record_entry),
                }
                .step_on(repo, context)
                .await?;
            }

            accept(assignment_info)
        })
        .await?;

    assignment_info_view(obj_dept, assignment_info).await
}

// Verifies that an invitation targets the current user and has valid roles.
fn ensure_invitation_matches(
    current_user_info: &UserInfo,
    assignment_invitation_info: &AssignmentInvitationInfo,
) -> BaseRest<()> {
    //
    if assignment_invitation_info.invitee_qid != current_user_info.qid {
        //
        let err_message = trl("error-no-pending-invitation");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            user_id = %current_user_info.id,
            invitee_qid = %current_user_info.qid,
            invitation_invitee_qid = %assignment_invitation_info.invitee_qid,
            invitation_code_present = true,
            "expected error: assignment invitation does not belong to current user",
        );

        return Err(expected(ExpectedVariant::Args, err_message));
    }

    validate_roles(
        assignment_invitation_info.roles,
        &current_user_info.id,
        &assignment_invitation_info.chapter_id,
    )
}

// Verifies that team membership grants every invited chapter role.
fn ensure_member_roles(
    assignment_invitation_info: &AssignmentInvitationInfo,
    member_info: Option<&MemberInfo>,
    team_id: &str,
    current_user_id: &str,
) -> BaseRest<()> {
    //
    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %assignment_invitation_info.chapter_id,
            user_id = %current_user_id,
            team_id = %team_id,
            roles = ?assignment_invitation_info.roles,
            "expected error: invited chapter roles are not assignable",
        );

        return Err(expected(ExpectedVariant::Perm, err_message));
    };

    if member_info
        .roles
        .contains_mask(assignment_invitation_info.roles)
    {
        return accept(());
    }

    let err_message = trl("error-chapter-role-not-assignable");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Perm,
        err_message = %err_message,
        chapter_id = %assignment_invitation_info.chapter_id,
        user_id = %current_user_id,
        team_id = %team_id,
        roles = ?assignment_invitation_info.roles,
        member_roles = ?member_info.roles,
        "expected error: invited chapter roles are not assignable",
    );

    Err(expected(ExpectedVariant::Perm, err_message))
}

// Creates a new assignment or merges the invitation roles into an existing one.
async fn upsert_assignment<C, R>(
    repo: &R,
    context: &mut C,
    existing_assignment_info: Option<AssignmentInfo>,
    assignment_invitation_info: &AssignmentInvitationInfo,
    current_user_id: &str,
) -> BaseRest<(AssignmentInfo, Option<ChapterWorkflowRecordPayload>)>
where
    C: Context,
    R: AssignmentRepo<C>,
{
    if let Some(existing_assignment_info) = existing_assignment_info {
        //
        return merge_existing_assignment(
            repo,
            context,
            existing_assignment_info,
            assignment_invitation_info.roles,
        )
        .await;
    }

    let assignment_entry = AssignmentEntry {
        id: AssignmentComplex::gen_id(),
        chapter_id: assignment_invitation_info.chapter_id.clone(),
        user_id: current_user_id.to_owned(),
        roles: assignment_invitation_info.roles,
    };

    let assignment_info = CreateAssignment {
        entry: &assignment_entry,
    }
    .step_on(repo, context)
    .await?;

    let payload = ChapterWorkflowRecordPayload::AssignmentCreated {
        subject_user_id: assignment_info.user_id.clone(),
        roles: assignment_info.roles,
    };

    accept((assignment_info, Some(payload)))
}

// Builds an expected application error with the supplied classification.
const fn expected(variant: ExpectedVariant, message: String) -> BaseError {
    BaseError::Expected { variant, message }
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

// Merges invited roles into an existing assignment and records any change.
async fn merge_existing_assignment<C, R>(
    repo: &R,
    context: &mut C,
    existing_assignment_info: AssignmentInfo,
    roles: RoleMask,
) -> BaseRest<(AssignmentInfo, Option<ChapterWorkflowRecordPayload>)>
where
    C: Context,
    R: AssignmentRepo<C>,
{
    let assignment_role_update =
        AssignmentComplex::merge_roles(&existing_assignment_info, roles);

    if assignment_role_update.roles == existing_assignment_info.roles {
        return accept((existing_assignment_info, None));
    }

    let assignment_info = UpdateAssignmentRoles {
        update: &assignment_role_update,
    }
    .step_on(repo, context)
    .await?;

    let payload = ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
        subject_user_id: assignment_info.user_id.clone(),
        previous_roles: existing_assignment_info.roles,
        next_roles: assignment_role_update.roles,
    };

    accept((assignment_info, Some(payload)))
}
