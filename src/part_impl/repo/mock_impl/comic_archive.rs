//! In-memory comic archive repository operations for use-case tests.

use std::collections::HashMap;

use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::read::proj::comic_archive::{
    ComicArchiveChapterSnapshot, ComicArchivePageSnapshot, ComicArchiveSnapshot,
};
use crate::model::read::proj::unit::UnitInfo;
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, expected, unrecoverable,
};
use crate::result::{BaseError, BaseRest, accept};

// Internal implementation of `order_unit_infos`.
fn order_unit_infos(unit_infos: Vec<UnitInfo>) -> BaseRest<Vec<UnitInfo>> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if unit_infos.is_empty() {
        return accept(Vec::new());
    }

    let mut infos_by_id = unit_infos
        .into_iter()
        .map(|unit_info| (unit_info.id.clone(), unit_info))
        .collect::<HashMap<_, _>>();

    let mut predecessor_counts = infos_by_id
        .keys()
        .map(|id| (id.clone(), 0_usize))
        .collect::<HashMap<_, _>>();

    for unit_info in infos_by_id.values() {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let Some(next_id) = unit_info.next_id.as_ref() else {
            continue;
        };

        if next_id == &unit_info.id {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        }

        let Some(predecessor_count) = predecessor_counts.get_mut(next_id)
        else {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        };

        *predecessor_count += 1;

        if *predecessor_count > 1 {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        }
    }

    let head_ids = predecessor_counts
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(id.as_str()))
        .collect::<Vec<_>>();

    let [head_id] = head_ids.as_slice() else {
        return Err(unrecoverable("persisted Unit chain is corrupt"));
    };

    let mut current_id = Some((*head_id).to_string());

    let mut visible_infos = Vec::with_capacity(infos_by_id.len());

    while let Some(id) = current_id {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let Some(unit_info) = infos_by_id.remove(&id) else {
            return Err(unrecoverable("persisted Unit chain is corrupt"));
        };

        current_id = unit_info.next_id.clone();

        if unit_info.hidden_at.is_none() {
            visible_infos.push(unit_info);
        }
    }

    if !infos_by_id.is_empty() {
        return Err(unrecoverable("persisted Unit chain is corrupt"));
    }

    accept(visible_infos)
}

impl Run<ListComicArchivePayloads<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListComicArchivePayloads<'_>,
    ) -> BaseRest<Vec<(OffsetDateTime, String)>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        let payloads = state
            .comic_archives
            .iter()
            .filter(|record| record.team_id == oper.team_id)
            .filter(|record| {
                //
                oper.months.iter().any(|month| {
                    //
                    record.created_at >= month.start
                        && record.created_at < month.end
                })
            })
            .map(|record| (record.created_at, record.archived_payload.clone()))
            .collect();

        accept(payloads)
    }
}

