//! Mock implementation of assignment repository operations.

// Internal organization of the `incl` module.
mod incl;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::spec::assignment::AssignmentListSpec;
use crate::model::write::assignment::AssignmentEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::part_impl::repo::mock_impl::assignment::incl::apply_assignment_incls;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::assignment::AssignmentInclOpt;

// Internal implementation of `list_infos`.
fn list_infos(
    state: &MockState,
    oper: &ListAssignmentInfos<'_, '_>,
) -> Vec<AssignmentInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let (role, incl_opt, page, mut assignment_infos) = match oper {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        ListAssignmentInfos::Spec { spec } => match spec {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            AssignmentListSpec::Chapter {
                chapter_id,
                role,
                incl_opt,
                offset,
                limit,
            } => (
                *role,
                incl_opt.as_slice(),
                Some((*offset as usize, limit.get() as usize)),
                state
                    .assignments
                    .iter()
                    .filter(|assignment_info| {
                        assignment_info.chapter_id == *chapter_id
                    })
                    .cloned()
                    .collect::<Vec<_>>(),
            ),

            AssignmentListSpec::User {
                owner_id,
                role,
                incl_opt,
                offset,
                limit,
            } => (
                *role,
                incl_opt.as_slice(),
                Some((*offset as usize, limit.get() as usize)),
                state
                    .assignments
                    .iter()
                    .filter(|assignment_info| {
                        assignment_info.user_id == *owner_id
                    })
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
        },

        ListAssignmentInfos::Chapter {
            chapter_id,
            role,
            incls,
        } => (
            *role,
            *incls,
            None,
            state
                .assignments
                .iter()
                .filter(|assignment_info| {
                    assignment_info.chapter_id == *chapter_id
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),

        ListAssignmentInfos::Chapters { chapter_ids, incls } => (
            None,
            *incls,
            None,
            state
                .assignments
                .iter()
                .filter(|assignment_info| {
                    chapter_ids.contains(&assignment_info.chapter_id.as_str())
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),
    };

    assignment_infos.retain(|assignment_info| {
        role.is_none_or(|role| assignment_info.roles.has_any_role(&[role]))
    });

    for assignment_info in &mut assignment_infos {
        apply_assignment_incls(state, assignment_info, incl_opt);
    }

    assignment_infos.sort_by(|left, right| {
        //
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    match page {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        Some((offset, limit)) => {
            //
            if offset >= assignment_infos.len() {
                Vec::new()
            } else {
                //
                // Internal implementation detail.
                // Internal implementation detail.
                let end = std::cmp::min(offset + limit, assignment_infos.len());

                assignment_infos[offset..end].to_vec()
            }
        }

        None => assignment_infos,
    }
}

// Internal implementation of `list_infos_excluded`.
fn list_infos_excluded(
    state: &MockState,
    chapter_id: &str,
) -> Vec<AssignmentInfo> {
    //
    list_infos(
        state,
        &ListAssignmentInfos::Chapter {
            chapter_id,
            role: None,
            incls: &[],
        },
    )
}

// Internal implementation of `get_assignment`.
fn get_assignment(
    state: &MockState,
    id: &str,
    incl_opt: &[AssignmentInclOpt],
) -> BaseRest<AssignmentInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut assignment_info = state
        .assignments
        .iter()
        .find(|assignment_info| assignment_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-assignment-not-found"))?;

    apply_assignment_incls(state, &mut assignment_info, incl_opt);

    accept(assignment_info)
}

// Find the most recent assignment for a user within a comic, then load all
// requested include relations.
fn find_assignment_by_user_and_comic(
    state: &MockState,
    user_id: &str,
    comic_id: &str,
    incls: &[AssignmentInclOpt],
) -> Option<AssignmentInfo> {
    //
    let mut assignment_infos = state
        .assignments
        .iter()
        .filter(|assignment_info| assignment_info.user_id == user_id)
        .filter(|assignment_info| {
            //
            state.chapters.iter().any(|chapter_info| {
                //
                chapter_info.id == assignment_info.chapter_id
                    && chapter_info.comic_id == comic_id
            })
        })
        .cloned()
        .collect::<Vec<_>>();

    assignment_infos.sort_by(|left, right| {
        //
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut assignment_info = assignment_infos.into_iter().next()?;

    apply_assignment_incls(state, &mut assignment_info, incls);

    Some(assignment_info)
}

// Internal implementation of `find_assignment`.
fn find_assignment(
    state: &MockState,
    chapter_id: &str,
    user_id: &str,
) -> Option<AssignmentInfo> {
    //
    state
        .assignments
        .iter()
        .find(|assignment_info| {
            //
            assignment_info.chapter_id == chapter_id
                && assignment_info.user_id == user_id
        })
        .cloned()
}

// Internal implementation of `create_assignment`.
fn create_assignment(
    state: &mut MockState,
    entry: &AssignmentEntry,
) -> BaseRest<AssignmentInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state
        .assignments
        .iter()
        .any(|assignment_info| assignment_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    if state.assignments.iter().any(|assignment_info| {
        //
        assignment_info.chapter_id == entry.chapter_id
            && assignment_info.user_id == entry.user_id
    }) {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let assignment_info = AssignmentInfo {
        id: entry.id.clone(),
        chapter_id: entry.chapter_id.clone(),
        user_id: entry.user_id.clone(),
        user: None,
        chapter: None,
        roles: entry.roles,
        created_at: time,
        updated_at: time,
    };

    state.assignments.push(assignment_info.clone());

    accept(assignment_info)
}

// Internal implementation of `delete_assignment_by_id`.
fn delete_assignment_by_id(state: &mut MockState, id: &str) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let index = state
        .assignments
        .iter()
        .position(|assignment_info| assignment_info.id == id)
        .ok_or_else(|| expected("error-assignment-not-found"))?;

    state.assignments.remove(index);

    accept(())
}

// Internal implementation of `delete_assignments_by_chapter_id`.
fn delete_assignments_by_chapter_id(
    state: &mut MockState,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    state.assignments.retain(|a| a.chapter_id != chapter_id);

    accept(())
}

impl Run<FindAssignmentInfo<'_, '_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        let assignment_info = match oper {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => find_assignment(&state, chapter_id, user_id),

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => find_assignment_by_user_and_comic(
                &state, user_id, comic_id, incls,
            ),
        };

        accept(assignment_info)
    }
}

impl Run<ListAssignmentInfos<'_, '_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> Result<Vec<AssignmentInfo>, Self::Error> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        let assignment_infos = list_infos(&state, oper);

        accept(assignment_infos)
    }
}

impl Run<GetAssignmentInfo<'_, '_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &GetAssignmentInfo<'_, '_>,
    ) -> Result<AssignmentInfo, Self::Error> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_assignment(&state, oper.id, oper.incls)
    }
}

impl Step<FindAssignmentInfo<'_, '_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let assignment_info = match oper {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => find_assignment(&context.state, chapter_id, user_id),

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => find_assignment_by_user_and_comic(
                &context.state,
                user_id,
                comic_id,
                incls,
            ),
        };

        accept(assignment_info)
    }
}

