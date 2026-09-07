//! RDB-backed page repository.

// Transaction-scoped page operations.
mod step_impl;

/// Page RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::read::proj::page::{
    PageInfo, PageRawIdentInfo, PageUnitScope,
};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    ApplyPageManifest, DeletePages, GetPageInfo, GetPageInfoExcluded,
    GetPageUnitScope, GetPageUnitScopeExcluded, ListEdittedDiffPageIds,
    ListFirstPageInfos, ListPageInfos, ListPageInfosExcluded,
    ListPageRawIdentInfos, SetPageUnitCountMetrics, ShiftPageIndexesTemporary,
    UpdatePageRawIdents,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::page::{
    PageRawIdentEntryRow, PageRawIdentInfoRow,
};
use crate::part_impl::repo::rdb_impl::page::step_impl::{
    apply_manifest, delete_by_chapter_id, delete_by_ids, get_info_by_id,
    get_info_excluded, get_unit_scope, get_unit_scope_excluded,
    list_editted_diff_page_ids, list_first_infos_by_chapter_ids, list_infos,
    list_infos_excluded, set_unit_counts, shift_indexes_temporary,
};
use crate::part_impl::repo::rdb_impl::schema::t_page_raw_ident;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel as map_diesel;

impl Run<GetPageInfo<'_>> for HybRepo {
    // Use base error for page read orchestration through the query dispatcher.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Fetch one page by id via shared repository dispatch.
    async fn run(&self, oper: &GetPageInfo<'_>) -> BaseRest<PageInfo> {
        submit_query!(self.rdb_core, get_info_by_id, oper.id)
    }
}

impl Run<GetPageUnitScope<'_>> for HybRepo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads the minimal Page scope needed by Unit operations.
    async fn run(
        &self,
        oper: &GetPageUnitScope<'_>,
    ) -> BaseRest<PageUnitScope> {
        submit_query!(self.rdb_core, get_unit_scope, oper.id)
    }
}

impl Run<ListPageInfos<'_>> for HybRepo {
    // Keep list query failures aligned with repository-level base error handling.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // List page infos for a chapter using the chapter id filter.
    async fn run(&self, oper: &ListPageInfos<'_>) -> BaseRest<Vec<PageInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.chapter_id)
    }
}

impl Run<ListFirstPageInfos<'_>> for HybRepo {
    // Return base error for first-page batched read path.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Preload first-page info for each requested chapter id in one query batch.
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        //
        submit_query!(
            self.rdb_core,
            list_first_infos_by_chapter_ids,
            oper.chapter_ids
        )
    }
}

impl Run<ListEdittedDiffPageIds<'_>> for HybRepo {
    // Error type for the Chapter proofread-diff Page query.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    //
    // Lists matching Page IDs in stable Chapter Page order.
    async fn run(
        &self,
        oper: &ListEdittedDiffPageIds<'_>,
    ) -> BaseRest<Vec<String>> {
        //
        submit_query!(
            self.rdb_core,
            list_editted_diff_page_ids,
            oper.chapter_id
        )
    }
}

