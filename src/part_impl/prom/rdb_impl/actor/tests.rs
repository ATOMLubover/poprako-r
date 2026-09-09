use super::pool::enforce_retry_limit;
use super::task_flow::TaskFlow;

#[test]
fn fourth_failure_becomes_dead() {
    let task_flow = enforce_retry_limit(
        TaskFlow::Retry {
            err_message: "failed".into(),
        },
        3,
    );

    assert!(matches!(task_flow, TaskFlow::Dead { .. }));
}

#[test]
fn first_three_failures_remain_retryable() {
    for retried_count in 0..3 {
        let task_flow = enforce_retry_limit(
            TaskFlow::Retry {
                err_message: "failed".into(),
            },
            retried_count,
        );

        assert!(matches!(task_flow, TaskFlow::Retry { .. }));
    }
}

#[test]
fn waiting_does_not_consume_retry_limit() {
    let task_flow = enforce_retry_limit(
        TaskFlow::Wait {
            err_message: "external state is pending".into(),
        },
        i64::MAX,
    );

    assert!(matches!(task_flow, TaskFlow::Wait { .. }));
}

// dispatch_uses_injected_repo(RdbPromActor::dispatch_payload)(positive): a decoded task mutates the injected mock without accessing queue storage.
#[tokio::test]
async fn dispatch_uses_injected_repo() {
    use crate::model::read::proj::member_invitation::MemberInvitationInfo;
    use crate::part::prom::payload::TaskPayload;
    use crate::part::prom::payload::invitation::InvitationPayload;
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::actor::base::RdbPromActor;
    use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
    use crate::part_impl::repo::mock_impl::Mock;
    use crate::value::role::{RoleField, RoleMask};
    use poprako_rdb_core::RdbCore;

    let core = RdbCore::from_database_url(
        "postgres://unused:unused@127.0.0.1:1/unused",
    )
    .unwrap();

    let mock = Mock::new();

    mock.seed_member_invitation(MemberInvitationInfo {
        id: "invitation".into(),
        team_id: "team".into(),
        invitor: None,
        invitor_id: "owner".into(),
        invitee_qid: "qid".into(),
        code: "code".into(),
        is_pending: true,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    });

    let actor = RdbPromActor::new(
        (RdbNucl::new(core), RdbPromRepo::new()),
        (mock.clone(), mock.clone(), mock.clone(), mock.clone()),
    );

    let payload = TaskPayload::Invitation {
        payload: InvitationPayload::Member {
            invitation_id: "invitation".into(),
        },
    };

    tokio::task::yield_now().await;

    assert_eq!(mock.snapshot().member_invitations.len(), 1);

    let flow = actor
        .dispatch_payload(
            payload.topic(),
            &serde_json::to_value(&payload).unwrap(),
        )
        .await;

    assert!(matches!(flow, TaskFlow::Complete));

    assert!(mock.snapshot().member_invitations.is_empty());
}
