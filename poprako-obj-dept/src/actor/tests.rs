use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

use super::{ObjDeptActor, POLL_INTERVAL, action_from_err};
use crate::key::ObjKey;
use crate::model::task::{CHECK, ObjDeptPromTask, ObjTaskAction, obj_task_id};
use crate::prom::ObjDeptProm;
use crate::rest::{ObjDeptError, ObjDeptRest};

#[derive(Clone, Default)]
struct MockProm {
    inner: Arc<MockPromInner>,
}

#[derive(Default)]
struct MockPromInner {
    tasks: Mutex<VecDeque<ObjDeptPromTask>>,
    reset_count: AtomicUsize,
    claim_count: AtomicUsize,
    complete_count: AtomicUsize,
}

impl MockProm {
    fn with_task(task: ObjDeptPromTask) -> Self {
        let prom = Self::default();

        prom.inner
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(task);

        prom
    }
}

impl ObjDeptProm for MockProm {
    async fn reset_tasks(&self) -> ObjDeptRest<usize> {
        self.inner.reset_count.fetch_add(1, Ordering::SeqCst);

        Ok(0)
    }

    async fn claim_task(&self) -> ObjDeptRest<Option<ObjDeptPromTask>> {
        self.inner.claim_count.fetch_add(1, Ordering::SeqCst);

        Ok(self
            .inner
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front())
    }

    async fn complete_task(
        &self,
        _task: &ObjDeptPromTask,
    ) -> ObjDeptRest<usize> {
        self.inner.complete_count.fetch_add(1, Ordering::SeqCst);

        Ok(1)
    }

    async fn retry_task<'a>(
        &'a self,
        _task: &'a ObjDeptPromTask,
        _message: &'a str,
    ) -> ObjDeptRest<usize> {
        Ok(1)
    }

    async fn mark_task_operator<'a>(
        &'a self,
        _task: &'a ObjDeptPromTask,
        _message: &'a str,
    ) -> ObjDeptRest<usize> {
        Ok(1)
    }
}

#[test]
fn unavailable_dependency_failure_remains_retryable_for_worker() {
    let action = action_from_err(ObjDeptError::Unavailable {
        message: "connection capacity unavailable".into(),
    });

    assert!(matches!(
        action,
        ObjTaskAction::Retry { message }
            if message == "connection capacity unavailable"
    ));
}

#[tokio::test(start_paused = true)]
async fn actor_runs_both_loops_immediately_then_waits_thirty_seconds() {
    let prom = MockProm::with_task(task());

    let actor = ObjDeptActor::new(prom.clone(), |_task| async {
        Ok(ObjTaskAction::Complete)
    });

    yield_until_idle().await;

    assert_eq!(prom.inner.claim_count.load(Ordering::SeqCst), 0);

    let actor = actor.run_detach();

    yield_until_idle().await;

    assert_eq!(prom.inner.reset_count.load(Ordering::SeqCst), 1);
    assert_eq!(prom.inner.claim_count.load(Ordering::SeqCst), 2);
    assert_eq!(prom.inner.complete_count.load(Ordering::SeqCst), 1);

    tokio::time::advance(std::time::Duration::from_secs(29)).await;

    yield_until_idle().await;

    assert_eq!(prom.inner.reset_count.load(Ordering::SeqCst), 1);
    assert_eq!(prom.inner.claim_count.load(Ordering::SeqCst), 2);

    tokio::time::advance(std::time::Duration::from_secs(1)).await;

    yield_until_idle().await;

    assert_eq!(prom.inner.reset_count.load(Ordering::SeqCst), 2);
    assert_eq!(prom.inner.claim_count.load(Ordering::SeqCst), 3);

    actor.cancel();

    assert!(actor.join().await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn maintenance_keeps_its_cadence_while_claimed_work_is_busy() {
    let prom = MockProm::with_task(task());

    let release = Arc::new(Notify::new());

    let handler_release = release.clone();

    let actor = ObjDeptActor::new(prom.clone(), move |_task| {
        let release = handler_release.clone();

        async move {
            release.notified().await;

            Ok(ObjTaskAction::Complete)
        }
    });

    let actor = actor.run_detach();

    yield_until_idle().await;

    assert_eq!(prom.inner.reset_count.load(Ordering::SeqCst), 1);
    assert_eq!(prom.inner.claim_count.load(Ordering::SeqCst), 1);

    tokio::time::advance(POLL_INTERVAL).await;

    yield_until_idle().await;

    assert_eq!(prom.inner.reset_count.load(Ordering::SeqCst), 2);
    assert_eq!(prom.inner.claim_count.load(Ordering::SeqCst), 1);

    release.notify_one();

    yield_until_idle().await;

    assert_eq!(prom.inner.complete_count.load(Ordering::SeqCst), 1);

    actor.cancel();

    assert!(actor.join().await.is_ok());
}

fn task() -> ObjDeptPromTask {
    let key = ObjKey {
        id: "page-1".into(),
        ver: 1,
        image: "page/page-1-1.png".into(),
    };

    ObjDeptPromTask {
        id: obj_task_id("page_image", CHECK, &key, 0),
        topic: "page_image".into(),
        oper: CHECK.into(),
        obj_id: key.id,
        ver: i64::from(key.ver),
        image: key.image,
        gen_no: 0,
        retried_count: 0,
        lease: 1,
    }
}

async fn yield_until_idle() {
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
}

// join_reports_supervisor_failure(ObjDeptActorDesc::join)(negative): a cancelled supervisor remains observable to its owner.
#[tokio::test]
async fn join_reports_supervisor_failure() {
    let actor = ObjDeptActor::new(MockProm::with_task(task()), |_task| async {
        Ok(ObjTaskAction::Complete)
    })
    .run_detach();

    actor.task.abort();

    assert!(actor.join().await.is_err());
}
