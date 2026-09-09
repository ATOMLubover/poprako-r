// develop_dispatches_user_signup(AsyncEffectDevelop::develop)(positive): signup events should create one system mail for the invitor.
// develop_dispatches_chapter_workflow_completed(AsyncEffectDevelop::develop)(positive): workflow completion should notify next-phase and reviewer assignees.
// develop_dispatches_chapter_published(AsyncEffectDevelop::develop)(positive): chapter publication should notify reviewer assignees.

use super::actor::EffectActor;
use super::*;

use std::sync::Arc;
use time::OffsetDateTime;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::part::effect::EffectEvent as _;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedEvent, ChapterWorkflowCompletedEvent,
};
use crate::part::effect::event::user::UserSignedUpEvent;
use crate::part_impl::repo::mock_impl::Mock;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::Stage;
use crate::value::role::{RoleField, RoleMask};

const BUF_SIZE: NonZeroUsize = match NonZeroUsize::new(8) {
    Some(buf_size) => buf_size,
    None => NonZeroUsize::MIN,
};

// Internal implementation of `team_info`.
// Build a stable test team with non-zero avatar metadata.
fn team_info() -> TeamInfo {
    //
    // Internal implementation detail.
    let time = OffsetDateTime::now_utc();

    TeamInfo {
        id: "team-1".to_string(),
        name: "Team One".to_string(),
        description: "Team description".to_string(),
        created_at: time,
        updated_at: time,
    }
}

// Internal implementation of `workset_info`.
// Build a stable test workset associated with the test team.
fn workset_info() -> WorksetInfo {
    //
    // Internal implementation detail.
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: "workset-1".to_string(),
        team_id: "team-1".to_string(),
        index: 0,
        name: "Workset One".to_string(),
        description: None,
        comic_count: 1,
        created_at: time,
        updated_at: time,
    }
}

// Internal implementation of `comic_info`.
// Build a stable test comic under the test workset and creator.
fn comic_info() -> ComicInfo {
    //
    // Internal implementation detail.
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: "comic-1".to_string(),
        workset_id: "workset-1".to_string(),
        index: 0,
        title: "Comic One".to_string(),
        author: "Author One".to_string(),
        description: None,
        chapter_count: 1,
        creator_id: "creator-user".to_string(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        archived_at: None,
        created_at: time,
        updated_at: time,
    }
}

