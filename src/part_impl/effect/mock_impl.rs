//! Mock implementation of [EffectDevelop] for testing event collection.

use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserSignedUpEvent;
use crate::part::effect::{Develop, EffectEvent};
use crate::part_impl::repo::mock_impl::Mock;

/// Mock implementation of [`EffectDevelop`].
///
/// Collected events are stored in the mock's internal event buffer and can
/// be drained via [`Mock::drain_events`] for assertion.
impl Develop for Mock {
    // Collect emitted events into the mock event buffer during tests.
    async fn develop(&self, events: Vec<Event>) {
        self.events.lock().unwrap().extend(events);
    }
}

// develop_collects_events(Develop::develop)(positive): emitted events should be stored for later draining.

/// Mock helper that verifies events are collected for later draining.
#[tokio::test]
async fn develop_collects_events() {
    //
    let mock = Mock::new();

    Event::UserSignedUp {
        payload: UserSignedUpEvent {
            team_id: "team-1".into(),
            invitor_id: "user-1".into(),
            invitee_qid: "qid-2".into(),
        },
    }
    .develop_on(&mock)
    .await;

    assert_eq!(mock.event_count(), 1);

    let events = mock.drain_events();

    assert_eq!(events.len(), 1);

    assert_eq!(mock.event_count(), 0);
}
