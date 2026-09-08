use poprako_orchestra::Oper;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::model::read::spec::assignment_invitation::AssignmentInvitationListSpec;
use crate::model::write::assignment_invitation::AssignmentInvitationEntry;

/// Creates an assignment invitation.
#[derive(Oper)]
#[oper(output = AssignmentInvitationInfo)]
pub struct CreateAssignmentInvitation<'a> {
    /// The assignment invitation entry data.
    pub entry: &'a AssignmentInvitationEntry,
}

/// Lists assignment invitation infos selected by a query specification.
#[derive(Oper)]
#[oper(output = Vec<AssignmentInvitationInfo>)]
pub struct ListAssignmentInvitationInfos<'a> {
    /// Query specification for filtering invitation infos.
    pub spec: &'a AssignmentInvitationListSpec,
}

/// Gets an assignment invitation that must exist.
#[derive(Oper)]
#[oper(output = AssignmentInvitationInfo)]
pub struct GetAssignmentInvitationInfo<'a> {
    /// Invitation identifier.
    pub id: &'a str,
}

/// Gets an assignment invitation that must exist (with exclusive lock).
#[derive(Oper)]
#[oper(output = AssignmentInvitationInfo)]
pub struct GetAssignmentInvitationInfoExcluded<'a> {
    /// Invitation code.
    pub code: &'a str,
}

/// Marks an assignment invitation as used.
#[derive(Oper)]
#[oper(output = ())]
pub struct MarkAssignmentInvitationUsed<'a> {
    /// Invitation identifier.
    pub id: &'a str,
}

/// Purges one expired assignment invitation when it remains pending.
#[derive(Oper)]
#[oper(output = ())]
pub struct PurgeExpiredAssignmentInvitation<'a> {
    /// Invitation identifier.
    pub id: &'a str,
}

/// Deletes assignment invitations selected by identifier or chapter.
#[derive(Oper)]
#[oper(output = ())]
pub enum DeleteAssignmentInvitations<'a> {
    //
    /// Deletes by invitation identifier.
    Id {
        /// Invitation identifier.
        id: &'a str,
    },

    /// Deletes by chapter.
    Chapter {
        /// Chapter identifier.
        chapter_id: &'a str,
    },
}
