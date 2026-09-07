use super::super::alloc::alloc_chapter_pages;
use super::super::alloc::alloc_image;
use super::super::mark_image_uploaded;
use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::data::instr::page::{
    AllocChapterPagesInstr, AllocPageImageInstr, MarkPageImageUploadedInstr,
    PageImageInstr,
};
use crate::part_impl::prom::mock_impl::process_pending;
use crate::result::ExpectedVariant;
use crate::test_util::{IMAGE_CONFIG, assert_expected_variant};
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::role::RoleField;

fn seed_manifest_scope(mock: &Mock, page_count: usize) {
    seed_page_scope(mock, page_count);

    mock.seed_assignment(page_assignment(
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));
}

fn manifest_page(
    page_id: Option<&str>,
    hash: u8,
    new_byte_len: Option<u64>,
    ext: ImageExt,
) -> PageImageInstr {
    PageImageInstr {
        raw_ident: None,
        page_id: page_id.map(Into::into),
        image_hash: ImageHash::new([hash; 32]),
        new_byte_len,
        ext,
    }
}

async fn alloc_manifest(
    mock: &Mock,
    pages: Vec<PageImageInstr>,
) -> crate::result::BaseRest<crate::data::val::page::AllocChapterPagesVal> {
    alloc_chapter_pages(
        (mock, mock, mock, mock, &IMAGE_CONFIG),
        page_token("user-1"),
        AllocChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages,
        },
    )
    .await
}

#[tokio::test]
async fn retained_pending_image_without_byte_length_needs_no_new_slot() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_model("page-1", 0));

    let existing_key =
        seed_page_obj(&mock, "page-1", 2, false, 0, ImageExt::Png);

    let allocated = alloc_manifest(
        &mock,
        vec![manifest_page(Some("page-1"), 0, None, ImageExt::Png)],
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(allocated.pages[0].slot.is_none());
    assert_eq!(snapshot.pages[0].id, "page-1");
    assert_eq!(
        snapshot.objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        existing_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn retained_pending_image_with_byte_length_resigns_same_generation() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_model("page-1", 0));

    let existing_key =
        seed_page_obj(&mock, "page-1", 2, false, 0, ImageExt::Png);

    let allocated = alloc_manifest(
        &mock,
        vec![manifest_page(Some("page-1"), 0, Some(4096), ImageExt::Png)],
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(allocated.pages[0].slot.as_ref().unwrap().image_ver, 2);
    assert_eq!(snapshot.objs["page_image"]["page-1"].version, 2);
    assert_eq!(snapshot.obj_tasks.len(), 1);
    assert!(matches!(
        &snapshot.obj_tasks[0].1,
        ObjTask::Check { key } if key == &existing_key
    ));
}

#[tokio::test]
async fn new_manifest_page_without_byte_length_is_rejected_before_writes() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let error = alloc_manifest(
        &mock,
        vec![manifest_page(None, 1, None, ImageExt::Jpg)],
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(error, ExpectedVariant::Args);
    assert!(snapshot.pages.is_empty());
    assert!(snapshot.objs.is_empty());
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn changed_retained_image_without_byte_length_rolls_back_manifest() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_model("page-1", 0));

    let existing_key =
        seed_page_obj(&mock, "page-1", 2, true, 0, ImageExt::Png);

    let error = alloc_manifest(
        &mock,
        vec![manifest_page(Some("page-1"), 1, None, ImageExt::Jpg)],
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(error, ExpectedVariant::Args);
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.pages[0].id, "page-1");
    assert_eq!(snapshot.pages[0].index, 0);
    assert_eq!(
        snapshot.objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        existing_key
    );
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn delayed_raw_advance_stays_pending_while_manifest_images_are_pending() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    alloc_manifest(
        &mock,
        vec![
            manifest_page(None, 1, Some(4096), ImageExt::Png),
            manifest_page(None, 2, Some(4096), ImageExt::Png),
        ],
    )
    .await
    .unwrap();

    process_pending(&mock).await.unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot.chapters[0]
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
    );
    assert!(snapshot.objs["page_image"].values().all(|record| {
        record.meta.as_ref().is_some_and(|meta| !meta.is_avail)
    }));
}

#[tokio::test]
async fn invalid_manifest_count_is_rejected_without_side_effects() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let error = alloc_manifest(&mock, Vec::new()).await.err().unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(error, ExpectedVariant::Args);
    assert!(snapshot.pages.is_empty());
    assert!(snapshot.objs.is_empty());
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn explicit_image_replacement_retains_page_and_deletes_old_generation() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    let mut existing_page = page_model("page-1", 0);

    existing_page.total_unit_count = 4;

    mock.seed_page(existing_page);

    let old_key = seed_page_obj(&mock, "page-1", 7, true, 0, ImageExt::Png);

    let allocated = alloc_manifest(
        &mock,
        vec![manifest_page(Some("page-1"), 1, Some(8192), ImageExt::Jpg)],
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(allocated.pages[0].page_id, "page-1");
    assert_eq!(allocated.pages[0].slot.as_ref().unwrap().image_ver, 8);
    assert_eq!(snapshot.pages[0].total_unit_count, 4);
    assert_eq!(snapshot.chapters[0].total_unit_count, 4);
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key == &old_key)
    }));
}

