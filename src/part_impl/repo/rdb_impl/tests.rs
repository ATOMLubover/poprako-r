mod schema;

use super::*;

use crate::shared::test_rdb::start;

#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn repo_rdb_impls_use_testcontainer() {
    //
    let test_rdb = start().await;

    let shared = test_rdb.core();

    crate::part_impl::obj_dept::tests::artwork_transactional_mark(
        shared.clone(),
    )
    .await;

    schema::all_application_table_columns_match_generated_schema(
        shared.clone(),
    )
    .await;

    announcement::tests::announcement_roundtrip_uses_testcontainer(
        shared.clone(),
    )
    .await;

    assignment::tests::assignment_roundtrip_uses_testcontainer(shared.clone())
        .await;

    assignment_invitation::tests::assignment_invitation_roundtrip_uses_testcontainer(
        shared.clone(),
    )
    .await;

    chapter::tests::chapter_roundtrip_uses_testcontainer(shared.clone()).await;

    chapter_workflow_record::tests::chapter_workflow_record_roundtrip_uses_testcontainer(
        shared.clone(),
    )
    .await;

    comic::tests::comic_roundtrip_uses_testcontainer(shared.clone()).await;

    comic_archive::tests::comic_archive_roundtrip_uses_testcontainer(
        shared.clone(),
    )
    .await;

    comic_archive::tests::tombstoned_comic_rejects_archive(shared.clone())
        .await;

    comment::tests::comment_roundtrip_uses_testcontainer(shared.clone()).await;

    member::tests::member_roundtrip_uses_testcontainer(shared.clone()).await;

    member::tests::user_cleanup_includes_tombstoned_team_memberships(
        shared.clone(),
    )
    .await;

    member::tests::concurrent_admin_role_removals_are_serialized(
        shared.clone(),
    )
    .await;

    member_invitation::tests::member_invitation_roundtrip_uses_testcontainer(
        shared.clone(),
    )
    .await;

    crate::part_impl::obj_dept::tests::concurrent_claim_is_unique_ordered_and_overflow_safe(
        shared.clone(),
    )
    .await;

    page::tests::page_roundtrip_uses_testcontainer(shared.clone()).await;

    system_mail::tests::system_mail_roundtrip_uses_testcontainer(
        shared.clone(),
    )
    .await;

    subtree_delete::tests::workset_subtree_delete_uses_mark_and_sweep(
        shared.clone(),
    )
    .await;

    team::tests::team_roundtrip_uses_testcontainer(shared.clone()).await;

    team::tests::resolve_team_id_uses_testcontainer(shared.clone()).await;

    term::tests::term_array_unique_and_fuzzy_roundtrip(shared.clone()).await;

    termbase::tests::termbase_unique_and_query_roundtrip(shared.clone()).await;

    unit::tests::unit_roundtrip_uses_testcontainer(shared.clone()).await;

    user::tests::user_roundtrip_uses_testcontainer(shared.clone()).await;

    workset::tests::workset_roundtrip_uses_testcontainer(shared).await;

    drop(test_rdb);
}
