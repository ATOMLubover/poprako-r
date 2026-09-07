//! In-memory tombstone claiming and physical deletion.

use std::collections::HashSet;

use crate::model::read::proj::subtree_delete::SubtreeDeleteSweepTarget;
use crate::part_impl::repo::mock_impl::MockState;
use crate::part_impl::repo::mock_impl::subtree_delete::page_ids;
use crate::value::subtree_delete::SubtreeSweepLevel;

/// Claim eligible tombstones from one explicit hierarchy level.
pub fn claim(
    state: &MockState,
    level: SubtreeSweepLevel,
) -> Option<SubtreeDeleteSweepTarget> {
    //
    let mut ids = match level {
        //
        SubtreeSweepLevel::Chapter => state
            .deleted_chapter_ids
            .iter()
            .filter(|id| {
                state.chapters.iter().any(|info| info.id == id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>(),

        //
        SubtreeSweepLevel::Comic => state
            .deleted_comic_ids
            .iter()
            .filter(|id| {
                //
                state.comics.iter().any(|info| info.id == id.as_str())
                    && !state
                        .chapters
                        .iter()
                        .any(|info| info.comic_id == id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>(),

        //
        SubtreeSweepLevel::Workset => state
            .deleted_workset_ids
            .iter()
            .filter(|id| {
                //
                state.worksets.iter().any(|info| info.id == id.as_str())
                    && !state
                        .comics
                        .iter()
                        .any(|info| info.workset_id == id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>(),

        //
        SubtreeSweepLevel::Team => state
            .deleted_team_ids
            .iter()
            .filter(|id| {
                //
                state.teams.iter().any(|info| info.id == id.as_str())
                    && !state
                        .worksets
                        .iter()
                        .any(|info| info.team_id == id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>(),
    };

    ids.sort_unstable();

    match level {
        //
        SubtreeSweepLevel::Chapter => ids
            .into_iter()
            .next()
            .map(|id| SubtreeDeleteSweepTarget::Chapter { id }),

        //
        SubtreeSweepLevel::Comic => claim_comics(ids),

        //
        SubtreeSweepLevel::Workset => claim_worksets(ids),

        //
        SubtreeSweepLevel::Team => ids
            .into_iter()
            .next()
            .map(|id| SubtreeDeleteSweepTarget::Team { id }),
    }
}

/// Delete one claimed chapter and its objects.
pub fn delete_chapter(state: &mut MockState, chapter_id: &str) {
    //
    let page_ids = page_ids(state, chapter_id);

    state
        .assignment_invitations
        .retain(|info| info.chapter_id != chapter_id);

    state
        .assignments
        .retain(|info| info.chapter_id != chapter_id);

    state
        .chapter_workflow_records
        .retain(|info| info.chapter_id != chapter_id);

    state.units.retain(|info| !page_ids.contains(&info.page_id));

    state.pages.retain(|info| !page_ids.contains(&info.id));

    state
        .page_raw_idents
        .retain(|page_id, _| !page_ids.contains(page_id));

    state.chapters.retain(|info| info.id != chapter_id);

    state.deleted_chapter_ids.remove(chapter_id);
}

/// Delete all comics in one claimed batch.
pub(super) fn delete_comics(state: &mut MockState, comic_ids: &[String]) {
    //
    let comic_ids = comic_ids.iter().cloned().collect::<HashSet<_>>();

    let termbase_ids = state
        .termbases
        .iter()
        .filter(|info| {
            //
            info.comic_id
                .as_ref()
                .is_some_and(|id| comic_ids.contains(id))
        })
        .map(|info| info.id.clone())
        .collect::<HashSet<_>>();

    state
        .terms
        .retain(|info| !termbase_ids.contains(&info.termbase_id));

    state
        .termbases
        .retain(|info| !termbase_ids.contains(&info.id));

    state
        .comic_archives
        .retain(|info| !comic_ids.contains(&info.source_comic_id));

    state.comics.retain(|info| !comic_ids.contains(&info.id));

    state.deleted_comic_ids.retain(|id| !comic_ids.contains(id));
}

/// Delete one claimed team and its direct dependants.
pub(super) fn delete_team(state: &mut MockState, team_id: &str) {
    //
    let termbase_ids = state
        .termbases
        .iter()
        .filter(|info| info.team_id.as_deref() == Some(team_id))
        .map(|info| info.id.clone())
        .collect::<HashSet<_>>();

    state
        .terms
        .retain(|info| !termbase_ids.contains(&info.termbase_id));

    state
        .termbases
        .retain(|info| !termbase_ids.contains(&info.id));

    state.comic_archives.retain(|info| info.team_id != team_id);

    state.announcements.retain(|info| info.team_id != team_id);

    state.comments.retain(|info| info.team_id != team_id);

    state
        .member_invitations
        .retain(|info| info.team_id != team_id);

    state.members.retain(|info| info.team_id != team_id);

    state.teams.retain(|info| info.id != team_id);

    state.deleted_team_ids.remove(team_id);
}

/// Sweep one claimed target.
pub fn sweep_target(state: &mut MockState, target: &SubtreeDeleteSweepTarget) {
    //
    match target {
        //
        SubtreeDeleteSweepTarget::Chapter { id } => delete_chapter(state, id),

        //
        SubtreeDeleteSweepTarget::Comics { ids } => delete_comics(state, ids),

        //
        SubtreeDeleteSweepTarget::Worksets { ids } => {
            //
            let ids = ids.iter().cloned().collect::<HashSet<_>>();

            state.worksets.retain(|info| !ids.contains(&info.id));

            state.deleted_workset_ids.retain(|id| !ids.contains(id));
        }

        //
        SubtreeDeleteSweepTarget::Team { id } => delete_team(state, id),
    }
}

// Restrict comic claims to one bounded batch.
fn claim_comics(mut ids: Vec<String>) -> Option<SubtreeDeleteSweepTarget> {
    //
    ids.truncate(64);

    //
    match ids.is_empty() {
        //
        true => None,

        //
        false => Some(SubtreeDeleteSweepTarget::Comics { ids }),
    }
}

// Restrict workset claims to one bounded batch.
fn claim_worksets(mut ids: Vec<String>) -> Option<SubtreeDeleteSweepTarget> {
    //
    ids.truncate(64);

    //
    match ids.is_empty() {
        //
        true => None,

        //
        false => Some(SubtreeDeleteSweepTarget::Worksets { ids }),
    }
}
