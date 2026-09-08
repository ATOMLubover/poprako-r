//! Test fixtures and cases for the user use case module.
//!
//! Tests exercise user profile reads, updates, avatar management, activity
//! tracking, and account deletion against a [`Mock`] that doubles as the
//! coordinator, repository, prom enqueuer, image pool, and effect developer.
//!
//! [`Mock`]: crate::part_impl::repo::mock_impl::Mock

// Delete flow coverage, including avatar cleanup and related records.
mod delete;
mod extra;

// get_info(get_info)(positive): self and other-user reads preserve activity without emitting events.
// get_info(get_info)(negative): missing user should propagate an argument error.
// update_info(update_info)(positive): owner update should change user info and member nickname.
// update_info(update_info)(negative): non-owner update should return a perm error without mutation.
// update_info(update_info)(negative): missing user should rollback the transaction.
// reserve_avatar(reserve_avatar)(positive): first reservation should update avatar state, enqueue a check, and return a put URL.
// reserve_avatar(reserve_avatar)(positive): replacing an avatar should enqueue delete and check messages.
// reserve_avatar(reserve_avatar)(negative): missing user should rollback avatar and prom state.
// reserve_avatar(reserve_avatar)(negative): put URL failure should propagate after transaction commit.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): matching owner and version should mark the avatar uploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): repeated matching version confirmation should remain successful.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): non-owner mark should return a perm error.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): stale version should rollback uploaded state.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): old reservation replay should fail without marking current avatar uploaded.
// delete(delete)(positive): owner delete should remove user, credentials, and memberships, and enqueue uploaded avatar deletion.
// delete(delete)(positive): deleting a user without an uploaded avatar should not enqueue prom records.
// delete(delete)(negative): deleting a user must retain an admin in every affected team.
// delete(delete)(negative): non-owner delete should return a perm error without mutation.
// delete(delete)(negative): missing user should rollback state.

use super::*;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use time::OffsetDateTime;

use crate::complex::user::UserComplex;
use crate::data::instr::user::{
    MarkUserAvatarUploadedInstr, UpdateUserInfoInstr, UpdateUserPasswordInstr,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockObjRecord};
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::{credential, user};
use crate::value::role::{RoleField, RoleMask};

/// Builds a [`MemberInfo`] fixture.
fn member(
    id: &str,
    user_id: &str,
    user_nickname: &str,
    team_id: &str,
) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_nickname.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::ADMIN),
    }
}

/// Builds a [`UserToken`] fixture for the given user ID.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

// Seeds one current user-avatar generation that is not yet available.
fn seed_user_avatar(mock: &Mock, user_id: &str, version: u32) {
    let key = ObjKey {
        id: user_id.into(),
        ver: version,
        image: format!("user_avatar/{}-{}.png", user_id, version),
    };

    let meta = ObjMeta {
        key,
        is_avail: false,
        hash: vec![0; 32],
        ext: "png".into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("user_avatar")
        .or_default()
        .insert(
            user_id.into(),
            MockObjRecord {
                version,
                meta: Some(meta),
            },
        );
}

/// Builds an [`UpdateUserInfoData`] fixture.
fn update_instr(id: &str, qid: &str, nickname: &str) -> UpdateUserInfoInstr {
    UpdateUserInfoInstr {
        id: id.into(),
        qid: qid.into(),
        nickname: nickname.into(),
    }
}

/// Builds [`UpdateUserPasswordInstr`] for replacing a user's password.
fn update_password_instr(
    current_password: &str,
    new_password: &str,
) -> UpdateUserPasswordInstr {
    UpdateUserPasswordInstr {
        current_password: current_password.into(),
        new_password: new_password.into(),
    }
}

#[tokio::test]
async fn get_info_preserves_activity_for_self() {
    //
    let mock = Mock::new();

    let mut user_info = user("user-1", "qid-1", "Nick");

    user_info.last_active_at = OffsetDateTime::UNIX_EPOCH;

    mock.seed_user(user_info, credential("user-1", "password"));

    let val = get_info((&mock, &mock), token("user-1"), "user-1".into())
        .await
        .unwrap();

    assert_eq!(val.id, "user-1");

    assert_eq!(val.nickname, "Nick");

    assert_eq!(val.last_active_at, 0);

    assert_eq!(
        mock.snapshot().users[0].last_active_at,
        OffsetDateTime::UNIX_EPOCH
    );

    assert_eq!(mock.event_count(), 0);
}

#[tokio::test]
async fn get_info_does_not_emit_active_for_other_user() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-2", "qid-2", "Other"),
        credential("user-2", "password"),
    );

    get_info((&mock, &mock), token("user-1"), "user-2".into())
        .await
        .unwrap();

    assert_eq!(mock.event_count(), 0);
}

