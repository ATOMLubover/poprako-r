use poprako_orchestra::Oper;

use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::model::read::spec::member_invitation::MemberInvitationListSpec;
use crate::model::write::member_invitation::{
    MemberInvitationEntry, MemberInvitationRoleRepl,
};
use crate::value::member_invitation::MemberInvitationInclOpt;

/// Creates a new member invitation.
#[derive(Oper)]
#[oper(output = MemberInvitationInfo)]
pub struct CreateMemberInvitation<'a> {
    /// The invitation entry to insert.
    pub entry: &'a MemberInvitationEntry,
}

/// Lists member invitations matching the given spec.
#[derive(Oper)]
#[oper(output = Vec<MemberInvitationInfo>)]
pub struct ListMemberInvitationInfos<'a> {
    /// The filter and pagination specification.
    pub spec: &'a MemberInvitationListSpec,
}

/// Retrieves a single invitation info by ID or code.
#[derive(Oper)]
#[oper(output = MemberInvitationInfo)]
pub enum GetMemberInvitationInfo<'a, 'b> {
    //
    /// Retrieves by invitation ID.
    Id {
        /// The invitation ID.
        id: &'a str,
        /// Which relations to include in the response.
        incls: &'b [MemberInvitationInclOpt],
    },

    /// Retrieves by invitation code.
    Code {
        /// The invitation code.
        code: &'a str,
    },
}

/// Updates a member invitation's fields or marks it as used.
#[derive(Oper)]
#[oper(output = ())]
pub enum UpdateMemberInvitation<'a> {
    //
    /// Updates the invitation fields.
    Info {
        /// The update payload.
        update: &'a MemberInvitationRoleRepl,
    },

    /// Marks the invitation as used.
    MarkUsed {
        /// The invitation ID.
        id: &'a str,
    },
}

/// Retrieves invitation info by code with excluded fields omitted.
#[derive(Oper)]
#[oper(output = MemberInvitationInfo)]
pub struct GetMemberInvitationInfoExcluded<'a> {
    /// The invitation code.
    pub code: &'a str,
}

/// Deletes a member invitation by ID.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteMemberInvitation<'a> {
    /// The invitation ID to delete.
    pub id: &'a str,
}

/// Purges one expired member invitation when it remains pending.
#[derive(Oper)]
#[oper(output = ())]
pub struct PurgeExpiredMemberInvitation<'a> {
    /// The invitation ID to purge.
    pub id: &'a str,
}
