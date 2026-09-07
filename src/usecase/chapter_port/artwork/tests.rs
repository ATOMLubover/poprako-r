//! Artwork upload lifecycle, authorization, and transaction tests.

use super::*;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::value::chapter::mask::StageMask;
use crate::value::role::{RoleField, RoleMask};
use time::OffsetDateTime;

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn comic(id: &str) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),

        workset_id: "workset-1".into(),

        index: 0,
        title: "Pop Comic".into(),
        author: "author".into(),
        description: None,
        chapter_count: 1,

        creator_id: "user-1".into(),

        workset: None,
        team: None,
        creator: None,

        last_active_at: time,
        archived_at: None,

        created_at: time,
        updated_at: time,
    }
}

fn workset(id: &str) -> WorksetInfo {
    //
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),

        team_id: "team-1".into(),

        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,

        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),

        comic_id: "comic-1".into(),
        is_pinned: true,

        index: 3,
        subtitle: "Arrival".into(),

        page_count: 2,
        total_unit_count: 2,
        translated_unit_count: 2,
        proofread_unit_count: 1,

        stages: StageMask::try_from(0u32).ok().unwrap(),

        creator_id: "user-1".into(),

        comic: None,
        creator: None,

        created_at: time,
        updated_at: time,
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),

        chapter_id: chapter_id.into(),

        user_id: user_id.into(),

        user: None,
        chapter: None,

        roles: role_mask,

        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str) -> MemberInfo {
    //
    MemberInfo {
        id: format!("member-{user_id}"),

        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),

        team_id: "team-1".into(),

        user: None,
        team: None,

        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn seed(role: RoleField) -> Mock {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1"));

    mock.seed_comic(comic("comic-1"));

    mock.seed_chapter(chapter("chapter-1"));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(role),
    ));

    mock
}

async fn allocate(mock: &Mock, hash: u8) -> BaseRest<AllocChapterArtworkVal> {
    let instr = AllocChapterArtworkInstr {
        artwork_hash: ArtworkHash::new([hash; 32]),
        new_byte_len: 1024,
        ext: "zip".into(),
    };

    alloc_artwork(
        (mock, mock, mock, &ArtworkConfig::default()),
        token("user-1"),
        "chapter-1".into(),
        instr,
    )
    .await
}

async fn mark(mock: &Mock, version: u32) -> BaseRest<()> {
    let instr = MarkChapterArtworkUploadedInstr {
        artwork_ver: version,
    };

    mark_artwork_uploaded(
        (mock, mock, mock, mock),
        token("user-1"),
        "chapter-1".into(),
        instr,
    )
    .await
}

async fn export(mock: &Mock, user: &str) -> BaseRest<ExportChapterArtworkVal> {
    export_artwork::<_, MockContext, _, _>(
        (mock, mock, mock),
        token(user),
        "chapter-1".into(),
    )
    .await
}

// artwork_completes_once(mark_artwork_uploaded)(positive): confirmation is atomic and idempotent from pending and active phases.
#[tokio::test]
async fn artwork_completes_once_from_pending_and_active() {
    for phase in [StagePhase::Pending, StagePhase::Active] {
        let mock = seed(RoleField::TYPESETTER);

        mock.state.lock().unwrap().chapters[0].stages =
            StageMask::try_from(0u32)
                .unwrap()
                .try_set_phase(Stage::TypesetRedraw, phase)
                .unwrap();

        let allocation = allocate(&mock, 1).await.unwrap();

        assert!(allocation.slot.is_some());

        assert!(export(&mock, "user-1").await.is_err());

        mark(&mock, allocation.artwork_ver).await.unwrap();

        mark(&mock, allocation.artwork_ver).await.unwrap();

        let snapshot = mock.snapshot();

        assert_eq!(
            snapshot.chapters[0].stages.get_phase(Stage::TypesetRedraw),
            StagePhase::Completed
        );

        assert_eq!(snapshot.chapter_workflow_records.len(), 1);

        assert!(
            matches!(snapshot.chapter_workflow_records[0].payload, ChapterWorkflowRecordPayload::StageTransitioned { origin: ChapterWorkflowRecordOrigin::ArtworkUpload, previous_phase, next_phase: StagePhase::Completed, .. } if previous_phase == phase)
        );

        assert_eq!(mock.event_count(), 1);

        let exported = export(&mock, "user-1").await.unwrap();

        assert_eq!(exported.artwork_ver, allocation.artwork_ver);

        assert_eq!(exported.artwork_hash.as_bytes(), &[1; 32]);

        assert!(exported.download_url.contains("chapter_artwork/chapter-1-"));

        assert!(!exported.download_url.contains("thumbnail"));

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.chapter_workflow_records.len(), 2);

        assert!(matches!(
            snapshot.chapter_workflow_records[1].payload,
            ChapterWorkflowRecordPayload::ArtworkExported {
                artwork_ver,
            } if artwork_ver == allocation.artwork_ver
        ));

        let duplicate = allocate(&mock, 1).await.unwrap();

        assert!(duplicate.slot.is_none());

        assert_eq!(duplicate.artwork_ver, allocation.artwork_ver);
    }
}

