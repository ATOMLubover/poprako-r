//! RDB-backed assignment repository.

// Shared list query builder for assignment read paths.
mod list;

/// Assignment RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::write::assignment::{AssignmentEntry, AssignmentRoleRepl};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::assignment::list::list_infos;
use crate::part_impl::repo::rdb_impl::entity::assignment::{
    AssignmentAspectRow, AssignmentEntryRow, AssignmentInfoRow,
    AssignmentRoleTimestamps,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::{
    f_chapter_id, f_created_at, f_id, f_user_id, t_assignment,
};
use crate::part_impl::repo::rdb_impl::schema::t_chapter::{
    f_comic_id as chapter_comic_id, table as chapter_table,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;
use crate::value::assignment::AssignmentInclOpt;

// Build list query helper functions for assignment read paths.
// Separate module.

// Delete one assignment by id in repository transaction flow.
#[instrument(level = "info", skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_assignment.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Convert a row into assignment info for orchestration return values.
fn row_into_info(row: AssignmentInfoRow) -> BaseRest<AssignmentInfo> {
    row.try_into()
}

// Convert a row list into assignment infos for list operations.
fn rows_into_infos(
    rows: Vec<AssignmentInfoRow>,
) -> BaseRest<Vec<AssignmentInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

// Lookup one assignment by chapter and user id for read operations.
#[instrument(level = "info", skip_all)]
async fn get_info_by_chapter_id_and_user_id(
    conn: &mut RdbConn,
    chapter_id: &str,
    user_id: &str,
) -> BaseRest<Option<AssignmentInfo>> {
    //
    let row = t_assignment
        .filter(f_chapter_id.eq(chapter_id))
        .filter(f_user_id.eq(user_id))
        .select(AssignmentInfoRow::as_select())
        .get_result::<AssignmentInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    row.map(row_into_info).transpose()
}

// Lookup one assignment for user + comic scope and apply include fields.
#[instrument(level = "info", skip_all)]
async fn find_info_by_user_id_and_comic_id(
    conn: &mut RdbConn,
    user_id: &str,
    comic_id: &str,
    incls: &[AssignmentInclOpt],
) -> BaseRest<Option<AssignmentInfo>> {
    //
    let row = t_assignment
        .inner_join(chapter_table)
        .filter(f_user_id.eq(user_id))
        .filter(chapter_comic_id.eq(comic_id))
        .select(AssignmentInfoRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .get_result::<AssignmentInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        return accept(None);
    };

    let mut assignment_info = row_into_info(row)?;

    incl::assignment::populate_assignment_incls(
        conn,
        std::slice::from_mut(&mut assignment_info),
        incls,
    )
    .await?;

    accept(Some(assignment_info))
}

// Query assignment by id and populate optional include fields.
#[instrument(level = "info", skip_all)]
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[AssignmentInclOpt],
) -> BaseRest<AssignmentInfo> {
    //
    let row = t_assignment
        .filter(f_id.eq(id))
        .select(AssignmentInfoRow::as_select())
        .get_result::<AssignmentInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-assignment-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            assignment_id = %id,
            operation = "get assignment info",
            "expected assignment error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    let mut info = row_into_info(row)?;

    incl::assignment::populate_assignment_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(info)
}

// Query chapter assignments with `FOR UPDATE` for transactional mutation windows.
#[instrument(level = "info", skip_all)]
async fn list_chapter_assignments_excluded(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<AssignmentInfo>> {
    //
    let rows = t_assignment
        .filter(f_chapter_id.eq(chapter_id))
        .select(AssignmentInfoRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .for_update()
        .load::<AssignmentInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

// Insert a new assignment row and return created assignment info.
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    model_entry: &AssignmentEntry,
) -> BaseRest<AssignmentInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = AssignmentEntryRow::from_model_entry(model_entry, now);

    let row = diesel::insert_into(t_assignment)
        .values(&entry)
        .returning(AssignmentInfoRow::as_returning())
        .get_result::<AssignmentInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

// Update assignment role timestamps and return latest assignment snapshot.
#[instrument(level = "info", skip_all)]
async fn put_roles(
    conn: &mut RdbConn,
    update: &AssignmentRoleRepl,
) -> BaseRest<AssignmentInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let timestamps = AssignmentRoleTimestamps::from_mask(update.roles, now);

    let aspect = AssignmentAspectRow::new(now).roles(&timestamps);

    let row = diesel::update(t_assignment.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .returning(AssignmentInfoRow::as_returning())
        .get_result::<AssignmentInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-assignment-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            assignment_id = %update.id,
            operation = "update assignment roles",
            "expected assignment error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    row_into_info(row)
}

impl Run<FindAssignmentInfo<'_, '_>> for HybRepo {
    // Keep assignment lookup orchestration errors mapped to repository base errors.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve assignment by chapter/user or user/comic and return optional payload.
    async fn run(
        &self,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> BaseRest<Option<AssignmentInfo>> {
        //
        match oper {
            //
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => submit_query!(
                self.rdb_core,
                get_info_by_chapter_id_and_user_id,
                chapter_id,
                user_id
            ),

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => submit_query!(
                self.rdb_core,
                find_info_by_user_id_and_comic_id,
                user_id,
                comic_id,
                incls
            ),
        }
    }
}

impl Run<ListAssignmentInfos<'_, '_>> for HybRepo {
    // Keep list-assignment orchestration failures normalized for call sites.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Delegate list query composition to shared listing helper with filters.
    async fn run(
        &self,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> BaseRest<Vec<AssignmentInfo>> {
        submit_query!(self.rdb_core, list_infos, oper)
    }
}

impl Run<GetAssignmentInfo<'_, '_>> for HybRepo {
    // Normalize get-assignment errors to base repository error type.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return one assignment info by id with requested include options.
    async fn run(
        &self,
        oper: &GetAssignmentInfo<'_, '_>,
    ) -> BaseRest<AssignmentInfo> {
        submit_query!(self.rdb_core, get_info_by_id, oper.id, oper.incls)
    }
}

impl<L> Step<ListAssignmentInfos<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use base error for listing assignments inside an existing transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve assignment list query by delegating to list module in context transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> BaseRest<Vec<AssignmentInfo>> {
        list_infos(context.conn(), oper).await
    }
}

impl<L> Step<FindAssignmentInfo<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional assignment lookup failures consistent with run-level errors.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve one assignment by chapter/user or user/comic within the open transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> BaseRest<Option<AssignmentInfo>> {
        //
        match oper {
            //
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => {
                //
                get_info_by_chapter_id_and_user_id(
                    context.conn(),
                    chapter_id,
                    user_id,
                )
                .await
            }

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => {
                //
                find_info_by_user_id_and_comic_id(
                    context.conn(),
                    user_id,
                    comic_id,
                    incls,
                )
                .await
            }
        }
    }
}

impl<L> Step<ListAssignmentInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Normalize excluded-list behavior errors under base repository semantics.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // List assignments for a chapter while applying exclusion filters under lock.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListAssignmentInfosExcluded<'_>,
    ) -> BaseRest<Vec<AssignmentInfo>> {
        list_chapter_assignments_excluded(context.conn(), oper.chapter_id).await
    }
}

impl<L> Step<CreateAssignment<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Translate assignment-create failures to base error within transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Insert a new assignment row and return created assignment information.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateAssignment<'_>,
    ) -> BaseRest<AssignmentInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<UpdateAssignmentRoles<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep role-update failures mapped to shared repository error contract.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply role updates to an assignment and return the refreshed record.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateAssignmentRoles<'_>,
    ) -> BaseRest<AssignmentInfo> {
        put_roles(context.conn(), oper.update).await
    }
}

// Delete all assignments bound to one chapter when a chapter is removed.
#[instrument(level = "info", skip_all)]
async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    diesel::delete(t_assignment.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl<L> Step<DeleteAssignments<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Map all delete-assignment branch failures to base repository errors.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Remove assignments by id or by chapter inside an active transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteAssignments<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            DeleteAssignments::Id { id } => delete(context.conn(), id).await,

            DeleteAssignments::Chapter { chapter_id } => {
                delete_by_chapter_id(context.conn(), chapter_id).await
            }
        }
    }
}
