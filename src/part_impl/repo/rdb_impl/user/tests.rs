// user_roundtrip_uses_testcontainer(GetUserInfo, GetUserCredential, FindUserInfo)(positive): user repo persists and reloads a user from an isolated PostgreSQL container.

use poprako_orchestra::Run;

use poprako_rdb_core::RdbCore;

use crate::part::repo::oper::user::{
    FindUserInfo, GetUserCredential, GetUserInfo,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;

const PREFIX: &str = "rdb-test-user-domain-";

/// Verifies user roundtrip via testcontainers.
/// Verifies user roundtrip via testcontainers.
pub async fn user_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let user_fixture = test_shared::seed_user(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let user_info = repo
        .run(&GetUserInfo {
            id: &user_fixture.user_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(user_info.id, user_fixture.user_entry.id);

    let user_credential = repo
        .run(&GetUserCredential {
            qid: &user_fixture.user_entry.qid,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(user_credential.user_id, user_fixture.user_entry.id);

    let found_user_info = repo
        .run(&FindUserInfo {
            qid: &user_fixture.user_entry.qid,
        })
        .await
        .ok()
        .unwrap()
        .unwrap();

    assert_eq!(found_user_info.id, user_fixture.user_entry.id);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
