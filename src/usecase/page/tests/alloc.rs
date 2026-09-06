use super::super::alloc::alloc_chapter_pages;

use time::OffsetDateTime;

use crate::data::instr::page::{AllocChapterPagesInstr, PageImageInstr};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{IMAGE_CONFIG, assert_expected_variant};
use crate::value::chapter::mask::StageMask;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

fn token() -> UserToken {
    UserToken {
        user_id: "user-1".into(),
    }
}

fn comic() -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: "comic-1".into(),
        workset_id: "workset-1".into(),
        index: 0,
        title: "comic".into(),
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

fn chapter(page_count: usize) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).unwrap(),
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn assignment() -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: "assignment-1".into(),
        chapter_id: "chapter-1".into(),
        user_id: "user-1".into(),
        user: None,
        chapter: None,
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
        created_at: time,
        updated_at: time,
    }
}

fn page(
    id: &str,
    index: usize,
    total: usize,
    translated: usize,
    proofread: usize,
) -> PageInfo {
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        total_unit_count: total,
        translated_unit_count: translated,
        proofread_unit_count: proofread,
        created_at: time,
        updated_at: time,
    }
}

fn page_instr(
    page_id: Option<&str>,
    hash: u8,
    new_byte_len: Option<u64>,
) -> PageImageInstr {
    PageImageInstr {
        raw_ident: None,
        page_id: page_id.map(Into::into),
        image_hash: ImageHash::new([hash; 32]),
        new_byte_len,
        ext: ImageExt::Png,
    }
}

fn seed_scope(mock: &Mock, page_count: usize) {
    mock.seed_comic(comic());

    mock.seed_chapter(chapter(page_count));

    mock.seed_assignment(assignment());
}

#[tokio::test]
async fn mixed_manifest_preserves_order_counts_and_object_obligations() {
    let mock = Mock::new();

    seed_scope(&mock, 3);

    mock.seed_page(page("page-a", 0, 3, 2, 1));

    mock.seed_page(page("page-b", 1, 4, 3, 2));

    mock.seed_page(page("page-deleted", 2, 9, 8, 7));

    mock.seed_page_image_obj("page-a", "png");

    mock.seed_page_image_obj("page-b", "png");

    mock.seed_page_image_obj("page-deleted", "png");

    let instr = AllocChapterPagesInstr {
        chapter_id: "chapter-1".into(),
        pages: vec![
            page_instr(Some("page-b"), 0, None),
            page_instr(None, 5, Some(1024)),
            page_instr(Some("page-a"), 6, Some(2048)),
        ],
    };

    let reserved = alloc_chapter_pages(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .unwrap();

    assert_eq!(reserved.pages.len(), 3);

    assert_eq!(reserved.pages[0].page_id, "page-b");

    assert_eq!(reserved.pages[0].index, 0);

    assert!(reserved.pages[0].slot.is_none());

    assert_eq!(reserved.pages[1].index, 1);

    assert!(reserved.pages[1].slot.is_some());

    assert_eq!(reserved.pages[2].page_id, "page-a");

    assert_eq!(reserved.pages[2].index, 2);

    assert!(reserved.pages[2].slot.is_some());

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 3);

    assert!(snapshot.pages.iter().all(|page| page.id != "page-deleted"));

    let retained_page = snapshot
        .pages
        .iter()
        .find(|page| page.id == "page-b")
        .unwrap();

    assert_eq!(retained_page.total_unit_count, 4);

    assert_eq!(retained_page.translated_unit_count, 3);

    assert_eq!(retained_page.proofread_unit_count, 2);

    let chapter = snapshot
        .chapters
        .iter()
        .find(|chapter| chapter.id == "chapter-1")
        .unwrap();

    assert_eq!(chapter.page_count, 3);

    assert_eq!(chapter.total_unit_count, 7);

    assert_eq!(chapter.translated_unit_count, 5);

    assert_eq!(chapter.proofread_unit_count, 3);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(snapshot.obj_tasks.len(), 4);
}

