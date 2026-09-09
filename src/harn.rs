//! Storage for the already-composed application parts.
//!
//! This module must contain no business logic, actor handler, background task,
//! or composition factory. It only stores application parts and exposes
//! references to them.

use std::sync::Arc;

use crate::config::AppConfig;

/// Shared storage for application ports composed by the entry point.
pub struct Harn<N, R, O, P, A, D> {
    /// Shared harness storage.
    inner: Arc<HarnInner<N, R, O, P, A, D>>,
}

impl<N, R, O, P, A, D> Clone for Harn<N, R, O, P, A, D> {
    // Clones the shared harness handle.
    fn clone(&self) -> Self {
        //
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// Stores the concrete application ports.
struct HarnInner<N, R, O, P, A, D> {
    //
    // Runtime application configuration.
    config: AppConfig,
    // Transaction coordinator selector.
    nucl: N,
    // Repository adapter bundle.
    repo: R,
    // Reliable remote-object lifecycle adapter.
    obj_dept: O,
    // Deferred-task producer.
    prom: P,
    // Authentication adapter.
    auth: A,
    // Best-effort event producer.
    develop: D,
}

impl<N, R, O, P, A, D> Harn<N, R, O, P, A, D> {
    /// Builds a harness from its application ports.
    pub fn new(
        config: AppConfig,
        (nucl, repo, obj_dept, prom, auth, develop): (N, R, O, P, A, D),
    ) -> Self {
        //
        Self {
            inner: Arc::new(HarnInner {
                config,
                nucl,
                repo,
                obj_dept,
                prom,
                auth,
                develop,
            }),
        }
    }

    /// Returns the runtime application configuration.
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    /// Returns the transaction coordinator selector.
    #[must_use]
    pub fn nucl(&self) -> &N {
        &self.inner.nucl
    }

    /// Returns the repository bundle.
    #[must_use]
    pub fn repo(&self) -> &R {
        &self.inner.repo
    }

    /// Returns the reliable remote-object lifecycle adapter.
    #[must_use]
    pub fn obj_dept(&self) -> &O {
        &self.inner.obj_dept
    }

    /// Returns the deferred-task producer.
    #[must_use]
    pub fn prom(&self) -> &P {
        &self.inner.prom
    }

    /// Returns the token authentication port.
    #[must_use]
    pub fn auth(&self) -> &A {
        &self.inner.auth
    }

    /// Returns the best-effort event producer.
    #[must_use]
    pub fn develop(&self) -> &D {
        &self.inner.develop
    }
}
