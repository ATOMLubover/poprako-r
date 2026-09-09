//! Durable Check/Delete contracts used by ObjDept.

use std::future::Future;

use time::OffsetDateTime;

use crate::key::ObjKey;
use crate::model::task::ObjDeptPromTask;
use crate::rest::ObjDeptRest;

/// Actor-side durable task operations.
pub trait ObjDeptProm {
    /// Reclaims expired work across every object topic.
    fn reset_tasks(&self) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Claims the globally oldest visible task.
    fn claim_task(
        &self,
    ) -> impl Future<Output = ObjDeptRest<Option<ObjDeptPromTask>>> + Send;

    /// Completes one exact fenced task.
    fn complete_task(
        &self,
        task: &ObjDeptPromTask,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Returns one exact fenced task to pending.
    fn retry_task<'a>(
        &'a self,
        task: &'a ObjDeptPromTask,
        message: &'a str,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Marks one exact fenced task for operator repair.
    fn mark_task_operator<'a>(
        &'a self,
        task: &'a ObjDeptPromTask,
        message: &'a str,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;
}

/// Transaction-side durable task creation.
pub trait ObjDeptPromDefer<C> {
    /// Defers one Check task in the caller transaction.
    fn defer_check<'a>(
        &'a self,
        context: &'a mut C,
        topic: &'a str,
        key: &'a ObjKey,
        expires_at: OffsetDateTime,
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;

    /// Defers one Delete task in the caller transaction.
    fn defer_delete<'a>(
        &'a self,
        context: &'a mut C,
        topic: &'a str,
        key: &'a ObjKey,
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;

    /// Defers Check tasks in one caller-transaction batch.
    fn defer_checks<'a>(
        &'a self,
        context: &'a mut C,
        topic: &'a str,
        checks: &'a [ObjDeptPromCheck],
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;

    /// Defers Delete tasks in one caller-transaction batch.
    fn defer_deletes<'a>(
        &'a self,
        context: &'a mut C,
        topic: &'a str,
        keys: &'a [ObjKey],
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;
}

/// One Check task requested by a batch object lifecycle operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjDeptPromCheck {
    //
    /// Logical object generation to verify.
    key: ObjKey,
    /// Time after which absence is treated as a failed upload.
    expires_at: OffsetDateTime,
}

impl ObjDeptPromCheck {
    /// Creates one deferred check for an exact logical object generation.
    #[must_use]
    pub const fn new(key: ObjKey, expires_at: OffsetDateTime) -> Self {
        Self { key, expires_at }
    }

    /// Returns the exact logical object generation to verify.
    #[must_use]
    pub const fn key(&self) -> &ObjKey {
        &self.key
    }

    /// Returns the time after which absence is treated as a failed upload.
    #[must_use]
    pub const fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }
}
