use super::*;

use poprako_orchestra::{Nucl as _, OperRun as _, OperStep as _};

use poprako_obj_dept::key::{KeyMap, ObjGen};
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::model::url::ObjUrlSpec;
use poprako_obj_dept::oper::{
    ClearObjs, DeleteObjs, GenObjUrls, MarkObjUploaded,
};

use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::value::image::{
    ComicCoverKey, ImageExt, PageImageKey, TeamAvatarKey, UserAvatarKey,
};

fn page_dom(page_id: &str) -> PageImageKey {
    PageImageKey {
        chapter_id: "chapter-1".into(),
        page_id: page_id.into(),
        ext: ImageExt::Png,
    }
}

#[test]
fn key_maps_preserve_the_business_physical_key_contract() {
    let page_key = PageImage::forward(&page_dom("page-1"), 7);

    let user_key = UserAvatar::forward(
        &UserAvatarKey {
            user_id: "user-1".into(),
            ext: ImageExt::Jpg,
        },
        8,
    );

    let team_key = TeamAvatar::forward(
        &TeamAvatarKey {
            team_id: "team-1".into(),
            ext: ImageExt::Webp,
        },
        9,
    );

    let comic_key = ComicCover::forward(
        &ComicCoverKey {
            comic_id: "comic-1".into(),
            ext: ImageExt::Avif,
        },
        10,
    );

    assert_eq!(page_key, "page/chapter_chapter-1/page-1-7.png");

    assert_eq!(user_key, "user_avatar/user-1-8.jpg");

    assert_eq!(team_key, "team_avatar/team-1-9.webp");

    assert_eq!(comic_key, "comic_cover/comic-1-10.avif");

    assert_eq!(
        PageImage::forward(&PageImage::reverse(&page_key).unwrap().0, 7),
        page_key
    );

    assert!(PageImage::reverse(&"page_image/page-1/7".into()).is_err());
}

#[test]
fn origin_only_spec_omits_image_renditions() {
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page-1"),
            ver: 1,
            image: "page/chapter_chapter-1/page-1-1.png".into(),
        },
        is_avail: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };

    let obj_url_spec = ObjUrlSpec::default().with_origin();

    let urls = gen_urls(Some(&meta), obj_url_spec, true).unwrap().unwrap();

    assert!(urls.origin_url.is_some());

    assert!(urls.optimized_url.is_none());

    assert!(urls.thumbnail_url.is_none());
}

#[test]
fn selected_image_renditions_are_generated() {
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page-1"),
            ver: 1,
            image: "page/chapter_chapter-1/page-1-1.png".into(),
        },
        is_avail: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };

    let obj_url_spec = ObjUrlSpec::default().with_optimized().with_thumbnail();

    let urls = gen_urls(Some(&meta), obj_url_spec, true).unwrap().unwrap();

    assert!(urls.origin_url.is_none());

    assert!(urls.optimized_url.is_some());

    assert!(urls.thumbnail_url.is_some());
}

#[tokio::test]
async fn mock_operation_rejects_empty_url_spec_without_metadata() {
    let mock = Mock::new();

    let metas = HashMap::new();

    let error = GenObjUrls::<PageImage>::new(&metas, ObjUrlSpec::default())
        .run_on(&mock)
        .await
        .unwrap_err();

    assert!(matches!(error, ObjDeptError::Invalid { .. }));
}

#[tokio::test]
async fn mock_operation_can_disable_thumbnails() {
    let mock = Mock::new().with_obj_thumbnail_disabled();
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page-1"),
            ver: 1,
            image: "page/chapter_chapter-1/page-1-1.png".into(),
        },
        is_avail: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };
    let metas = HashMap::from([(String::from("page-1"), meta)]);

    let obj_url_spec = ObjUrlSpec::default()
        .with_origin()
        .with_optimized()
        .with_thumbnail();

    let urls = GenObjUrls::<PageImage>::new(&metas, obj_url_spec)
        .run_on(&mock)
        .await
        .unwrap();

    assert!(matches!(
        urls.get("page-1"),
        Some(ObjUrls {
            thumbnail_url: None,
            ..
        }),
    ));

    assert!(matches!(
        urls.get("page-1"),
        Some(ObjUrls {
            optimized_url: Some(_),
            ..
        }),
    ));
}