#[tokio::test]
async fn get_info_propagates_missing_user() {
    //
    let mock = Mock::new();

    let err = get_info((&mock, &mock), token("user-1"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn mark_avatar_uploaded_optimistically_exposes_current_generation() {
    //
    let mock = Mock::new();

    seed_user_avatar(&mock, "user-1", 3);

    for _ in 0..2 {
        mark_avatar_uploaded::<MockContext, _>(
            (&mock,),
            token("user-1"),
            "user-1".into(),
            MarkUserAvatarUploadedInstr { image_ver: 3 },
        )
        .await
        .unwrap();
    }

    let snapshot = mock.snapshot();

    let avatar_meta = snapshot.objs["user_avatar"]["user-1"]
        .meta
        .as_ref()
        .unwrap();

    assert!(avatar_meta.is_avail);
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_stale_generation_without_mutation() {
    //
    let mock = Mock::new();

    seed_user_avatar(&mock, "user-1", 3);

    let err = mark_avatar_uploaded::<MockContext, _>(
        (&mock,),
        token("user-1"),
        "user-1".into(),
        MarkUserAvatarUploadedInstr { image_ver: 2 },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    let avatar_meta = snapshot.objs["user_avatar"]["user-1"]
        .meta
        .as_ref()
        .unwrap();

    assert!(!avatar_meta.is_avail);
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_non_owner_without_mutation() {
    //
    let mock = Mock::new();

    seed_user_avatar(&mock, "user-1", 3);

    let err = mark_avatar_uploaded::<MockContext, _>(
        (&mock,),
        token("user-2"),
        "user-1".into(),
        MarkUserAvatarUploadedInstr { image_ver: 3 },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    let avatar_meta = snapshot.objs["user_avatar"]["user-1"]
        .meta
        .as_ref()
        .unwrap();

    assert!(!avatar_meta.is_avail);
}

#[tokio::test]
async fn update_info_updates_user_and_member_nickname() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "password"),
    );

    mock.seed_member(member("member-1", "user-1", "Old", "team-1"));

    update_info(
        (&mock, &mock),
        token("user-1"),
        update_instr("user-1", "qid-new", "New"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users[0].qid, "qid-new");

    assert_eq!(snapshot.users[0].nickname, "New");

    assert_eq!(snapshot.members[0].user_nickname, "New");
}

#[tokio::test]
async fn update_info_rejects_non_owner_without_mutation() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "password"),
    );

    let err = update_info(
        (&mock, &mock),
        token("user-2"),
        update_instr("user-1", "qid-new", "New"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users[0].qid, "qid-1");

    assert_eq!(snapshot.users[0].nickname, "Old");
}

#[tokio::test]
async fn update_info_rolls_back_missing_user() {
    //
    let mock = Mock::new();

    let err = update_info(
        (&mock, &mock),
        token("user-1"),
        update_instr("user-1", "qid-new", "New"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().users.is_empty());
}

#[tokio::test]
async fn update_password_replaces_the_verified_password() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "old-password"),
    );

    update_password(
        (&mock, &mock),
        token("user-1"),
        "user-1".into(),
        update_password_instr("old-password", "new-password"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(
        UserComplex::verify_password(
            "new-password",
            &snapshot.credentials[0].password_hash,
        )
        .await
    );

    assert!(
        !UserComplex::verify_password(
            "old-password",
            &snapshot.credentials[0].password_hash,
        )
        .await
    );
}

#[tokio::test]
async fn update_password_rejects_an_incorrect_current_password() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "old-password"),
    );

    let err = update_password(
        (&mock, &mock),
        token("user-1"),
        "user-1".into(),
        update_password_instr("wrong-password", "new-password"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Auth);

    let snapshot = mock.snapshot();

    assert!(
        UserComplex::verify_password(
            "old-password",
            &snapshot.credentials[0].password_hash,
        )
        .await
    );
}
