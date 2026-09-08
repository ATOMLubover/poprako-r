//! Event delivery dispatcher for the asynchronous effect actor.

use poprako_orchestra::Context;
use tracing::instrument;

use crate::part::effect::event::Event;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::usecase::system_mail;

/// Routes a delivered event to domain-oriented application use cases.
#[instrument(level = "info", skip_all)]
pub async fn dispatch<C, R>(repo: &R, event: Event)
where
    C: Context + Send,
    R: AssignmentRepo<C> + ChapterRepo<C> + TeamRepo<C> + SystemMailRepo + Sync,
{
    match event {
        //
        Event::UserSignedUp { payload } => {
            //
            system_mail::invitation::notify_invitor::<C, R>(
                repo,
                &payload.invitor_id,
                &payload.invitee_qid,
                &payload.team_id,
            )
            .await;
        }

        Event::ChapterPublished { payload } => {
            //
            system_mail::chapter::notify_reviewers_on_publish::<C, R>(
                repo,
                &payload.chapter_id,
            )
            .await;
        }

        Event::ChapterWorkflowCompleted { payload } => {
            //
            system_mail::chapter::notify_next_phase::<C, R>(
                repo,
                &payload.chapter_id,
                payload.completed_stage,
            )
            .await;

            system_mail::chapter::notify_reviewers_on_progress::<C, R>(
                repo,
                &payload.chapter_id,
                payload.completed_stage,
            )
            .await;
        }
    }
}