// Assemble and return a comic archive snapshot (including chapter, page, and unit info) for submission.
fn get_snapshot_excluded(
    context: &mut MockContext,
    source_comic_id: &str,
) -> BaseRest<ComicArchiveSnapshot> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let comic_info = context
        .state
        .comics
        .iter()
        .find(|comic_info| {
            //
            comic_info.id == source_comic_id
                && !context.state.deleted_comic_ids.contains(source_comic_id)
        })
        .cloned()
        .ok_or_else(|| expected("error-comic-not-found"))?;

    let workset_info = context
        .state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == comic_info.workset_id)
        .cloned()
        .ok_or_else(|| expected("error-workset-not-found"))?;

    let chapter_snapshots = context
        .state
        .chapters
        .iter()
        .filter(|chapter_info| chapter_info.comic_id == comic_info.id)
        .cloned()
        .map(|chapter_info| {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            let assignment_infos = context
                .state
                .assignments
                .iter()
                .filter(|assignment_info| {
                    assignment_info.chapter_id == chapter_info.id
                })
                .cloned()
                .map(|mut assignment_info| {
                    //
                    // Internal implementation detail.
                    // Internal implementation detail.
                    assignment_info.user = Some(
                        context
                            .state
                            .users
                            .iter()
                            .find(|user_info| {
                                user_info.id == assignment_info.user_id
                            })
                            .cloned()
                            .ok_or_else(|| expected("error-user-not-found"))?,
                    );

                    accept(assignment_info)
                })
                .collect::<BaseRest<Vec<_>>>()?;

            let page_snapshots = context
                .state
                .pages
                .iter()
                .filter(|page_info| page_info.chapter_id == chapter_info.id)
                .cloned()
                .map(|page_info| {
                    //
                    // Internal implementation detail.
                    // Internal implementation detail.
                    let unordered_unit_infos = context
                        .state
                        .units
                        .iter()
                        .filter(|unit_info| unit_info.page_id == page_info.id)
                        .cloned()
                        .collect();

                    let mut unit_infos =
                        order_unit_infos(unordered_unit_infos)?;

                    unit_infos
                        .retain(|unit_info| unit_info.hidden_at.is_none());

                    accept(ComicArchivePageSnapshot {
                        page_info,
                        unit_infos,
                    })
                })
                .collect::<BaseRest<Vec<_>>>()?;

            let mut workflow_record_infos = context
                .state
                .chapter_workflow_records
                .iter()
                .filter(|record_info| record_info.chapter_id == chapter_info.id)
                .cloned()
                .collect::<Vec<_>>();

            workflow_record_infos.sort_by(|left, right| {
                //
                left.created_at
                    .cmp(&right.created_at)
                    .then_with(|| left.id.cmp(&right.id))
            });

            accept(ComicArchiveChapterSnapshot {
                chapter_info,
                assignment_infos,
                workflow_record_infos,
                page_snapshots,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept(ComicArchiveSnapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    })
}

// After persisting the archive entry, retain the comic and remove its active descendants.
fn commit(
    context: &mut MockContext,
    comic_archive_entry: &ComicArchiveEntry,
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if context.archive_commit_failure {
        //
        return Err(unrecoverable(
            "[MockComicArchive::commit] injected archive commit failure",
        ));
    }

    if context
        .state
        .deleted_comic_ids
        .contains(&comic_archive_entry.record.source_comic_id)
    {
        return Err(expected("error-comic-not-found"));
    }

    context
        .state
        .comic_archives
        .push(comic_archive_entry.record.clone());

    context
        .state
        .assignment_invitations
        .retain(|assignment_invitation_info| {
            //
            !comic_archive_entry
                .source_chapter_ids
                .contains(&assignment_invitation_info.chapter_id)
        });

    context.state.assignments.retain(|assignment_info| {
        //
        !comic_archive_entry
            .source_chapter_ids
            .contains(&assignment_info.chapter_id)
    });

    context
        .state
        .chapter_workflow_records
        .retain(|record_info| {
            //
            !comic_archive_entry
                .source_chapter_ids
                .contains(&record_info.chapter_id)
        });

    let termbase_ids = context
        .state
        .termbases
        .iter()
        .filter(|termbase_info| {
            //
            termbase_info.comic_id.as_deref()
                == Some(comic_archive_entry.record.source_comic_id.as_str())
        })
        .map(|termbase_info| termbase_info.id.clone())
        .collect::<Vec<_>>();

    context
        .state
        .terms
        .retain(|term_info| !termbase_ids.contains(&term_info.termbase_id));

    context.state.termbases.retain(|termbase_info| {
        //
        termbase_info.comic_id.as_deref()
            != Some(comic_archive_entry.record.source_comic_id.as_str())
    });

    context.state.units.retain(|unit_info| {
        //
        !comic_archive_entry
            .source_page_ids
            .contains(&unit_info.page_id)
    });

    context.state.pages.retain(|page_info| {
        //
        !comic_archive_entry
            .source_chapter_ids
            .contains(&page_info.chapter_id)
    });

    context.state.page_raw_idents.retain(|page_id, _| {
        !comic_archive_entry.source_page_ids.contains(page_id)
    });

    context.state.chapters.retain(|chapter_info| {
        //
        !comic_archive_entry
            .source_chapter_ids
            .contains(&chapter_info.id)
    });

    let archived_at = comic_archive_entry.record.created_at;

    let comic_info = context
        .state
        .comics
        .iter_mut()
        .find(|comic_info| {
            comic_info.id == comic_archive_entry.record.source_comic_id
        })
        .ok_or_else(|| expected("error-comic-not-found"))?;

    comic_info.archived_at = Some(archived_at);

    comic_info.updated_at = archived_at;

    accept(())
}

impl<'a> Step<GetComicArchiveSnapshotExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetComicArchiveSnapshotExcluded<'a>,
    ) -> BaseRest<ComicArchiveSnapshot> {
        get_snapshot_excluded(context, oper.comic_id)
    }
}

impl<'a> Step<CommitComicArchive<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CommitComicArchive<'a>,
    ) -> BaseRest<()> {
        commit(context, oper.entry)
    }
}
