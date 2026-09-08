//! Mock user repository operations.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::UserEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, UpdateUser,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Insert a new user entry into mock state and mirror it into credentials.
fn create_user(state: &mut MockState, entry: &UserEntry) -> BaseRest<UserInfo> {
    //
    // Validate unique id/qid before inserting to avoid inconsistent fixture state.
    if state.users.iter().any(|user| user.id == entry.id)
        || state.users.iter().any(|user| user.qid == entry.qid)
    {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let user_info = UserInfo {
        id: entry.id.clone(),
        qid: entry.qid.clone(),
        nickname: entry.nickname.clone(),
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    };

    state.users.push(user_info.clone());

    state.credentials.push(UserCredential {
        user_id: entry.id.clone(),
        password_hash: entry.password_hash.clone(),
    });

    accept(user_info)
}

// Load one user info from mock state by id.
fn get_user_info(state: &MockState, id: &str) -> BaseRest<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-user-not-found"))
}

// Find one user info by qid and return an optional result.
fn find_user_info(state: &MockState, qid: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.qid == qid)
        .cloned()
}

// Resolve credential for a qid from linked mock tables.
fn get_user_credential(
    state: &MockState,
    qid: &str,
) -> BaseRest<UserCredential> {
    //
    // Resolve user first, then locate corresponding credential row.
    let user_info = state
        .users
        .iter()
        .find(|user_info| user_info.qid == qid)
        .ok_or_else(|| expected("error-user-not-found"))?;

    state
        .credentials
        .iter()
        .find(|credential| credential.user_id == user_info.id)
        .cloned()
        .ok_or_else(|| expected("error-user-not-found"))
}

// Apply a domain update mutation to the in-memory user store.
fn update_user(state: &mut MockState, oper: &UpdateUser<'_>) -> BaseRest<()> {
    //
    // Dispatch variant to a single mutable flow with optional identity/hash branches.
    let id = match oper {
        //
        UpdateUser::Info { repl } => repl.id.as_str(),

        UpdateUser::TouchLastActive { id } => id,

        UpdateUser::PasswordHash { repl } => repl.id.as_str(),
    };

    let update = match oper {
        //
        UpdateUser::Info { repl } => {
            Some((repl.qid.as_str(), repl.nickname.as_str()))
        }

        UpdateUser::TouchLastActive { .. }
        | UpdateUser::PasswordHash { .. } => None,
    };

    let user_info = state
        .users
        .iter_mut()
        .find(|user_info| user_info.id == id)
        .ok_or_else(|| expected("error-user-not-found"))?;

    let updated_at = now();

    if let Some((qid, nickname)) = update {
        //
        user_info.qid = qid.to_string();

        user_info.nickname = nickname.to_string();
    }

    if matches!(oper, UpdateUser::TouchLastActive { .. }) {
        user_info.last_active_at = updated_at;
    }

    user_info.updated_at = updated_at;

    // Internal state field `UpdateUser`.
    // Internal implementation detail.
    if let UpdateUser::PasswordHash { repl } = oper {
        //
        // Mutate matching credential hash for the same user id.
        let credential = state
            .credentials
            .iter_mut()
            .find(|credential| credential.user_id == repl.id)
            .ok_or_else(|| expected("error-user-not-found"))?;

        credential.password_hash = repl.password_hash.clone();
    }

    if matches!(oper, UpdateUser::TouchLastActive { .. }) {
        //
        for member_info in state
            .members
            .iter_mut()
            .filter(|member_info| member_info.user_id == id)
        {
            member_info.user_last_active_at = updated_at;
        }
    }

    accept(())
}

impl<'a> Run<GetUserInfo<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve `GetUserInfo` against the locked mock state.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &GetUserInfo<'a>) -> BaseRest<UserInfo> {
        //
        // Lock state immutably for read-only user info lookup.
        let state = self.state.lock().unwrap();

        get_user_info(&state, oper.id)
    }
}

impl<'a> Run<GetUserCredential<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve credentials from mock state by qid.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetUserCredential<'a>,
    ) -> BaseRest<UserCredential> {
        //
        // Lock state immutably for safe credential read.
        let state = self.state.lock().unwrap();

        get_user_credential(&state, oper.qid)
    }
}

impl<'a> Run<FindUserInfo<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve optional user info by qid in shared state.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &FindUserInfo<'a>) -> BaseRest<Option<UserInfo>> {
        //
        // Lock state immutably for optional find-by-qid.
        let state = self.state.lock().unwrap();

        accept(find_user_info(&state, oper.qid))
    }
}

impl<'a> Run<UpdateUser<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply update user mutations under mutable lock.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateUser<'a>) -> BaseRest<()> {
        //
        // Lock mutable state and reuse shared update helper.
        let mut state = self.state.lock().unwrap();

        update_user(&mut state, oper)
    }
}

impl<'a> Step<CreateUser<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Create user using transaction-local mutable state.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateUser<'a>,
    ) -> BaseRest<UserInfo> {
        create_user(&mut context.state, oper.entry)
    }
}

impl<'a> Step<FindUserInfo<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve optional user by qid from transaction context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindUserInfo<'a>,
    ) -> BaseRest<Option<UserInfo>> {
        accept(find_user_info(&context.state, oper.qid))
    }
}

impl<'a> Step<UpdateUser<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply update operation using context-scoped user state.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateUser<'a>,
    ) -> BaseRest<()> {
        update_user(&mut context.state, oper)
    }
}

impl<'a> Step<GetUserInfoExcluded<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Load by id with context state for exclusive update preparation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetUserInfoExcluded<'a>,
    ) -> BaseRest<UserInfo> {
        get_user_info(&context.state, oper.id)
    }
}

impl<'a> Step<DeleteUser<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Delete user and cleanup dependent mock artifacts.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteUser<'a>,
    ) -> BaseRest<()> {
        //
        // Remove user row and all related linked state for the same user id.
        let position = context
            .state
            .users
            .iter()
            .position(|user_info| user_info.id == oper.id)
            .ok_or_else(|| expected("error-user-not-found"))?;

        context.state.users.remove(position);

        context
            .state
            .credentials
            .retain(|credential| credential.user_id != oper.id);

        context
            .state
            .members
            .retain(|member_info| member_info.user_id != oper.id);

        context
            .state
            .member_invitations
            .retain(|info| info.invitor_id != oper.id);

        context
            .state
            .system_mails
            .retain(|mail| mail.receiver_id != oper.id);

        accept(())
    }
}