#[tokio::test]
async fn upload_mark_is_exact_current_and_idempotent() {
    let mock = Mock::new();
    let key = ObjKey {
        id: String::from("page-1"),
        ver: 4,
        image: "page/chapter_chapter-1/page-1-4.png".into(),
    };
    let meta = ObjMeta {
        key: key.clone(),
        is_avail: false,
        hash: vec![4; 32],
        ext: String::from("png"),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("page_image")
        .or_default()
        .insert(
            key.id.clone(),
            MockObjRecord {
                version: key.ver,
                meta: Some(meta),
            },
        );

    let generation = ObjGen {
        id: key.id.clone(),
        ver: key.ver,
    };

    let first = MarkObjUploaded::<PageImage>::new(&generation)
        .run_on(&mock)
        .await
        .unwrap();
    let second = MarkObjUploaded::<PageImage>::new(&generation)
        .run_on(&mock)
        .await
        .unwrap();
    let stale_key = ObjGen {
        id: key.id.clone(),
        ver: 3,
    };
    let stale = MarkObjUploaded::<PageImage>::new(&stale_key)
        .run_on(&mock)
        .await
        .unwrap();
    let missing_key = ObjGen {
        id: String::from("missing"),
        ver: 1,
    };
    let missing = MarkObjUploaded::<PageImage>::new(&missing_key)
        .run_on(&mock)
        .await
        .unwrap();

    assert!(first);

    assert!(second);

    assert!(!stale);

    assert!(!missing);

    let snapshot = mock.snapshot();
    let uploaded = snapshot
        .objs
        .get("page_image")
        .and_then(|objs| objs.get("page-1"))
        .and_then(|record| record.meta.as_ref())
        .is_some_and(|meta| meta.is_avail);

    assert!(uploaded);

    let mut state = mock.state.lock().unwrap();

    let detached_prepared = match state
        .objs
        .get_mut("page_image")
        .and_then(|objs| objs.get_mut("page-1"))
    {
        Some(record) => {
            record.meta = None;

            true
        }
        None => false,
    };

    drop(state);

    assert!(detached_prepared);

    let detached = MarkObjUploaded::<PageImage>::new(&generation)
        .run_on(&mock)
        .await
        .unwrap();

    assert!(!detached);
}

#[tokio::test]
async fn slot_and_delete_defer_check_and_delete_debt() {
    let mock = Mock::new();

    let obj_dept = mock.clone();

    mock.coord(async move |context| {
        //
        let obj_spec = ObjSlotSpec {
            dom: page_dom("page-1"),
            hash: &[1; 32],
            content_type: "image/png",
            byte_len: 1024,
        };

        GenObjSlot::<PageImage>::new(&obj_spec)
            .step_on(&obj_dept, context)
            .await?;

        let ids = vec![String::from("page-1")];

        DeleteObjs::<PageImage>::new(&ids)
            .step_on(&obj_dept, context)
            .await?;

        Ok::<(), ObjDeptError>(())
    })
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot
            .objs
            .get("page_image")
            .is_none_or(HashMap::is_empty)
    );

    assert_eq!(snapshot.obj_tasks.len(), 2);

    assert!(matches!(snapshot.obj_tasks[0].1, ObjTask::Check { .. }));

    assert!(matches!(snapshot.obj_tasks[1].1, ObjTask::Delete { .. }));
}

#[tokio::test]
async fn matching_available_content_returns_no_slot_without_mutation() {
    let mock = Mock::new();

    let obj_dept = mock.clone();

    let first_slot = mock
        .coord(async move |context| {
            let obj_spec = ObjSlotSpec {
                dom: page_dom("page-1"),
                hash: &[1; 32],
                content_type: "image/png",
                byte_len: 1024,
            };

            GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await
        })
        .await
        .unwrap()
        .unwrap();

    MarkObjUploaded::<PageImage>::new(&ObjGen {
        id: first_slot.key.id.clone(),
        ver: first_slot.key.ver,
    })
    .run_on(&mock)
    .await
    .unwrap();

    let task_count = mock.snapshot().obj_tasks.len();

    let obj_dept = mock.clone();

    let repeated_slot = mock
        .coord(async move |context| {
            let obj_spec = ObjSlotSpec {
                dom: page_dom("page-1"),
                hash: &[1; 32],
                content_type: "image/png",
                byte_len: 1024,
            };

            GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await
        })
        .await
        .unwrap();

    assert!(repeated_slot.is_none());
    assert_eq!(mock.snapshot().objs["page_image"]["page-1"].version, 1);
    assert_eq!(mock.snapshot().obj_tasks.len(), task_count);
}

