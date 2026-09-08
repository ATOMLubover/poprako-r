use poprako_orchestra::{Nucl as _, OperRun as _};

use super::Mock;
use crate::part::repo::oper::user::GetUserInfo;
use crate::result::BaseError;
use crate::test_util::fixture::{credential, user};

#[tokio::test]
async fn run_reads_seeded_user() {
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let user_info = GetUserInfo { id: "user-1" }.run_on(&mock).await.unwrap();

    assert_eq!(user_info.nickname, "Nick");
}

#[tokio::test]
async fn coord_rolls_back_state() {
    let mock = Mock::new();

    let seeded = user("user-1", "qid-1", "Nick");

    let rest = mock
        .coord(async move |context| {
            context.state.users.push(seeded);

            Err::<(), _>(BaseError::Unrecoverable {
                message: "rollback".into(),
            })
        })
        .await;

    assert!(rest.is_err());

    assert!(mock.snapshot().users.is_empty());
}
