use crate::key::ObjKey;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Persisted Check operation discriminator.
pub const CHECK: &str = "obj_prom_oper:check";

/// Persisted Delete operation discriminator.
pub const DELETE: &str = "obj_prom_oper:delete";

/// Builds the deterministic identity for one durable object obligation.
#[must_use]
pub fn obj_task_id(
    topic: &str,
    oper: &str,
    key: &ObjKey,
    gen_no: i64,
) -> String {
    //
    format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}",
        topic.len(),
        topic,
        oper,
        key.id.len(),
        key.id,
        key.ver,
        key.image.len(),
        key.image,
        gen_no,
    )
}

/// Validates a claimed raw task envelope before typed dispatch.
///
/// # Errors
///
/// Returns a message when persisted envelope fields are inconsistent.
pub fn validate_task(task: &ObjDeptPromTask) -> ObjDeptRest<()> {
    //
    let key = task.key()?;

    let is_nonnegative = task.gen_no >= 0 && task.retried_count >= 0;

    let has_valid_lease = task.lease > 0;

    let expected_id = obj_task_id(&task.topic, &task.oper, &key, task.gen_no);

    match (is_nonnegative, has_valid_lease, task.id == expected_id) {
        //
        (true, true, true) => Ok(()),

        _ => Err(ObjDeptError::Invalid {
            message: "invalid object task envelope".into(),
        }),
    }
}

/// One raw durable task owned by an exact lease.
#[derive(Debug, Clone)]
pub struct ObjDeptPromTask {
    //
    /// Stable task identifier.
    pub id: String,

    /// Static object topic.
    pub topic: String,
    /// Persisted operation discriminator.
    pub oper: String,

    /// Persisted object identifier.
    pub obj_id: String,
    /// Persisted object version.
    pub ver: i64,
    /// Persisted complete physical object key.
    pub image: String,

    /// Obligation generation.
    pub gen_no: i64,
    /// Completed retry count.
    pub retried_count: i64,
    /// Exact fencing lease.
    pub lease: i64,
}

impl ObjDeptPromTask {
    /// Decodes the persisted logical key.
    ///
    /// # Errors
    ///
    /// Returns a message when the stored version is invalid.
    pub fn key(&self) -> ObjDeptRest<ObjKey> {
        //
        let Ok(ver) = u32::try_from(self.ver) else {
            //
            return Err(ObjDeptError::Invalid {
                message: "object task ver is outside u32".into(),
            });
        };

        Ok(ObjKey {
            id: self.obj_id.clone(),
            ver,
            image: self.image.clone(),
        })
    }
}

/// Mechanical task action returned by a typed object handler.
pub enum ObjTaskAction {
    //
    /// The obligation completed.
    Complete,

    /// The obligation remains retryable.
    Retry {
        /// Safe retry diagnostic.
        message: String,
    },

    /// Persisted state requires repair.
    Operator {
        /// Safe repair diagnostic.
        message: String,
    },
}

/// Durable work recorded by test adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObjTask {
    //
    /// Checks remote visibility for one object version.
    Check {
        /// Object version to check.
        key: ObjKey,
    },

    /// Deletes one physical object version.
    Delete {
        /// Object version to delete.
        key: ObjKey,
    },
}