#[tokio::test]
async fn maximum_manifest_reserves_all_pages_in_input_order() {
    let mock = Mock::new();

    seed_scope(&mock, 0);

    let pages = (0u8..200)
        .map(|index| page_instr(None, index, Some(1024)))
        .collect::<Vec<_>>();

    let instr = AllocChapterPagesInstr {
        chapter_id: "chapter-1".into(),
        pages,
    };

    let reserved = alloc_chapter_pages(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .unwrap();

    assert_eq!(reserved.pages.len(), 200);

    assert!(reserved.pages.iter().enumerate().all(|(index, page)| {
        usize::try_from(page.index).ok() == Some(index) && page.slot.is_some()
    }));

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 200);

    assert_eq!(snapshot.obj_tasks.len(), 200);

    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn unknown_retained_page_rejects_before_manifest_writes() {
    let mock = Mock::new();

    seed_scope(&mock, 1);

    mock.seed_page(page("page-a", 0, 3, 2, 1));

    let instr = AllocChapterPagesInstr {
        chapter_id: "chapter-1".into(),
        pages: vec![page_instr(Some("unknown-page"), 0, None)],
    };

    let result = alloc_chapter_pages(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await;

    assert!(result.is_err());

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(snapshot.pages[0].id, "page-a");

    assert_eq!(snapshot.pages[0].index, 0);

    assert_eq!(snapshot.chapters[0].page_count, 1);

    assert!(snapshot.objs.is_empty());

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn missing_retained_object_rolls_back_manifest_reorder() {
    let mock = Mock::new();

    seed_scope(&mock, 2);

    mock.seed_page(page("page-a", 0, 3, 2, 1));

    mock.seed_page(page("page-b", 1, 4, 3, 2));

    let instr = AllocChapterPagesInstr {
        chapter_id: "chapter-1".into(),
        pages: vec![
            page_instr(Some("page-b"), 0, None),
            page_instr(Some("page-a"), 0, None),
        ],
    };

    let result = alloc_chapter_pages(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await;

    assert!(result.is_err());

    let snapshot = mock.snapshot();

    let page_a = snapshot
        .pages
        .iter()
        .find(|page| page.id == "page-a")
        .unwrap();

    let page_b = snapshot
        .pages
        .iter()
        .find(|page| page.id == "page-b")
        .unwrap();

    assert_eq!(page_a.index, 0);

    assert_eq!(page_b.index, 1);

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn duplicate_page_id_is_args_error_without_side_effects() {
    let mock = Mock::new();

    seed_scope(&mock, 1);

    mock.seed_page(page("page-a", 0, 3, 2, 1));

    let instr = AllocChapterPagesInstr {
        chapter_id: "chapter-1".into(),
        pages: vec![
            page_instr(Some("page-a"), 1, Some(1024)),
            page_instr(Some("page-a"), 2, Some(2048)),
        ],
    };

    let error = alloc_chapter_pages(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(snapshot.pages[0].id, "page-a");

    assert_eq!(snapshot.pages[0].index, 0);

    assert_eq!(snapshot.pages[0].total_unit_count, 3);

    assert_eq!(snapshot.pages[0].translated_unit_count, 2);

    assert_eq!(snapshot.pages[0].proofread_unit_count, 1);

    assert_eq!(snapshot.chapters[0].page_count, 1);

    assert!(snapshot.objs.is_empty());

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn cross_chapter_retained_page_is_args_error_without_side_effects() {
    let mock = Mock::new();

    seed_scope(&mock, 1);

    mock.seed_page(page("page-a", 0, 3, 2, 1));

    let mut other_chapter = chapter(1);

    other_chapter.id = "chapter-2".into();

    mock.seed_chapter(other_chapter);

    let mut other_page = page("page-other", 0, 5, 4, 3);

    other_page.chapter_id = "chapter-2".into();

    mock.seed_page(other_page);

    let instr = AllocChapterPagesInstr {
        chapter_id: "chapter-1".into(),
        pages: vec![page_instr(Some("page-other"), 0, None)],
    };

    let error = alloc_chapter_pages(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 2);

    assert_eq!(snapshot.pages[0].id, "page-a");

    assert_eq!(snapshot.pages[0].index, 0);

    assert_eq!(snapshot.pages[1].id, "page-other");

    assert_eq!(snapshot.pages[1].index, 0);

    assert_eq!(snapshot.pages[1].total_unit_count, 5);

    assert_eq!(snapshot.pages[1].translated_unit_count, 4);

    assert_eq!(snapshot.pages[1].proofread_unit_count, 3);

    assert_eq!(snapshot.chapters[0].page_count, 1);

    assert_eq!(snapshot.chapters[1].page_count, 1);

    assert!(snapshot.objs.is_empty());

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}
