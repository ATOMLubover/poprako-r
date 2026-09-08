//! RDB-backed member-invitation repository — free query functions and thin trait impls.

/// Member invitation RDB integration tests.
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

use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::model::read::spec::member_invitation::MemberInvitationListSpec;
use crate::model::write::member_invitation::MemberInvitationEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo,
    GetMemberInvitationInfoExcluded, ListMemberInvitationInfos,
    PurgeExpiredMemberInvitation, UpdateMemberInvitation,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::member_invitation::{
    MemberInvitationAspectRow, MemberInvitationEntryRow,
    MemberInvitationInfoRow,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_member_invitation::dsl::{
    f_code, f_created_at, f_id, f_pending, f_team_id, t_member_invitation,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

// ── Free functions ──────────────────────────────────────────────────────────

// Delete a member invitation by ID.
#[instrument(level = "info", skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Delete the raw invitation row by primary key.
    diesel::delete(t_member_invitation.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Query member invitations matching the given list spec, with optional includes.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &MemberInvitationListSpec,
) -> BaseRest<Vec<MemberInvitationInfo>> {
    //
    let mut query = t_member_invitation
        .filter(f_team_id.eq(spec.team_id.as_str()))
        .select(MemberInvitationInfoRow::as_select())
        .into_boxed();

    query = match spec.is_pending {
        //
        Some(is_pending) => query.filter(f_pending.eq(is_pending)),

        None => query,
    };

    let rows = query
        .order_by(f_created_at.desc())
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
        .load::<MemberInvitationInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let mut infos = Vec::with_capacity(rows.len());

    for row in rows {
        infos.push(row.try_into()?);
    }

    incl::member_invitation::populate_member_invitation_incls(
        conn,
        &mut infos,
        &spec.incl_opt,
    )
    .await?;

    accept(infos)
}

// Load a single invitation info by ID with optional includes.
#[instrument(level = "info", skip_all)]
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[MemberInvitationInclOpt],
) -> BaseRest<MemberInvitationInfo> {
    //
    let row = t_member_invitation
        .filter(f_id.eq(id))
        .select(MemberInvitationInfoRow::as_select())
        .get_result::<MemberInvitationInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-invitation-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            invitation_id = %id,
            operation = "get member invitation info",
            "expected member invitation error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    let mut info = row.try_into()?;

    incl::member_invitation::populate_member_invitation_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(info)
}

// Create a new member invitation and return its info.
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &MemberInvitationEntry,
) -> BaseRest<MemberInvitationInfo> {
    //
    let entry = MemberInvitationEntryRow::from(entry);

    let row = diesel::insert_into(t_member_invitation)
        .values(&entry)
        .returning(MemberInvitationInfoRow::as_returning())
        .get_result::<MemberInvitationInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

// Look up a pending invitation by code.
#[instrument(level = "info", skip_all)]
async fn get_info_by_code(
    conn: &mut RdbConn,
    code: &str,
) -> BaseRest<MemberInvitationInfo> {
    //
    let row = t_member_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(MemberInvitationInfoRow::as_select())
        .get_result::<MemberInvitationInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-invitation-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            invitation_code = %code,
            pending = true,
            operation = "get pending member invitation by code",
            "expected member invitation error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    row.try_into()
}

// Look up a pending invitation by code, locking the row for update.
#[instrument(level = "info", skip_all)]
async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> BaseRest<MemberInvitationInfo> {
    //
    let row = t_member_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(MemberInvitationInfoRow::as_select())
        .for_update()
        .get_result::<MemberInvitationInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-invitation-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            invitation_code = %code,
            pending = true,
            operation = "lock pending member invitation by code",
            "expected member invitation error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    row.try_into()
}

// Mark a pending invitation as used.
#[instrument(level = "info", skip_all)]
async fn mark_pending_as_used(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = MemberInvitationAspectRow::new(now).pending(false);

    diesel::update(t_member_invitation.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Update the roles on an existing invitation.
#[instrument(level = "info", skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    roles: RoleMask,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = MemberInvitationAspectRow::new(now)
        .role_mask(i64::from(u32::from(roles)));

    diesel::update(t_member_invitation.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Deletes a member invitation only while it remains pending.
#[instrument(level = "info", skip_all)]
async fn purge_pending(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(
        t_member_invitation
            .filter(f_id.eq(id))
            .filter(f_pending.eq(true)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

impl Run<ListMemberInvitationInfos<'_>> for HybRepo {
    // Map list filters into shared query layer for member invitation collections.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Execute list query through list_infos helper.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListMemberInvitationInfos<'_>,
    ) -> BaseRest<Vec<MemberInvitationInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.spec)
    }
}

impl Run<GetMemberInvitationInfo<'_, '_>> for HybRepo {
    // Resolve invite info by id/code in non-transactional context.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Support direct lookup via id or one-time code resolution.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetMemberInvitationInfo<'_, '_>,
    ) -> BaseRest<MemberInvitationInfo> {
        //
        match oper {
            //
            GetMemberInvitationInfo::Id { id, incls } => {
                submit_query!(self.rdb_core, get_info_by_id, id, incls)
            }

            GetMemberInvitationInfo::Code { code } => {
                submit_query!(self.rdb_core, get_info_by_code, code)
            }
        }
    }
}

impl<L> Step<CreateMemberInvitation<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional create failures in base error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Create invitation record in current transaction and return full info.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateMemberInvitation<'_>,
    ) -> BaseRest<MemberInvitationInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<GetMemberInvitationInfo<'_, '_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional read failures in base error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve invitation by id/code with optional include hydration when needed.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetMemberInvitationInfo<'_, '_>,
    ) -> BaseRest<MemberInvitationInfo> {
        //
        match oper {
            //
            GetMemberInvitationInfo::Id { id, incls } => {
                get_info_by_id(context.conn(), id, incls).await
            }

            GetMemberInvitationInfo::Code { code } => {
                get_info_by_code(context.conn(), code).await
            }
        }
    }
}

impl<L> Step<UpdateMemberInvitation<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional update failures in base error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply either metadata updates or used-state transition by variant.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateMemberInvitation<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateMemberInvitation::Info { update } => {
                //
                update_info(context.conn(), update.id.as_str(), update.roles)
                    .await
            }

            UpdateMemberInvitation::MarkUsed { id } => {
                mark_pending_as_used(context.conn(), id).await
            }
        }
    }
}

impl<L> Step<GetMemberInvitationInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional exclusive-read failures in base error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Fetch invitation by code with lock, used when mutation follows.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetMemberInvitationInfoExcluded<'_>,
    ) -> BaseRest<MemberInvitationInfo> {
        get_info_by_code_excluded(context.conn(), oper.code).await
    }
}

impl<L> Step<DeleteMemberInvitation<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional delete failures in base error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Remove invitation by id as part of current transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteMemberInvitation<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<L> Step<PurgeExpiredMemberInvitation<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Keep transactional purge failures in base error type.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Purge only pending invitations using id filter.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &PurgeExpiredMemberInvitation<'_>,
    ) -> BaseRest<()> {
        purge_pending(context.conn(), oper.id).await
    }
}

impl Run<PurgeExpiredMemberInvitation<'_>> for HybRepo {
    // Keep non-transactional purge failures in base error type.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route expired purge operation to shared query helper.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &PurgeExpiredMemberInvitation<'_>,
    ) -> BaseRest<()> {
        submit_query!(self.rdb_core, purge_pending, oper.id)
    }
}