#[tokio::test]
async fn delayed_raw_advance_completes_after_every_generation_is_marked() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let allocated = alloc_manifest(
        &mock,
        vec![
            manifest_page(None, 1, Some(4096), ImageExt::Png),
            manifest_page(None, 2, Some(4096), ImageExt::Png),
        ],
    )
    .await
    .unwrap();

    for page in allocated.pages {
        let image_ver = page.slot.unwrap().image_ver;

        mark_image_uploaded(
            (&mock, &mock),
            page_token("user-1"),
            page.page_id,
            MarkPageImageUploadedInstr { image_ver },
        )
        .await
        .unwrap();
    }

    process_pending(&mock).await.unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot.chapters[0]
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Completed)
    );
    assert_eq!(snapshot.chapter_workflow_records.len(), 1);
}

#[tokio::test]
async fn published_chapter_rejects_allocations_but_accepts_current_mark() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.state.lock().unwrap().chapters[0].stages = StageMask::try_from(0u32)
        .unwrap()
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_page(page_model("page-1", 0));

    seed_page_obj(&mock, "page-1", 1, false, 0, ImageExt::Png);

    let manifest_result = alloc_manifest(
        &mock,
        vec![manifest_page(Some("page-1"), 0, Some(4096), ImageExt::Png)],
    )
    .await;

    assert!(manifest_result.is_err());

    let image_result = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_token("user-1"),
        "page-1".into(),
        AllocPageImageInstr {
            raw_ident: None,
            image_hash: ImageHash::new([1; 32]),
            new_byte_len: 4096,
            ext: ImageExt::Jpg,
        },
    )
    .await;

    assert!(image_result.is_err());

    mark_image_uploaded(
        (&mock, &mock),
        page_token("user-1"),
        "page-1".into(),
        MarkPageImageUploadedInstr { image_ver: 1 },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot.objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn raw_ident_is_replaced_after_manifest_matching_and_reordering() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let mut first = manifest_page(None, 1, Some(4096), ImageExt::Png);

    first.raw_ident = Some("first.JPG".into());

    let mut second = manifest_page(None, 2, Some(4096), ImageExt::Png);

    second.raw_ident = Some("second.PNG".into());

    let allocated = alloc_manifest(&mock, vec![first, second]).await.unwrap();

    let first_id = allocated.pages[0].page_id.clone();

    let second_id = allocated.pages[1].page_id.clone();

    let mut second = manifest_page(None, 2, None, ImageExt::Png);

    second.raw_ident = Some("renamed.PNG".into());

    let reordered = alloc_manifest(
        &mock,
        vec![second, manifest_page(None, 1, None, ImageExt::Png)],
    )
    .await
    .unwrap();

    assert_eq!(reordered.pages[0].page_id, second_id);
    assert_eq!(reordered.pages[1].page_id, first_id);
    assert!(reordered.pages.iter().all(|page| page.slot.is_none()));

    let snapshot = mock.snapshot();

    assert!(!snapshot.page_raw_idents.contains_key(&first_id));
    assert_eq!(
        snapshot.page_raw_idents[&second_id].raw_ident,
        "renamed.PNG"
    );

    let mut retained = manifest_page(Some(&second_id), 2, None, ImageExt::Png);

    retained.raw_ident = None;

    alloc_manifest(&mock, vec![retained]).await.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);
    assert!(snapshot.page_raw_idents.is_empty());
}

#[tokio::test]
async fn manifest_failure_rolls_back_raw_ident_assignment() {
    let mock = Mock::new().with_obj_delete_failure();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_model("page-1", 0));

    seed_page_obj(&mock, "page-1", 1, true, 0, ImageExt::Png);

    let mut replacement = manifest_page(None, 2, Some(4096), ImageExt::Png);

    replacement.raw_ident = Some("replacement.png".into());

    assert!(alloc_manifest(&mock, vec![replacement]).await.is_err());

    let snapshot = mock.snapshot();

    assert!(snapshot.page_raw_idents.is_empty());
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.pages[0].id, "page-1");
    assert_eq!(snapshot.objs["page_image"]["page-1"].version, 1);
}

#[tokio::test]
async fn repeated_reallocation_does_not_retain_obsolete_raw_idents() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    for hash in 1..=16 {
        let mut page = manifest_page(None, hash, Some(4096), ImageExt::Png);

        page.raw_ident = Some(format!("source-{hash}.png"));

        let allocated = alloc_manifest(&mock, vec![page]).await.unwrap();

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.pages.len(), 1);
        assert_eq!(snapshot.page_raw_idents.len(), 1);
        assert!(
            snapshot
                .page_raw_idents
                .contains_key(&allocated.pages[0].page_id)
        );
    }

    assert!(alloc_manifest(&mock, vec![]).await.is_err());

    assert_eq!(mock.snapshot().page_raw_idents.len(), 1);

    alloc_manifest(&mock, vec![manifest_page(None, 16, None, ImageExt::Png)])
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);
    assert!(snapshot.page_raw_idents.is_empty());
}
