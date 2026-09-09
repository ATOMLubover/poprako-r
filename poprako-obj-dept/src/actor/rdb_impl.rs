//! RDB-backed object-task classification and typed dispatch.

use std::cmp::Ordering;

use crate::rdb_impl::ObjRdbRow;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Maximum duration of one remote request.
pub const REMOTE_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(45);

/// Relative position of a task key against the latest object row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjKeyState {
    //
    /// No row remains.
    Missing,

    /// The task precedes the current version.
    Stale,

    /// The current row retains only its watermark.
    Retired,

    /// The current version is not known to be remotely present.
    Unavailable,

    /// The current version is known to be remotely present.
    Avail,

    /// The task is newer than the persisted watermark.
    Future,
}

/// Returns whether a Check task must reconcile remote presence.
#[must_use]
pub const fn requires_presence_reconciliation(state: ObjKeyState) -> bool {
    matches!(state, ObjKeyState::Unavailable | ObjKeyState::Avail)
}

/// Returns whether a failed presence CAS observed the same active generation.
#[must_use]
pub const fn presence_cas_conflict_requires_retry(state: ObjKeyState) -> bool {
    matches!(state, ObjKeyState::Unavailable | ObjKeyState::Avail)
}

/// Classifies a task version against the latest object row.
///
/// # Errors
///
/// Returns an error when persisted object state is invalid.
pub fn classify(ver: u32, row: Option<&ObjRdbRow>) -> ObjDeptRest<ObjKeyState> {
    //
    let Some(row) = row else {
        return Ok(ObjKeyState::Missing);
    };

    let watermark = u32::try_from(row.ver).map_err(|_| {
        //
        ObjDeptError::Unrecoverable {
            message: "object ver is outside u32".into(),
        }
    })?;

    match ver.cmp(&watermark) {
        //
        Ordering::Less => Ok(ObjKeyState::Stale),

        Ordering::Greater => Ok(ObjKeyState::Future),

        Ordering::Equal => {
            //
            match (&row.key, &row.f_is_uploaded, &row.hash, &row.ext) {
                //
                (None, None, None, None) => Ok(ObjKeyState::Retired),

                (Some(_), Some(false), Some(_), Some(_)) => {
                    Ok(ObjKeyState::Unavailable)
                }

                (Some(_), Some(true), Some(_), Some(_)) => {
                    Ok(ObjKeyState::Avail)
                }

                _ => Err(ObjDeptError::Unrecoverable {
                    message: "invalid object row".into(),
                }),
            }
        }
    }
}