// Internal implementation of `chapter_info`.
// Build a stable test chapter for async workflow dispatch tests.
fn chapter_info() -> ChapterInfo {
    //
    // Internal implementation detail.
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".to_string(),
        comic_id: "comic-1".to_string(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "Chapter One".to_string(),
        page_count: 1,
        total_unit_count: 1,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0).ok().unwrap(),
        creator_id: "creator-user".to_string(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

// Internal implementation of `seed_chapter_scope`.
// Seed team/workset/comic/chapter fixtures used by chapter tests.
fn seed_chapter_scope(mock: &Mock) {
    //
    // Internal implementation detail.
    mock.seed_team(team_info());

    mock.seed_workset(workset_info());

    mock.seed_comic(comic_info());

    mock.seed_chapter(chapter_info());
}

// Internal implementation of `assignment_info`.
// Build a test assignment record bound to the seeded chapter.
fn assignment_info(id: &str, user_id: &str, roles: RoleMask) -> AssignmentInfo {
    //
    // Internal implementation detail.
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: id.to_string(),
        chapter_id: "chapter-1".to_string(),
        user_id: user_id.to_string(),
        user: None,
        chapter: None,
        roles,
        created_at: time,
        updated_at: time,
    }
}

#[tokio::test]
async fn develop_dispatches_user_signup() {
    //
    // Internal implementation detail.
    let mock = Arc::new(Mock::new());

    mock.seed_team(team_info());

    let (develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor =
        EffectActor::new(mock.as_ref().clone(), effect_recv).run_detach();

    Event::UserSignedUp {
        payload: UserSignedUpEvent {
            team_id: "team-1".to_string(),
            invitor_id: "user-owner".to_string(),
            invitee_qid: "10001".to_string(),
        },
    }
    .develop_on(&develop)
    .await;

    actor.cancel();

    actor.join().await.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.system_mails.len(), 1);

    assert_eq!(snapshot.system_mails[0].receiver_id, "user-owner");
}

#[tokio::test]
async fn develop_dispatches_chapter_workflow_completed() {
    //
    // Internal implementation detail.
    let mock = Arc::new(Mock::new());

    seed_chapter_scope(&mock);

    mock.seed_assignment(assignment_info(
        "assignment-proofreader",
        "proofreader-user",
        RoleMask::from(RoleField::PROOFREADER),
    ));

    mock.seed_assignment(assignment_info(
        "assignment-reviewer",
        "reviewer-user",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let (develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor =
        EffectActor::new(mock.as_ref().clone(), effect_recv).run_detach();

    Event::ChapterWorkflowCompleted {
        payload: ChapterWorkflowCompletedEvent {
            chapter_id: "chapter-1".to_string(),
            completed_stage: Stage::Translate,
        },
    }
    .develop_on(&develop)
    .await;

    actor.cancel();

    actor.join().await.unwrap();

    let snapshot = mock.snapshot();

    let mut receiver_ids = snapshot
        .system_mails
        .iter()
        .map(|system_mail| system_mail.receiver_id.as_str())
        .collect::<Vec<_>>();

    receiver_ids.sort_unstable();

    assert_eq!(receiver_ids, vec!["proofreader-user", "reviewer-user"]);
}

#[tokio::test]
async fn develop_dispatches_chapter_published() {
    //
    // Internal implementation detail.
    let mock = Arc::new(Mock::new());

    seed_chapter_scope(&mock);

    mock.seed_assignment(assignment_info(
        "assignment-reviewer",
        "reviewer-user",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let (develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor =
        EffectActor::new(mock.as_ref().clone(), effect_recv).run_detach();

    Event::ChapterPublished {
        payload: ChapterPublishedEvent {
            chapter_id: "chapter-1".to_string(),
        },
    }
    .develop_on(&develop)
    .await;

    actor.cancel();

    actor.join().await.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.system_mails.len(), 1);

    assert_eq!(snapshot.system_mails[0].receiver_id, "reviewer-user");
}

// cancel_is_idempotent(EffectActorDesc::cancel)(positive): repeated cancellation safely joins the consumer.
#[tokio::test]
async fn cancel_is_idempotent() {
    let (_develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor = EffectActor::new(Mock::new(), effect_recv).run_detach();

    actor.cancel();

    actor.cancel();

    actor.join().await.unwrap();
}

// clone_drop_preserves_consumer(AsyncEffectDevelop::clone)(positive): dropping a producer clone must not cancel event processing.
#[tokio::test]
async fn clone_drop_preserves_consumer() {
    let mock = Mock::new();

    mock.seed_team(team_info());

    let (develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor = EffectActor::new(mock.clone(), effect_recv).run_detach();

    drop(develop.clone());

    Event::UserSignedUp {
        payload: UserSignedUpEvent {
            team_id: "team-1".into(),
            invitor_id: "owner".into(),
            invitee_qid: "10001".into(),
        },
    }
    .develop_on(&develop)
    .await;

    actor.cancel();

    actor.join().await.unwrap();

    assert_eq!(mock.snapshot().system_mails.len(), 1);
}

// construction_defers_processing(EffectActor::new)(positive): queued events stay pending until explicit startup.
#[tokio::test]
async fn construction_defers_processing() {
    let mock = Mock::new();

    mock.seed_team(team_info());

    let (develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor = EffectActor::new(mock.clone(), effect_recv);

    Event::UserSignedUp {
        payload: UserSignedUpEvent {
            team_id: "team-1".into(),
            invitor_id: "owner".into(),
            invitee_qid: "10001".into(),
        },
    }
    .develop_on(&develop)
    .await;

    tokio::task::yield_now().await;

    assert!(mock.snapshot().system_mails.is_empty());

    let actor = actor.run_detach();

    actor.cancel();

    actor.join().await.unwrap();

    assert_eq!(mock.snapshot().system_mails.len(), 1);
}

// descriptor_drop_stops_consumer(EffectActorDesc::drop)(positive): dropping the runtime owner closes the receive side.
#[tokio::test]
async fn descriptor_drop_stops_consumer() {
    let (develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor = EffectActor::new(Mock::new(), effect_recv).run_detach();

    drop(actor);

    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        develop.send.closed(),
    )
    .await
    .unwrap();
}

// join_reports_supervisor_failure(EffectActorDesc::join)(negative): abrupt task termination is reported through the runtime owner.
#[tokio::test]
async fn join_reports_supervisor_failure() {
    let mock = Mock::new();

    mock.seed_team(team_info());

    let (develop, effect_recv) = AsyncEffectDevelop::new(BUF_SIZE);

    let actor = EffectActor::new(mock.clone(), effect_recv).run_detach();

    let _ = std::panic::catch_unwind(|| {
        let _guard = mock.state.lock().unwrap();

        panic!("poison injected repository");
    });

    Event::UserSignedUp {
        payload: UserSignedUpEvent {
            team_id: "team-1".into(),
            invitor_id: "owner".into(),
            invitee_qid: "10001".into(),
        },
    }
    .develop_on(&develop)
    .await;

    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), actor.join())
            .await
            .unwrap()
            .is_err()
    );
}
