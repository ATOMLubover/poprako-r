//! Multi-operation Unit access evidence loaders.

use poprako_orchestra::{Context, OperRun as _};

use poprako_util::i18n::trl;

use crate::complex::unit::perm::UnitListAccess;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Loaded evidence that grants Unit list and search access.
pub enum UnitListAccessInfo {
    //
    /// Access through team membership.
    Member {
        /// Team membership evidence.
        member_info: Box<MemberInfo>,
    },

    /// Access through chapter assignment.
    Assignee {
        /// Chapter assignment evidence.
        assignment_info: Box<AssignmentInfo>,
    },
}

impl UnitListAccessInfo {
    /// Borrows this loaded evidence for pure permission evaluation.
    pub fn as_access(&self) -> UnitListAccess<'_> {
        //
        match self {
            //
            Self::Member { member_info } => UnitListAccess::Member {
                member_info: member_info.as_ref(),
            },

            Self::Assignee { assignment_info } => UnitListAccess::Assignee {
                assignment_info: assignment_info.as_ref(),
            },
        }
    }
}

/// Loads evidence needed by chapter-scoped Unit reads.
pub struct UnitAccessLoader;

impl UnitAccessLoader {
    /// Loads membership or assignment evidence for one Chapter.
    pub async fn load_access_info_from_chapter<C, R>(
        repo: &R,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseRest<UnitListAccessInfo>
    where
        C: Context,
        R: TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
    {
        let team_id = ResolveTeamId::Chapter { id: chapter_id }
            .run_on(repo)
            .await?;

        let member_info = FindMemberInfo {
            user_id,
            team_id: &team_id,
        }
        .run_on(repo)
        .await?;

        if let Some(member_info) = member_info {
            //
            return accept(UnitListAccessInfo::Member {
                member_info: Box::new(member_info),
            });
        }

        let assignment_info = FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        }
        .run_on(repo)
        .await?;

        let Some(assignment_info) = assignment_info else {
            //
            let err_message = trl("error-unit-list-perm-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Perm,
                err_message = %err_message,
                chapter_id,
                user_id,
                "expected error: unit list permission denied",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: err_message,
            });
        };

        accept(UnitListAccessInfo::Assignee {
            assignment_info: Box::new(assignment_info),
        })
    }
}
