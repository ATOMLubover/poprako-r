// archive(archive)(positive): archive should retain the comic marker, queue every image key, and remove active descendants.
// archive(archive)(negative): non-admin callers should not create archive rows or delete active data.
// archive(archive)(negative): archive persistence failure should roll back payload, outbox, and active-data changes.
// export(export)(positive): retained month slots should return stored JSON strings without decoding.

use super::*;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::task::ObjTask;
use time::OffsetDateTime;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::page::{PageInfo, PageRawIdentInfo};
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::unit::UnitCoord;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::role::{RoleField, RoleMask};

fn seed_archive_scope(mock: &Mock, member_roles: RoleMask) {
    //
    let archived_at = OffsetDateTime::now_utc();

    let stage_mask = StageMask::try_from(0)
        .unwrap()
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_user(
        UserInfo {
            id: "user-1".into(),
            qid: "qid-user-1".into(),
            nickname: "archiver".into(),
            is_sadmin: false,
            last_active_at: archived_at,
            created_at: archived_at,
            updated_at: archived_at,
        },
        UserCredential {
            user_id: "user-1".into(),
            password_hash: "hashed".into(),
        },
    );

    mock.seed_workset(WorksetInfo {
        id: "workset-1".into(),
        team_id: "team-1".into(),
        index: 4,
        name: "workset".into(),
        description: Some("archive scope".into()),
        comic_count: 7,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_member(MemberInfo {
        id: "member-1".into(),
        user_id: "user-1".into(),
        user_nickname: "archiver".into(),
        user_last_active_at: archived_at,
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: member_roles,
    });

    mock.seed_comic(ComicInfo {
        id: "comic-1".into(),
        workset_id: "workset-1".into(),
        index: 2,
        title: "comic title".into(),
        author: "comic author".into(),
        description: Some("comic description".into()),
        chapter_count: 1,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: archived_at,
        archived_at: None,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_chapter(ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter subtitle".into(),
        page_count: 1,
        total_unit_count: 1,
        translated_unit_count: 1,
        proofread_unit_count: 1,
        stages: stage_mask,
        creator_id: "user-1".into(),
        creator: None,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_assignment(AssignmentInfo {
        id: "assignment-1".into(),
        chapter_id: "chapter-1".into(),
        user_id: "user-1".into(),
        user: None,
        chapter: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_assignment_invitation(AssignmentInvitationInfo {
        id: "invitation-1".into(),
        chapter_id: "chapter-1".into(),
        inviter_id: "user-1".into(),
        invitee_qid: "qid-invitee".into(),
        code: "invite-code".into(),
        is_pending: true,
        roles: RoleMask::from(RoleField::PROOFREADER),
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.state.lock().unwrap().page_raw_idents.insert(
        "page-1".into(),
        PageRawIdentInfo {
            page_id: "page-1".into(),
            raw_ident: "source.png".into(),
            created_at: archived_at,
            updated_at: archived_at,
        },
    );

    mock.seed_page(PageInfo {
        id: "page-1".into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        total_unit_count: 1,
        translated_unit_count: 1,
        proofread_unit_count: 1,
        created_at: archived_at,
        updated_at: archived_at,
    });

    mock.seed_unit(UnitInfo {
        id: "unit-1".into(),
        page_id: "page-1".into(),
        next_id: None,
        is_bubble: true,
        is_proofread: true,
        coord: UnitCoord {
            x_coord: 1.5,
            y_coord: 2.5,
        },
        translated_text: Some("translated".into()),
        last_translator_id: Some("user-1".into()),
        proofread_text: Some("proofread".into()),
        last_proofreader_id: Some("user-1".into()),
        hidden_at: None,
        created_at: archived_at,
        updated_at: archived_at,
    });
}

fn token() -> UserToken {
    UserToken {
        user_id: "user-1".into(),
    }
}

fn seed_archive_objs(mock: &Mock) -> (ObjKey, ObjKey) {
    let cover_key = ObjKey {
        id: "comic-1".into(),
        ver: 5,
        image: "comic_cover/comic-1-5.png".into(),
    };

    let page_key = ObjKey {
        id: "page-1".into(),
        ver: 4,
        image: "page/chapter_chapter-1/page-1-4.webp".into(),
    };

    let mut state = mock.state.lock().unwrap();

    state.objs.entry("comic_cover").or_default().insert(
        "comic-1".into(),
        MockObjRecord {
            version: cover_key.ver,
            meta: Some(ObjMeta {
                key: cover_key.clone(),
                is_avail: false,
                hash: vec![5; 32],
                ext: "png".into(),
            }),
        },
    );

    state.objs.entry("page_image").or_default().insert(
        "page-1".into(),
        MockObjRecord {
            version: page_key.ver,
            meta: Some(ObjMeta {
                key: page_key.clone(),
                is_avail: false,
                hash: vec![4; 32],
                ext: "webp".into(),
            }),
        },
    );

    (cover_key, page_key)
}

#[tokio::test]
async fn archive_retains_comic_marker_queues_images_and_deletes_children() {
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    let (cover_key, page_key) = seed_archive_objs(&mock);

    let archive_comic_val =
        archive((&mock, &mock, &mock), token(), "comic-1".into())
            .await
            .unwrap();

    let snapshot = mock.snapshot();

    assert_ne!(archive_comic_val.archived_id, "comic-1");

    assert_eq!(snapshot.comics.len(), 1);
    assert_eq!(snapshot.comics[0].id, "comic-1");
    assert!(snapshot.comics[0].archived_at.is_some());

    assert!(snapshot.chapters.is_empty());
    assert!(snapshot.assignments.is_empty());
    assert!(snapshot.assignment_invitations.is_empty());
    assert!(snapshot.pages.is_empty());
    assert!(snapshot.page_raw_idents.is_empty());
    assert!(snapshot.units.is_empty());

    assert_eq!(snapshot.worksets[0].comic_count, 7);

    assert_eq!(snapshot.comic_archives.len(), 1);
    assert_eq!(snapshot.comic_archives[0].team_id, "team-1");
    assert_eq!(snapshot.comic_archives[0].source_comic_id, "comic-1");
    assert_eq!(snapshot.comic_archives[0].archiver_id, "user-1");

    let archived_comic_payload: serde_json::Value =
        serde_json::from_str(&snapshot.comic_archives[0].archived_payload)
            .unwrap();

    assert_eq!(archived_comic_payload["source_comic_id"], "comic-1");
    assert_eq!(archived_comic_payload["workset"]["id"], "workset-1");
    assert_eq!(
        archived_comic_payload["chapters"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        archived_comic_payload["chapters"][0]["source_chapter_id"],
        "chapter-1"
    );
    assert_eq!(
        archived_comic_payload["chapters"][0]["assignments"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        archived_comic_payload["chapters"][0]["assignments"][0]["user"]["nickname"],
        "archiver"
    );
    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"][0]["source_page_id"],
        "page-1"
    );
    assert_eq!(
        archived_comic_payload["chapters"][0]["pages"][0]["units"][0]["source_unit_id"],
        "unit-1"
    );

    assert!(snapshot.objs["comic_cover"]["comic-1"].meta.is_none());
    assert!(snapshot.objs["page_image"].is_empty());
    assert_eq!(snapshot.obj_tasks.len(), 2);
    assert!(snapshot.obj_tasks.iter().any(|(topic, task)| {
        *topic == "comic_cover"
            && matches!(task, ObjTask::Delete { key } if key == &cover_key)
    }));
    assert!(snapshot.obj_tasks.iter().any(|(topic, task)| {
        *topic == "page_image"
            && matches!(task, ObjTask::Delete { key } if key == &page_key)
    }));
}

#[tokio::test]
async fn export_returns_stored_strings_grouped_by_month() {
    //
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    archive((&mock, &mock, &mock), token(), "comic-1".into())
        .await
        .unwrap();

    let now = OffsetDateTime::now_utc();

    let month = format!("{:04}-{:02}", now.year(), u8::from(now.month()));

    let val = export(
        (&mock,),
        token(),
        "team-1".into(),
        ExportComicArchivesInstr {
            months: vec![month.clone()],
        },
    )
    .await
    .unwrap();

    let stored = &mock.snapshot().comic_archives[0].archived_payload;

    assert_eq!(val.0[&month], vec![stored.clone()]);
}

#[tokio::test]
async fn archive_rejects_non_admin_without_writing_or_deleting() {
    //
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::TRANSLATOR));

    let (cover_key, page_key) = seed_archive_objs(&mock);

    let archive_result =
        archive((&mock, &mock, &mock), token(), "comic-1".into()).await;

    assert_expected_variant(archive_result.unwrap_err(), ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.len(), 1);

    assert_eq!(snapshot.comic_archives.len(), 0);

    assert_eq!(
        snapshot.objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        cover_key
    );
    assert_eq!(
        snapshot.objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        page_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn archive_rejects_tombstoned_comic_without_writing_or_deleting() {
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    let (cover_key, page_key) = seed_archive_objs(&mock);

    mock.state
        .lock()
        .unwrap()
        .deleted_comic_ids
        .insert("comic-1".into());

    let archive_result =
        archive((&mock, &mock, &mock), token(), "comic-1".into()).await;

    assert_expected_variant(archive_result.unwrap_err(), ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.comic_archives.is_empty());
    assert_eq!(snapshot.chapters.len(), 1);
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.page_raw_idents.len(), 1);

    assert_eq!(
        snapshot.objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        cover_key
    );
    assert_eq!(
        snapshot.objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        page_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn archive_rolls_back_when_archive_persistence_fails() {
    //
    let mock = Mock::new().with_archive_commit_failure();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    let (cover_key, page_key) = seed_archive_objs(&mock);

    let archive_result =
        archive((&mock, &mock, &mock), token(), "comic-1".into()).await;

    assert!(matches!(
        archive_result,
        Err(BaseError::Unrecoverable { .. })
    ));

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.len(), 1);

    assert_eq!(snapshot.chapters.len(), 1);

    assert_eq!(snapshot.assignments.len(), 1);

    assert_eq!(snapshot.assignment_invitations.len(), 1);

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(snapshot.units.len(), 1);

    assert_eq!(snapshot.worksets[0].comic_count, 7);

    assert_eq!(snapshot.comic_archives.len(), 0);

    assert_eq!(
        snapshot.objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        cover_key
    );
    assert_eq!(
        snapshot.objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        page_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}
