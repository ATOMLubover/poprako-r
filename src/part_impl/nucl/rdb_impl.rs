//! Diesel-backed transaction coordinator.

use std::future::Future;
use std::marker::PhantomData;

use diesel::pg::{Pg, PgQueryBuilder};
use diesel::query_builder::QueryBuilder as _;
use diesel_async::{
    AnsiTransactionManager, AsyncPgConnection, TransactionManager as _,
};
use poprako_orchestra::nucl::Error as NuclError;
use poprako_orchestra::{Level, Nucl};
use tracing::instrument;

use poprako_rdb_core::RdbCore;

use crate::part::nucl::{ReptRead, Serial};
use crate::result::BaseError;
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// Selects one of the transaction coordinators owned by the application.
pub struct HybNucl<NR, NS>(NR, NS);

impl<NR, NS> HybNucl<NR, NS> {
    /// Combines the repeatable-read and serializable coordinators.
    pub const fn new(rept_read: NR, serial: NS) -> Self {
        Self(rept_read, serial)
    }

    /// Returns the repeatable-read transaction coordinator.
    pub const fn rept_read(&self) -> &NR {
        &self.0
    }

    /// Returns the serializable transaction coordinator.
    pub const fn serial(&self) -> &NS {
        &self.1
    }
}

// Selects the typed Diesel transaction isolation builder.
trait RdbLevel: Level + Sized {
    /// Begins a transaction at this marker's isolation level.
    fn begin(
        conn: &mut AsyncPgConnection,
    ) -> impl Future<Output = diesel::QueryResult<()>> + Send;
}

impl RdbLevel for ReptRead {
    // Begins a repeatable-read transaction through Diesel's typed builder.
    async fn begin(conn: &mut AsyncPgConnection) -> diesel::QueryResult<()> {
        //
        let begin = {
            //
            let transaction = conn.build_transaction().repeatable_read();

            render_begin(&transaction)?
        };

        AnsiTransactionManager::begin_transaction_sql(conn, &begin).await
    }
}

impl RdbLevel for Serial {
    // Begins a serializable transaction through Diesel's typed builder.
    async fn begin(conn: &mut AsyncPgConnection) -> diesel::QueryResult<()> {
        //
        let begin = {
            //
            let transaction = conn.build_transaction().serializable();

            render_begin(&transaction)?
        };

        AnsiTransactionManager::begin_transaction_sql(conn, &begin).await
    }
}

// Renders Diesel's typed transaction builder for the transaction manager.
fn render_begin<T>(transaction: &T) -> diesel::QueryResult<String>
where
    T: diesel::query_builder::QueryFragment<Pg>,
{
    //
    let mut query_builder = PgQueryBuilder::default();

    transaction.to_sql(&mut query_builder, &Pg)?;

    Ok(query_builder.finish())
}

/// Diesel-backed transaction coordinator that wraps operations in database transactions.
///
/// Each call to [`Nucl::coord`] opens a new connection, begins a transaction,
/// runs the closure, and commits or rolls back on success or failure.
pub struct RdbNucl<L = ReptRead> {
    //
    /// Shared database connection pool used for transactions.
    core: RdbCore,
    /// Isolation-level marker carried by this coordinator.
    level: PhantomData<L>,
}

impl<L> RdbNucl<L> {
    /// Builds a new `RdbNucl` from an [`RdbCore`] connection pool.
    #[must_use]
    pub const fn new(core: RdbCore) -> Self {
        //
        Self {
            core,
            level: PhantomData,
        }
    }
}

impl<L> Clone for RdbNucl<L> {
    // Share the existing database pool without constraining the level marker.
    fn clone(&self) -> Self {
        Self::new(self.core.clone())
    }
}

impl<L> Nucl for RdbNucl<L>
where
    L: RdbLevel + Send + Sync,
{
    // Exposes the coordinator's isolation level.
    type Level = L;

    // Transaction error type propagated through the Nucl trait.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Transaction context wrapping a pooled Diesel connection.
    type Context = RdbContext<L>;

    // Coordinates a closure within a database transaction, committing on success
    // and rolling back on error.
    #[instrument(level = "info", skip_all)]
    async fn coord<F, T, E>(&self, f: F) -> Result<T, NuclError<Self::Error, E>>
    where
        F: AsyncFnOnce(&mut Self::Context) -> Result<T, E> + Send,
        T: Send,
        E: Send,
    {
        let conn = self
            .core
            .get()
            .await
            .map_err(|source| NuclError::Backend(source.into()))?;

        let mut rdb_context = RdbContext::new(conn);

        L::begin(rdb_context.conn())
            .await
            .map_err(|error| NuclError::Backend(diesel(error)))?;

        match f(&mut rdb_context).await {
            //
            Ok(value) => {
                //
                AnsiTransactionManager::commit_transaction(rdb_context.conn())
                    .await
                    .map_err(|error| NuclError::Backend(diesel(error)))?;

                Ok(value)
            }

            Err(error) => {
                //
                AnsiTransactionManager::rollback_transaction(
                    rdb_context.conn(),
                )
                .await
                .map_err(|rollback_error| {
                    NuclError::Backend(diesel(rollback_error))
                })?;

                Err(NuclError::Step(error))
            }
        }
    }
}
