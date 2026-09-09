use super::*;

use crate::part_impl::prom::rdb_impl::actor::base::RdbPromActor;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;

use crate::shared::test_rdb::start;

#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn prom_rdb_impls_use_testcontainer() {
    //
    let test_rdb = start().await;

    let shared = test_rdb.core();

    repo::tests::poll_pending_selects_one_visible_message_per_idle_topic(
        shared.clone(),
    )
    .await;

    repo::tests::retry_message_allows_later_topic_message_to_advance(
        shared.clone(),
    )
    .await;

    repo::tests::wait_message_preserves_retry_budget(shared.clone()).await;

    repo::tests::stale_attempt_finalization_preserves_newer_lease(
        shared.clone(),
    )
    .await;

    repo::tests::completed_message_purge_preserves_non_completed_records(
        shared.clone(),
    )
    .await;

    writer_and_consumer_lifecycles_are_independent(shared).await;

    drop(test_rdb);
}

// writer_and_consumer_lifecycles_are_independent(RdbProm/RdbPromActor)(positive): writing is transactional and consumption begins only after explicit startup.
async fn writer_and_consumer_lifecycles_are_independent(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part::prom::payload::invitation::InvitationPayload;
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::obj_dept::tests::ArtworkTestPool;
    use crate::part_impl::obj_dept::{NormObjDept, RdbObjDeptProm};
    use crate::part_impl::repo::HybRepo;
    use crate::part_impl::repo::mock_impl::Mock;
    use diesel::{
        ExpressionMethods as _, QueryDsl as _, TextExpressionMethods as _,
    };
    use poprako_orchestra::{Nucl as _, OperStep as _};

    let nucl = RdbNucl::new(shared.clone());

    let writer = RdbProm::new();

    let payload = TaskPayload::Invitation {
        payload: InvitationPayload::Member {
            invitation_id: "nonexistent".into(),
        },
    };

    let committed_id = "rdb-test-prom-writer-commit".to_string();

    let rollback_id = "rdb-test-prom-writer-rollback".to_string();

    let committed = Task {
        id: &committed_id,
        payload: &payload,
        delay: None,
    };

    nucl.coord(async |context| {
        Defer::new(committed).step_on(&writer, context).await
    })
    .await
    .unwrap();

    let rolled_back = Task {
        id: &rollback_id,
        payload: &payload,
        delay: None,
    };

    let result = nucl
        .coord(async |context| {
            Defer::new(rolled_back).step_on(&writer, context).await?;

            Err::<(), _>(BaseError::Unrecoverable {
                message: "deliberate rollback".into(),
            })
        })
        .await;

    assert!(result.is_err());

    let mut conn = shared.get().await.unwrap();

    let ids = t_local_message::table
        .filter(t_local_message::f_id.like("rdb-test-prom-writer-%"))
        .select(t_local_message::f_id)
        .load::<String>(&mut conn)
        .await
        .unwrap();

    assert_eq!(ids, [committed_id.clone()]);

    let repo = HybRepo::new(shared.clone());

    let dept = NormObjDept::new(
        shared.clone(),
        ArtworkTestPool,
        RdbObjDeptProm::new(shared.clone()),
    );

    let actor = RdbPromActor::new(
        (nucl.clone(), RdbPromRepo::new()),
        (nucl.clone(), repo.clone(), dept.view(), Mock::new()),
    );

    tokio::task::yield_now().await;

    let status = t_local_message::table
        .filter(t_local_message::f_id.eq(&committed_id))
        .select(t_local_message::f_status)
        .first::<String>(&mut conn)
        .await
        .unwrap();

    assert_eq!(status, "local_message_status:pending");

    let actor = actor.run_detach();

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let status = t_local_message::table
                .filter(t_local_message::f_id.eq(&committed_id))
                .select(t_local_message::f_status)
                .first::<String>(&mut conn)
                .await
                .unwrap();

            if status == "local_message_status:completed" {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    actor.cancel();

    actor.cancel();

    tokio::time::timeout(std::time::Duration::from_secs(10), actor.join())
        .await
        .unwrap()
        .unwrap();

    let scheduler =
        crate::extra::sched::Sched::new(nucl, repo, dept).run_detach();

    scheduler.cancel();

    tokio::time::timeout(std::time::Duration::from_secs(10), scheduler.join())
        .await
        .unwrap()
        .unwrap();
}
