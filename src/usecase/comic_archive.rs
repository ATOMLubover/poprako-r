//! Use cases for immutable comic archives.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::{ClearObjs, DeleteObjs};
use poprako_util::i18n::trl;

use crate::complex::comic_archive::{
    ComicArchiveComplex, ComicArchivePermComplex,
};
use crate::data::instr::comic_archive::ExportComicArchivesInstr;
use crate::data::val::comic_archive::{
    ArchiveComicVal, ExportComicArchivesVal,
};
use crate::model::shared::user::UserToken;
use crate::part::nucl::Serial;
use crate::part::obj_dept::{ChapterArtwork, ComicCover, PageImage};
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::comic_archive::ComicArchiveMonth;

// TODO: Generate this complete JSON export through a bounded temporary
// artifact and let the HTTP adapter stream it after database access ends.
// Preserve every selected month and payload; resource pressure must fail the
// export rather than truncate it.
/// Exports selected retained UTC month slots for one team.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn export<C, R>(
    (repo,): (&R,),
    token: UserToken,
    team_id: String,
    instr: ExportComicArchivesInstr,
) -> BaseRest<ExportComicArchivesVal>
where
    C: Context,
    R: ComicArchiveRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            team_id = %team_id,
            user_id = %token.user_id,
            "expected error: comic archive export membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    ComicArchivePermComplex::ensure_user_can_export(&member_info)?;

    let months = ComicArchiveMonth::parse_retained(
        instr.months,
        OffsetDateTime::now_utc(),
    )?;

    let records = ListComicArchivePayloads {
        team_id: &team_id,
        months: &months,
    }
    .run_on(repo)
    .await?;

    let mut payloads_by_month =
        (0..months.len()).map(|_| Vec::new()).collect::<Vec<_>>();

    for (created_at, archived_payload) in records {
        //
        let Some((_, payloads)) = months
            .iter()
            .zip(payloads_by_month.iter_mut())
            .find(|(month, _)| {
                created_at >= month.start && created_at < month.end
            })
        else {
            continue;
        };

        payloads.push(archived_payload);
    }

    let exports = months
        .into_iter()
        .zip(payloads_by_month)
        .map(|(month, payloads)| (month.label, payloads))
        .collect::<BTreeMap<_, _>>();

    accept(ExportComicArchivesVal(exports))
}

/// Archive one active comic, its descendants, and all retained image keys.
#[instrument(level = "info", skip(nucl, repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn archive<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    comic_id: String,
) -> BaseRest<ArchiveComicVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: ComicRepo<C>
        + ComicArchiveRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
    O: ObjDept<ChapterArtwork, C>
        + ObjDept<ComicCover, C>
        + ObjDept<PageImage, C>
        + Send
        + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &comic_id,
    )
    .await?;

    ComicArchivePermComplex::ensure_user_can_archive(&member_info)?;

    let archive_comic_val = nucl
        .coord(async move |context| {
            //
            let comic_archive_snapshot = GetComicArchiveSnapshotExcluded {
                comic_id: &comic_id,
            }
            .step_on(repo, context)
            .await?;

            ComicArchiveComplex::ensure_snapshot_archivable(
                &comic_archive_snapshot,
            )?;

            let archived_at = OffsetDateTime::now_utc();

            let comic_archive_entry = ComicArchiveComplex::prepare_entry(
                comic_archive_snapshot,
                token.user_id,
                archived_at,
            )
            .await?;

            ClearObjs::<ComicCover>::new(std::slice::from_ref(
                &comic_archive_entry.record.source_comic_id,
            ))
            .step_on(obj_dept, context)
            .await
            .map_err(BaseError::from)?;

            DeleteObjs::<PageImage>::new(&comic_archive_entry.source_page_ids)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            DeleteObjs::<ChapterArtwork>::new(
                &comic_archive_entry.source_chapter_ids,
            )
            .step_on(obj_dept, context)
            .await
            .map_err(BaseError::from)?;

            CommitComicArchive {
                entry: &comic_archive_entry,
            }
            .step_on(repo, context)
            .await?;

            let record = comic_archive_entry.record;

            accept(ArchiveComicVal {
                archived_id: record.id,
            })
        })
        .await?;

    accept(archive_comic_val)
}
