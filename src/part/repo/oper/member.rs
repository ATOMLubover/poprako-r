use poprako_orchestra::Oper;

use crate::model::read::proj::member::MemberInfo;
use crate::model::read::spec::member::MemberListSpec;
use crate::model::write::member::{
    MemberEntry, MemberNicknameRepl, MemberRoleRepl,
};
use crate::value::member::MemberInclOpt;

/// Creates a new team member record.
#[derive(Oper)]
#[oper(output = MemberInfo)]
pub struct CreateMember<'a> {
    /// The member entry to insert.
    pub entry: &'a MemberEntry,
}

/// Updates a member's fields (nickname or role).
#[derive(Oper)]
#[oper(output = ())]
pub enum UpdateMember<'a> {
    //
    /// Updates the member's nickname.
    UserNickname {
        /// The nickname replacement payload.
        repl: &'a MemberNicknameRepl,
    },

    /// Updates the member's role.
    Role {
        /// The role update payload.
        update: &'a MemberRoleRepl,
    },
}

/// Lists member infos by query spec or by user ID.
#[derive(Oper)]
#[oper(output = Vec<MemberInfo>)]
pub enum ListMemberInfos<'a> {
    //
    /// Lists members matching the given spec.
    Spec {
        /// The filter and pagination specification.
        spec: &'a MemberListSpec,
    },

    /// Lists all memberships for a user.
    User {
        /// The user ID.
        user_id: &'a str,
    },
}

/// Finds a single member by user and team.
#[derive(Oper)]
#[oper(output = Option<MemberInfo>)]
pub struct FindMemberInfo<'a> {
    //
    /// The user ID.
    pub user_id: &'a str,
    /// The team ID.
    pub team_id: &'a str,
}

/// Retrieves a single member's info by ID with optional includes.
#[derive(Oper)]
#[oper(output = MemberInfo)]
pub struct GetMemberInfo<'a, 'b> {
    //
    /// The member ID.
    pub id: &'a str,
    /// Which relations to include in the response.
    pub incls: &'b [MemberInclOpt],
}

/// Locks and lists every member of one active team.
#[derive(Oper)]
#[oper(output = Vec<MemberInfo>)]
pub struct LockTeamMemberInfos<'a> {
    /// Team whose members must remain locked for the transaction.
    pub team_id: &'a str,
}

/// Deletes a member record by ID.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteMember<'a> {
    /// The member ID to delete.
    pub id: &'a str,
}

/// Deletes every membership owned by one user, including tombstoned teams.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteUserMemberships<'a> {
    /// User whose memberships must be removed.
    pub user_id: &'a str,
}
