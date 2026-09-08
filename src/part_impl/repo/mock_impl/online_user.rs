//! Mock online-user repository operations.

use std::time::{Duration, Instant};

use poprako_orchestra::Run;

use crate::part::repo::oper::online_user::{ListOnlineUserIds, MarkUserOnline};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseError, BaseRest, accept};

// Keep mock lease behavior aligned with the production memory adapter.
const ONLINE_USER_TTL: Duration = Duration::from_mins(10);

impl Run<MarkUserOnline<'_>> for Mock {
    // Keep mock failures on the shared repository error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Refresh one user's online lease in the mock repository.
    async fn run(&self, oper: &MarkUserOnline<'_>) -> BaseRest<()> {
        //
        let expires_at = Instant::now() + ONLINE_USER_TTL;

        let mut online_user_deadlines =
            self.online_user_deadlines.lock().unwrap();

        online_user_deadlines
            .entry(oper.team_id.into())
            .or_default()
            .insert(oper.user_id.into(), expires_at);

        accept(())
    }
}

impl Run<ListOnlineUserIds<'_>> for Mock {
    // Keep mock failures on the shared repository error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Remove expired mock leases and return sorted active user ids.
    async fn run(&self, oper: &ListOnlineUserIds<'_>) -> BaseRest<Vec<String>> {
        //
        let now = Instant::now();

        let mut online_user_deadlines =
            self.online_user_deadlines.lock().unwrap();

        let Some(team_deadlines) = online_user_deadlines.get_mut(oper.team_id)
        else {
            return accept(Vec::new());
        };

        team_deadlines.retain(|_, expires_at| *expires_at > now);

        let mut online_user_ids =
            team_deadlines.keys().cloned().collect::<Vec<_>>();

        let is_empty = team_deadlines.is_empty();

        if is_empty {
            online_user_deadlines.remove(oper.team_id);
        }

        online_user_ids.sort();

        accept(online_user_ids)
    }
}