/// Expands one typed RDB object handler.
#[doc(hidden)]
#[macro_export]
// Expands the typed RDB object handler selected by manifest dispatch.
macro_rules! handle_obj_dept_task {
    ($core:expr, $pool:expr, $task:expr, $obj:ty, $obj_mod:ident $(,)?) => {{
        use ::poprako_obj_dept::pool::ObjDeptPool as _;

        let core = $core;
        let pool = $pool;
        let task = $task;
        let key = task.key()?;
        let physical_key = &key.image;

        let (dom, decoded_version) =
            <$obj as ::poprako_obj_dept::key::KeyMap>::reverse(physical_key)?;
        let task_key_is_consistent =
            <$obj as ::poprako_obj_dept::key::KeyMap>::id(&dom) == key.id
                && decoded_version == key.ver
                && <$obj as ::poprako_obj_dept::key::KeyMap>::forward(
                    &dom,
                    decoded_version,
                ) == *physical_key;

        if !task_key_is_consistent {
            return Ok(
                ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                    message: "object task key is inconsistent".into(),
                },
            );
        }

        let mut conn = core
            .get()
            .await
            .map_err(::poprako_obj_dept::rdb_impl::rdb_err)?;
        let (row, initial_revision) = match task.oper.as_str() {
            ::poprako_obj_dept::model::task::CHECK => {
                let presence_state =
                    $obj_mod::load_for_presence_reconciliation(
                        &mut conn,
                        &key.id,
                    )
                    .await?;

                match presence_state {
                    Some((row, revision)) => (Some(row), Some(revision)),
                    None => (None, None),
                }
            }
            _ => ($obj_mod::load(&mut conn, &key.id, false).await?, None),
        };
        let state = ::poprako_obj_dept::actor::rdb_impl::classify(
            key.ver,
            row.as_ref(),
        )?;

        let active_key = ::poprako_obj_dept::rdb_impl::active_key::<$obj>(
            &key.id,
            row.as_ref(),
        )?;

        if active_key
            .as_ref()
            .is_some_and(|active_key| {
                active_key.ver == key.ver
                    && active_key.image != key.image
            })
        {
            return Ok(
                ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                    message: "object task key differs from current generation"
                        .into(),
                },
            );
        }

        drop(conn);

        match (task.oper.as_str(), state) {
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Missing
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Stale
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Retired,
            ) => {
                ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
                    pool.del(&physical_key),
                )
                .await
                .map_err(|_| {
                    ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                        message: "object delete timed out".into(),
                    }
                })??;

                Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete)
            }
            (::poprako_obj_dept::model::task::CHECK, state)
                if ::poprako_obj_dept::actor::rdb_impl::requires_presence_reconciliation(state) =>
            {
                let Some(initial_revision) = initial_revision else {
                    return Ok(
                        ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                            message: "active object lacks a revision token".into(),
                        },
                    );
                };

                // SAFETY: Remote existence is intentionally accepted as upload
                // evidence. Content-hash verification is deferred because it
                // would materially reduce upload throughput.
                let exists = ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
                    pool.has(&physical_key),
                )
                .await
                .map_err(|_| {
                    ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                        message: "object check timed out".into(),
                    }
                })??;
                let mut conn = core
                    .get()
                    .await
                    .map_err(::poprako_obj_dept::rdb_impl::rdb_err)?;
                let updated = match exists {
                    true => {
                        $obj_mod::mark_uploaded_if_revision(
                            &mut conn,
                            &key.id,
                            key.ver,
                            initial_revision,
                        )
                            .await?
                    }
                    false => {
                        $obj_mod::mark_unuploaded_if_revision(
                            &mut conn,
                            &key.id,
                            key.ver,
                            initial_revision,
                        )
                            .await?
                    }
                };

                let changed_state = match updated {
                    0 => {
                        let row = $obj_mod::load(&mut conn, &key.id, false)
                            .await?;

                        Some(::poprako_obj_dept::actor::rdb_impl::classify(
                            key.ver,
                            row.as_ref(),
                        )?)
                    }
                    1 => None,
                    _ => {
                        drop(conn);

                        return Ok(
                            ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                                message: "object check changed multiple rows".into(),
                            },
                        );
                    }
                };

                drop(conn);

                match (updated, changed_state) {
                    (1, _) => {}
                    (
                        0,
                        Some(
                            ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Missing
                            | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Stale
                            | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Retired,
                        ),
                    ) => {
                        ::tokio::time::timeout(
                            ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
                            pool.del(&physical_key),
                        )
                        .await
                        .map_err(|_| {
                            ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                                message: "object delete timed out".into(),
                            }
                        })??;
                    }
                    (0, Some(state))
                        if ::poprako_obj_dept::actor::rdb_impl::presence_cas_conflict_requires_retry(state) =>
                    {
                        return Ok(
                            ::poprako_obj_dept::model::task::ObjTaskAction::Retry {
                                message: "object changed during check".into(),
                            },
                        );
                    }
                    (
                        0,
                        Some(
                            ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Future,
                        ),
                    ) => {
                        return Ok(
                            ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                                message: "object changed to an older watermark".into(),
                            },
                        );
                    }
                    _ => unreachable!("checked object update count and state"),
                }

                Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete)
            }
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Future,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "check task is newer than object state".into(),
            }),
            (
                ::poprako_obj_dept::model::task::DELETE,
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Missing
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Stale
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Retired,
            ) => {
                ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
                    pool.del(&physical_key),
                )
                .await
                .map_err(|_| {
                    ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                        message: "object delete timed out".into(),
                    }
                })??;

                Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete)
            }
            (
                ::poprako_obj_dept::model::task::DELETE,
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Unavailable
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Avail
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Future,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "delete task targets current object".into(),
            }),
            _ => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "unknown object task operation".into(),
            }),
        }
    }};
}
