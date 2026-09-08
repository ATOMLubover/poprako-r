//! Process-local online-user lease repository operations.

#[cfg(test)]
// Online-user adapter tests use explicit instants without sleeping.
mod tests;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use poprako_orchestra::Run;

use crate::part::repo::oper::online_user::{ListOnlineUserIds, MarkUserOnline};
use crate::part_impl::repo::HybRepo;
use crate::result::{BaseError, BaseRest, accept};

// One successful mark keeps a user online for ten minutes.
const ONLINE_USER_TTL: Duration = Duration::from_mins(10);

// Refresh one online-user deadline using an explicit clock instant.
fn mark_user_online_at(
    online_user_deadlines: &DashMap<String, HashMap<String, Instant>>,
    team_id: &str,
    user_id: &str,
    now: Instant,
) {
    //
    let Some(expires_at) = now.checked_add(ONLINE_USER_TTL) else {
        return;
    };

    online_user_deadlines
        .entry(team_id.into())
        .or_default()
        .insert(user_id.into(), expires_at);
}

// List one team's active users using an explicit clock instant.
fn list_online_user_ids_at(
    online_user_deadlines: &DashMap<String, HashMap<String, Instant>>,
    team_id: &str,
    now: Instant,
) -> Vec<String> {
    //
    let Entry::Occupied(mut team_entry) =
        online_user_deadlines.entry(team_id.into())
    else {
        return Vec::new();
    };

    let (mut online_user_ids, is_empty) = {
        //
        let online_user_deadlines = team_entry.get_mut();

        online_user_deadlines.retain(|_, expires_at| *expires_at > now);

        (
            online_user_deadlines.keys().cloned().collect::<Vec<_>>(),
            online_user_deadlines.is_empty(),
        )
    };

    if is_empty {
        team_entry.remove();
    }

    online_user_ids.sort();

    online_user_ids
}

impl Run<MarkUserOnline<'_>> for HybRepo {
    // Keep memory adapter failures on the shared repository error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Refresh one user's online lease in the target team.
    async fn run(&self, oper: &MarkUserOnline<'_>) -> BaseRest<()> {
        //
        mark_user_online_at(
            &self.active_ddls,
            oper.team_id,
            oper.user_id,
            Instant::now(),
        );

        accept(())
    }
}

impl Run<ListOnlineUserIds<'_>> for HybRepo {
    // Keep memory adapter failures on the shared repository error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Remove expired leases and return the target team's active user ids.
    async fn run(&self, oper: &ListOnlineUserIds<'_>) -> BaseRest<Vec<String>> {
        //
        let online_user_ids = list_online_user_ids_at(
            &self.active_ddls,
            oper.team_id,
            Instant::now(),
        );

        accept(online_user_ids)
    }
}