#[tokio::test]
async fn matching_pending_content_resumes_the_current_generation() {
    let mock = Mock::new();

    let obj_dept = mock.clone();

    let first_slot = mock
        .coord(async move |context| {
            let obj_spec = ObjSlotSpec {
                dom: page_dom("page-1"),
                hash: &[1; 32],
                content_type: "image/png",
                byte_len: 1024,
            };

            GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await
        })
        .await
        .unwrap()
        .unwrap();

    let obj_dept = mock.clone();

    let resumed_slot = mock
        .coord(async move |context| {
            let obj_spec = ObjSlotSpec {
                dom: page_dom("page-1"),
                hash: &[1; 32],
                content_type: "image/png",
                byte_len: 1024,
            };

            GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(resumed_slot.key, first_slot.key);
    assert_eq!(mock.snapshot().objs["page_image"]["page-1"].version, 1);
    assert_eq!(mock.snapshot().obj_tasks.len(), 2);
    assert!(
        mock.snapshot()
            .obj_tasks
            .iter()
            .all(|(_, task)| matches!(task, ObjTask::Check { .. }))
    );
}

#[tokio::test]
async fn clear_allows_replacement_without_reusing_a_generation() {
    let mock = Mock::new();
    let obj_dept = mock.clone();

    let replacement = mock
        .coord(async move |context| {
            let obj_spec = ObjSlotSpec {
                dom: page_dom("page-1"),
                hash: &[1; 32],
                content_type: "image/png",
                byte_len: 1024,
            };

            GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await?;

            let ids = vec![String::from("page-1")];

            ClearObjs::<PageImage>::new(&ids)
                .step_on(&obj_dept, context)
                .await?;

            GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await
        })
        .await
        .unwrap();

    assert_eq!(replacement.unwrap().key.ver, 2);
}

#[tokio::test]
async fn batch_slots_reject_duplicate_ids_before_mutation() {
    let mock = Mock::new();
    let obj_dept = mock.clone();

    let result = mock
        .coord(async move |context| {
            let specs = [
                ObjSlotSpec {
                    dom: page_dom("page-1"),
                    hash: &[1; 32],
                    content_type: "image/png",
                    byte_len: 1024,
                },
                ObjSlotSpec {
                    dom: page_dom("page-1"),
                    hash: &[2; 32],
                    content_type: "image/png",
                    byte_len: 2048,
                },
            ];

            GenObjSlots::<PageImage>::new(&specs)
                .step_on(&obj_dept, context)
                .await
        })
        .await;

    assert!(matches!(
        result,
        Err(poprako_orchestra::nucl::Error::Step(
            ObjDeptError::Invalid { .. }
        )),
    ));

    let snapshot = mock.snapshot();

    assert!(snapshot.objs.get("page_image").is_none());

    assert!(snapshot.obj_tasks.is_empty());
}

// artwork_key_mapping(ChapterArtwork)(negative): artwork keys round-trip arbitrary safe file suffixes and reject malformed identities.
#[test]
fn artwork_key_mapping_rejects_noncanonical_paths() {
    use crate::part::obj_dept::ChapterArtwork;
    use crate::value::artwork::ChapterArtworkKey;

    let artwork_key = ChapterArtworkKey {
        chapter_id: "chapter-123".into(),
        ext: "PSD".into(),
    };

    let physical_key = ChapterArtwork::forward(&artwork_key, 3);

    assert_eq!(
        ChapterArtwork::reverse(&physical_key).unwrap(),
        (artwork_key, 3)
    );

    for invalid_key in [
        "chapter_artwork/chapter-123-03.PSD",
        "chapter_artwork/chapter-123-3../zip",
        "page/chapter-123-3.zip",
        "chapter_artwork/-3.zip",
    ] {
        assert!(ChapterArtwork::reverse(&invalid_key.to_owned()).is_err());
    }
}
