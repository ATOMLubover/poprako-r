/// In-memory repository operations used by the hybrid production adapter.
pub mod mem_impl;

/// In-memory repository adapter used by tests.
#[cfg(test)]
pub mod mock_impl;

/// RDBMS-based repository implementation using Diesel and async connections.
pub mod rdb_impl;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use poprako_rdb_core::RdbCore;

/// Hybrid repository handle backed by `PostgreSQL` and process-local memory.
#[derive(Clone)]
pub struct HybRepo {
    //
    /// Shared database connection pool.
    rdb_core: RdbCore,
    /// Team-partitioned online-user lease deadlines.
    active_ddls: Arc<DashMap<String, HashMap<String, Instant>>>,
}

impl HybRepo {
    /// Builds a new hybrid repository from an [`RdbCore`] connection pool.
    #[must_use]
    pub fn new(core: RdbCore) -> Self {
        //
        Self {
            rdb_core: core,
            active_ddls: Arc::default(),
        }
    }
}
