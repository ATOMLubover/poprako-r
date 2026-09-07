use super::super::delete::delete;
use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::model::read::proj::page::PageRawIdentInfo;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::role::RoleField;

#[tokio::test]
async fn admin_delete_removes_pages_objects_and_clears_chapter_counts() {
    let mock = Mock::new();

    seed_page_scope(&mock, 2);

    {
        let mut state = mock.state.lock().unwrap();
        let chapter_info = &mut state.chapters[0];

        chapter_info.total_unit_count = 7;
        chapter_info.translated_unit_count = 5;
        chapter_info.proofread_unit_count = 3;
    }

    mock.seed_member(page_member("user-1", RoleMask::from(RoleField::ADMIN)));
    mock.seed_page(page_model("page-1", 0));
    mock.seed_page(page_model("page-2", 1));

    let page_key = seed_page_obj(&mock, "page-1", 1, true, 1, ImageExt::Png);

    let before = OffsetDateTime::now_utc();

    mock.state.lock().unwrap().page_raw_idents.insert(
        "page-1".into(),
        PageRawIdentInfo {
            page_id: "page-1".into(),
            raw_ident: "source.png".into(),
            created_at: before,
            updated_at: before,
        },
    );

    delete(
        (&mock, &mock, &mock),
        page_token("user-1"),
        "chapter-1".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();
    let chapter_info = &snapshot.chapters[0];

    assert!(snapshot.pages.is_empty());
    assert!(snapshot.page_raw_idents.is_empty());
    assert_eq!(chapter_info.page_count, 0);
    assert_eq!(chapter_info.total_unit_count, 0);
    assert_eq!(chapter_info.translated_unit_count, 0);
    assert_eq!(chapter_info.proofread_unit_count, 0);
    assert!(snapshot.objs["page_image"].is_empty());
    assert_eq!(snapshot.obj_tasks.len(), 1);
    assert!(matches!(
        &snapshot.obj_tasks[0].1,
        ObjTask::Delete { key } if key == &page_key
    ));
    assert!(snapshot.comics[0].last_active_at >= before);
}

#[tokio::test]
async fn non_admin_delete_rejection_rolls_back_pages_and_objects() {
    let mock = Mock::new();

    seed_page_scope(&mock, 1);

    mock.seed_member(page_member(
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_page(page_model("page-1", 0));

    let page_key = seed_page_obj(&mock, "page-1", 1, true, 1, ImageExt::Png);

    let error = delete(
        (&mock, &mock, &mock),
        page_token("user-1"),
        "chapter-1".into(),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(error, ExpectedVariant::Perm);
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.chapters[0].page_count, 1);
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
