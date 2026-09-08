use poprako_orchestra::Oper;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::{UserCredsRepl, UserEntry, UserInfoRepl};

/// Creates a user.
#[derive(Oper)]
#[oper(output = UserInfo)]
pub struct CreateUser<'a> {
    /// The user entry to insert.
    pub entry: &'a UserEntry,
}

/// Looks up a user by identifier.
#[derive(Oper)]
#[oper(output = UserInfo)]
pub struct GetUserInfo<'a> {
    /// The unique user identifier.
    pub id: &'a str,
}

/// Looks up user credentials by OAuth qid.
#[derive(Oper)]
#[oper(output = UserCredential)]
pub struct GetUserCredential<'a> {
    /// The OAuth qualified identifier.
    pub qid: &'a str,
}

/// Finds a user by OAuth qid, returning `None` if not found.
#[derive(Oper)]
#[oper(output = Option<UserInfo>)]
pub struct FindUserInfo<'a> {
    /// The OAuth qualified identifier.
    pub qid: &'a str,
}

/// Updates a user.
#[derive(Oper)]
#[oper(output = ())]
pub enum UpdateUser<'a> {
    //
    /// Updates user metadata fields.
    Info {
        /// The replacement payload.
        repl: &'a UserInfoRepl,
    },

    /// Touches the last-active timestamp.
    TouchLastActive {
        /// The unique user identifier.
        id: &'a str,
    },

    /// Updates the password hash.
    PasswordHash {
        /// The replacement payload.
        repl: &'a UserCredsRepl,
    },
}

/// Looks up a user by identifier, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = UserInfo)]
pub struct GetUserInfoExcluded<'a> {
    /// The unique user identifier.
    pub id: &'a str,
}

/// Deletes a user.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteUser<'a> {
    /// The user id.
    pub id: &'a str,
}