impl<L> Step<GetPageInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use base error for row-level page reads inside a running transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read one page record in context and convert DB row into `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageInfo<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_by_id(context.conn(), oper.id).await
    }
}

impl<L> Step<GetPageUnitScope<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Declares the transaction isolation level required for this read.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads the minimal Page scope from the active transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageUnitScope<'_>,
    ) -> BaseRest<PageUnitScope> {
        get_unit_scope(context.conn(), oper.id).await
    }
}

impl<L> Step<ListPageInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Reuse base error semantics for chapter page list operations in transactions.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load all pages under a chapter id directly from the transactional connection.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageInfos<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ListPageInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep excluded-list query errors on the shared base error channel.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Read pages for a chapter while applying exclusion rules for deleted rows.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListPageInfosExcluded<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        list_infos_excluded(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<GetPageInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use repository base error for filtered read path with row exclusion.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Load page detail under excluded-read options and return mapped `PageInfo`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageInfoExcluded<'_>,
    ) -> BaseRest<PageInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<GetPageUnitScopeExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Declares the transaction isolation level required for this locked read.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Locks and loads the minimal Page scope used by Unit edits.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetPageUnitScopeExcluded<'_>,
    ) -> BaseRest<PageUnitScope> {
        get_unit_scope_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<SetPageUnitCountMetrics<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep counter update failures consistent for transaction call sites.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Apply counter synchronization payload to page-level aggregates.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &SetPageUnitCountMetrics<'_>,
    ) -> BaseRest<()> {
        set_unit_counts(context.conn(), oper.id, oper.count_metrics).await
    }
}

impl<L> Step<ShiftPageIndexesTemporary<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Maintain base-error parity for temporary page index reordering.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Move current indexes aside before the manifest batch applies final indexes.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ShiftPageIndexesTemporary<'_>,
    ) -> BaseRest<()> {
        shift_indexes_temporary(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<ApplyPageManifest<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Maintain base-error parity for page manifest writes.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply all final page identities and indexes with one typed batch upsert.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ApplyPageManifest<'_>,
    ) -> BaseRest<Vec<PageInfo>> {
        apply_manifest(context.conn(), oper.entries).await
    }
}

impl<L> Step<DeletePages<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep delete error semantics on the shared repository error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Delete pages by chapter or explicit IDs within the active transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeletePages<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            DeletePages::Chapter { chapter_id } => {
                delete_by_chapter_id(context.conn(), chapter_id).await
            }

            DeletePages::Ids { ids } => {
                delete_by_ids(context.conn(), ids).await
            }
        }
    }
}

impl Run<ListPageRawIdentInfos<'_>> for HybRepo {
    // Shared application error type.
    type Error = BaseError;

    // Loads only requested page associations in one query.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListPageRawIdentInfos<'_>,
    ) -> BaseRest<Vec<PageRawIdentInfo>> {
        //
        if oper.page_ids.is_empty() {
            return accept(Vec::new());
        }

        let mut conn = self.rdb_core.get().await?;

        let rows = t_page_raw_ident::table
            .filter(t_page_raw_ident::f_page_id.eq_any(oper.page_ids))
            .select(PageRawIdentInfoRow::as_select())
            .load::<PageRawIdentInfoRow>(&mut *conn)
            .await
            .map_err(map_diesel)?;

        accept(rows.into_iter().map(PageRawIdentInfo::from).collect())
    }
}

impl<L> Step<UpdatePageRawIdents<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Allocation holds the owning chapter and page locks.
    type Level = ReptRead;

    // Shared application error type.
    type Error = BaseError;

    // Replaces optional source filenames in at most two statements.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdatePageRawIdents<'_>,
    ) -> BaseRest<()> {
        //
        let unnamed_ids = oper
            .repl
            .idents
            .iter()
            .filter(|(_, raw_ident)| raw_ident.is_none())
            .map(|(page_id, _)| *page_id)
            .collect::<Vec<_>>();

        let entries = oper
            .repl
            .idents
            .iter()
            .filter_map(|(page_id, raw_ident)| {
                //
                Some(PageRawIdentEntryRow {
                    f_page_id: page_id,
                    f_raw_ident: (*raw_ident)?,
                })
            })
            .collect::<Vec<_>>();

        if !unnamed_ids.is_empty() {
            //
            diesel::delete(
                t_page_raw_ident::table
                    .filter(t_page_raw_ident::f_page_id.eq_any(&unnamed_ids)),
            )
            .execute(context.conn())
            .await
            .map_err(map_diesel)?;
        }

        if !entries.is_empty() {
            //
            diesel::insert_into(t_page_raw_ident::table)
                .values(&entries)
                .on_conflict(t_page_raw_ident::f_page_id)
                .do_update()
                .set((
                    t_page_raw_ident::f_raw_ident
                        .eq(excluded(t_page_raw_ident::f_raw_ident)),
                    t_page_raw_ident::f_updated_at
                        .eq(OffsetDateTime::now_utc()),
                ))
                .execute(context.conn())
                .await
                .map_err(map_diesel)?;
        }

        accept(())
    }
}