impl Step<ListAssignmentInfosExcluded<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListAssignmentInfosExcluded<'_>,
    ) -> Result<Vec<AssignmentInfo>, Self::Error> {
        accept(list_infos_excluded(&context.state, oper.chapter_id))
    }
}

impl Step<ListAssignmentInfos<'_, '_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> Result<Vec<AssignmentInfo>, Self::Error> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let assignment_infos = list_infos(&context.state, oper);

        accept(assignment_infos)
    }
}

impl Step<CreateAssignment<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateAssignment<'_>,
    ) -> Result<AssignmentInfo, Self::Error> {
        create_assignment(&mut context.state, oper.entry)
    }
}

impl Step<UpdateAssignmentRoles<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateAssignmentRoles<'_>,
    ) -> Result<AssignmentInfo, Self::Error> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let assignment_info = context
            .state
            .assignments
            .iter_mut()
            .find(|assignment_info| assignment_info.id == oper.update.id)
            .ok_or_else(|| expected("error-assignment-not-found"))?;

        assignment_info.roles = oper.update.roles;

        assignment_info.updated_at = now();

        accept(assignment_info.clone())
    }
}

impl Step<DeleteAssignments<'_>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteAssignments<'_>,
    ) -> Result<(), Self::Error> {
        //
        match oper {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            DeleteAssignments::Id { id } => {
                delete_assignment_by_id(&mut context.state, id)
            }

            DeleteAssignments::Chapter { chapter_id } => {
                delete_assignments_by_chapter_id(&mut context.state, chapter_id)
            }
        }
    }
}