// artwork_replacement(alloc_artwork)(negative): replacing content retires the old version without reverting completed work.
#[tokio::test]
async fn artwork_replacement_rejects_stale_confirmation() {
    let mock = seed(RoleField::REDRAWER);

    let first = allocate(&mock, 1).await.unwrap();

    mark(&mock, first.artwork_ver).await.unwrap();

    let second = allocate(&mock, 2).await.unwrap();

    assert!(second.artwork_ver > first.artwork_ver);

    assert!(mark(&mock, first.artwork_ver).await.is_err());

    assert!(export(&mock, "user-1").await.is_err());

    assert_eq!(
        mock.snapshot().chapters[0]
            .stages
            .get_phase(Stage::TypesetRedraw),
        StagePhase::Completed
    );

    mark(&mock, second.artwork_ver).await.unwrap();

    assert_eq!(mock.event_count(), 1);

    assert_eq!(
        export(&mock, "user-1")
            .await
            .unwrap()
            .artwork_hash
            .as_bytes(),
        &[2; 32]
    );

    mock.state
        .lock()
        .unwrap()
        .objs
        .get_mut("chapter_artwork")
        .unwrap()
        .get_mut("chapter-1")
        .unwrap()
        .meta
        .as_mut()
        .unwrap()
        .is_avail = false;

    assert!(export(&mock, "user-1").await.is_err());

    assert_eq!(
        mock.snapshot().chapters[0]
            .stages
            .get_phase(Stage::TypesetRedraw),
        StagePhase::Completed
    );
}

// artwork_permissions(alloc_artwork)(negative): only designated chapter roles may submit; team members may export.
#[tokio::test]
async fn artwork_permissions_match_submission_and_export_roles() {
    for role in [RoleField::TYPESETTER, RoleField::REDRAWER, RoleField::ADMIN] {
        let mock = seed(role);

        let allocation = allocate(&mock, 1).await.unwrap();

        mark(&mock, allocation.artwork_ver).await.unwrap();

        mock.seed_member(member("member"));

        assert!(export(&mock, "member").await.is_ok());

        assert!(export(&mock, "outsider").await.is_err());
    }

    for role in [
        RoleField::TRANSLATOR,
        RoleField::PROOFREADER,
        RoleField::REVIEWER,
        RoleField::PUBLISHER,
    ] {
        let mock = seed(role);

        assert!(allocate(&mock, 1).await.is_err());

        assert!(mark(&mock, 1).await.is_err());

        assert!(mock.snapshot().objs.is_empty());
    }
}

// artwork_atomicity(mark_artwork_uploaded)(negative): a later transaction failure rolls back availability, stage, and record.
#[tokio::test]
async fn artwork_completion_rolls_back_when_comic_touch_fails() {
    let mock = seed(RoleField::ADMIN);

    let allocation = allocate(&mock, 1).await.unwrap();

    mock.state.lock().unwrap().comics.clear();

    assert!(mark(&mock, allocation.artwork_ver).await.is_err());

    let snapshot = mock.snapshot();

    assert!(
        !snapshot.objs["chapter_artwork"]["chapter-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );

    assert_eq!(
        snapshot.chapters[0].stages.get_phase(Stage::TypesetRedraw),
        StagePhase::Pending
    );

    assert!(snapshot.chapter_workflow_records.is_empty());

    assert_eq!(mock.event_count(), 0);
}

// artwork_limits(alloc_artwork)(negative): invalid sizes and suffixes never reserve objects.
#[tokio::test]
async fn artwork_allocation_enforces_limits_and_frozen_state() {
    let mock = seed(RoleField::ADMIN);

    for (byte_len, ext) in [
        (0, "zip"),
        (512 * 1024 * 1024 + 1, "zip"),
        (10, "../zip"),
        (10, ""),
        (10, "zip/foo"),
    ] {
        let instr = AllocChapterArtworkInstr {
            artwork_hash: ArtworkHash::new([1; 32]),
            new_byte_len: byte_len,
            ext: ext.into(),
        };

        assert!(
            alloc_artwork(
                (&mock, &mock, &mock, &ArtworkConfig::default()),
                token("user-1"),
                "chapter-1".into(),
                instr
            )
            .await
            .is_err()
        );
    }

    assert!(mock.snapshot().objs.is_empty());

    let instr = AllocChapterArtworkInstr {
        artwork_hash: ArtworkHash::new([1; 32]),
        new_byte_len: 512 * 1024 * 1024,
        ext: "PSD".into(),
    };

    let allocated = alloc_artwork(
        (&mock, &mock, &mock, &ArtworkConfig::default()),
        token("user-1"),
        "chapter-1".into(),
        instr,
    )
    .await
    .unwrap();

    mock.state.lock().unwrap().chapters[0].stages = StageMask::try_from(0u32)
        .unwrap()
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    assert!(mark(&mock, allocated.artwork_ver).await.is_err());

    assert!(allocate(&mock, 2).await.is_err());
}
