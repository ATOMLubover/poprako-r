use super::{
    ObjViewIds, ObjViewSnapshot, TestContext, TestObjDept, TestRepo,
    assignment_info,
};
use crate::usecase::internal::page::{PageLoader, PinnedChapterSnapshot};

#[tokio::test]
async fn queried_pinned_chapters_are_reused_for_cover_fallback() {
    let obj_dept = TestObjDept::default();

    obj_dept.omit("cover-list", "comic-1");

    let repo = TestRepo::default();

    let comic_ids = ["comic-1"];

    let pinned_chapter_snapshot =
        PinnedChapterSnapshot::load_from_comics(&repo, &comic_ids)
            .await
            .unwrap();

    let assignment_info = assignment_info();

    let mut ids = ObjViewIds::default();

    ids.collect_assignments(std::slice::from_ref(&assignment_info));

    let snapshot = ObjViewSnapshot::load_with_comic_fallbacks::<
        TestContext,
        _,
        _,
    >(&repo, &obj_dept, ids, Some(&pinned_chapter_snapshot))
    .await
    .unwrap();

    assert_eq!(repo.pinned_chapter_calls().len(), 1);

    assert_eq!(
        snapshot
            .assignment(assignment_info)
            .chapter
            .and_then(|chapter| chapter.comic)
            .and_then(|comic| comic.cover_url),
        Some("https://obj.test/page-1".into()),
    );
}

#[tokio::test]
async fn queried_absent_pinned_chapters_are_not_loaded_again() {
    let repo = TestRepo::default();

    let comic_ids = ["comic-without-pinned-chapter"];

    let pinned_chapter_snapshot =
        PinnedChapterSnapshot::load_from_comics(&repo, &comic_ids)
            .await
            .unwrap();

    let page_ids = PageLoader::load_ids_from_comics(
        &repo,
        &comic_ids,
        Some(&pinned_chapter_snapshot),
    )
    .await
    .unwrap();

    assert!(page_ids.is_empty());
    assert_eq!(repo.pinned_chapter_calls().len(), 1);
}
