// member_roundtrip_uses_testcontainer(MemberRepo)(positive): member repo creates, lists, fetches, and updates roles in an isolated PostgreSQL container.
// concurrent_admin_role_removals_are_serialized(MemberRepo)(concurrency): overlapping admin removals leave one admin and surface one retryable conflict.

use super::*;

use std::sync::Arc;
use std::time::Duration;

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperStep as _, Step as _};
use tokio::sync::{Semaphore, mpsc};

use poprako_rdb_core::RdbCore;

use crate::complex::member::MemberComplex;
use crate::model::read::spec::member::MemberListSpec;
use crate::model::write::member::{MemberEntry, MemberRoleRepl};
use crate::model::write::user::UserEntry;
use crate::part::nucl::{ReptRead, Serial};
use crate::part::repo::oper::member::{
    CreateMember, DeleteUserMemberships, GetMemberInfo, ListMemberInfos,
    UpdateMember,
};
use crate::part::repo::oper::user::{DeleteUser, GetUserInfo, UpdateUser};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::{t_member, t_team, t_user};
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-member-domain-";

/// Verifies member roundtrip via testcontainers.
/// Verifies member roundtrip via testcontainers.
pub async fn member_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let admin_role = RoleMask::from(RoleField::ADMIN);

    let member_role = RoleMask::from(RoleField::TRANSLATOR);

    let member_entry = MemberEntry {
        id: format!("{}member", PREFIX),
        user_id: team_fixture.user_entry.id.clone(),
        user_nickname: team_fixture.user_entry.nickname.clone(),
        team_id: team_fixture.team_entry.id.clone(),
        roles: admin_role,
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateMember {
                entry: &member_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    repo.run(&UpdateUser::TouchLastActive {
        id: &team_fixture.user_entry.id,
    })
    .await
    .ok()
    .unwrap();

    let touched_user_info = repo
        .run(&GetUserInfo {
            id: &team_fixture.user_entry.id,
        })
        .await
        .ok()
        .unwrap();

    let touched_member_info = repo
        .run(&GetMemberInfo {
            id: &member_entry.id,
            incls: &[],
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(
        touched_member_info.user_last_active_at,
        touched_user_info.last_active_at
    );

    let member_list_spec = MemberListSpec::Team {
        team_id: team_fixture.team_entry.id.clone(),
        fuzzy_nickname: Some("RDB".into()),
        role: Some(RoleField::ADMIN),
        incl_opt: vec![MemberInclOpt::User],
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let member_infos = repo
        .run(&ListMemberInfos::Spec {
            spec: &member_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(member_infos.len(), 1);

    assert_eq!(
        member_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_entry.id
    );

    let member_role_update = MemberRoleRepl {
        id: member_entry.id.clone(),
        roles: member_role,
    };

    nucl.coord(async |context| {
        //

        repo.step(
            context,
            &UpdateMember::Role {
                update: &member_role_update,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let member_info = repo
        .run(&GetMemberInfo {
            id: &member_entry.id,
            incls: &[MemberInclOpt::User],
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(member_info.roles, member_role);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

/// Verifies user cleanup removes memberships hidden by a team tombstone.
pub async fn user_cleanup_includes_tombstoned_team_memberships(
    shared: RdbCore,
) {
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let member_entry = MemberEntry {
        id: format!("{}tombstoned-member", PREFIX),
        user_id: team_fixture.user_entry.id.clone(),
        user_nickname: team_fixture.user_entry.nickname.clone(),
        team_id: team_fixture.team_entry.id.clone(),
        roles: RoleMask::from(RoleField::ADMIN),
    };

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<Serial>::new(shared.clone());

    nucl.coord(async |context| {
        repo.step(
            context,
            &CreateMember {
                entry: &member_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let mut conn = shared.get().await.unwrap();

    diesel::update(
        t_team::table.filter(t_team::f_id.eq(&team_fixture.team_entry.id)),
    )
    .set(t_team::f_deleted_at.eq(Some(time::OffsetDateTime::now_utc())))
    .execute(&mut conn)
    .await
    .unwrap();

    drop(conn);

    nucl.coord(async |context| {
        DeleteUserMemberships {
            user_id: &team_fixture.user_entry.id,
        }
        .step_on(&repo, context)
        .await?;

        DeleteUser {
            id: &team_fixture.user_entry.id,
        }
        .step_on(&repo, context)
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let mut conn = shared.get().await.unwrap();

    let member_count = t_member::table
        .filter(t_member::f_id.eq(&member_entry.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    let user_count = t_user::table
        .filter(t_user::f_id.eq(&team_fixture.user_entry.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(member_count, 0);
    assert_eq!(user_count, 0);

    drop(conn);

    test_shared::cleanup(&shared, PREFIX).await.unwrap();
}

/// Verifies concurrent admin removals preserve one team administrator.
pub async fn concurrent_admin_role_removals_are_serialized(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let second_user_entry = UserEntry {
        id: format!("{}second-user", PREFIX),
        qid: format!("{}second-qid", PREFIX),
        nickname: "RDB second admin".into(),
        password_hash: "password-hash".into(),
    };

    test_shared::create_user(&shared, &second_user_entry).await;

    let first_member_entry = MemberEntry {
        id: format!("{}first-admin-member", PREFIX),
        user_id: team_fixture.user_entry.id.clone(),
        user_nickname: team_fixture.user_entry.nickname.clone(),
        team_id: team_fixture.team_entry.id.clone(),
        roles: RoleMask::from(RoleField::ADMIN),
    };

    let second_member_entry = MemberEntry {
        id: format!("{}second-admin-member", PREFIX),
        user_id: second_user_entry.id,
        user_nickname: second_user_entry.nickname,
        team_id: team_fixture.team_entry.id.clone(),
        roles: RoleMask::from(RoleField::ADMIN),
    };

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateMember {
                entry: &first_member_entry,
            },
        )
        .await?;

        repo.step(
            context,
            &CreateMember {
                entry: &second_member_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let (ready_tx, mut ready_rx) = mpsc::channel(2);

    let write_permits = Arc::new(Semaphore::new(0));

    let first_removal = tokio::spawn(remove_admin_role_after_read(
        shared.clone(),
        team_fixture.team_entry.id.clone(),
        first_member_entry.id.clone(),
        ready_tx.clone(),
        Arc::clone(&write_permits),
    ));

    let second_removal = tokio::spawn(remove_admin_role_after_read(
        shared.clone(),
        team_fixture.team_entry.id.clone(),
        second_member_entry.id.clone(),
        ready_tx,
        Arc::clone(&write_permits),
    ));

    tokio::time::timeout(Duration::from_secs(10), async {
        ready_rx
            .recv()
            .await
            .expect("first transaction must report its completed read");

        ready_rx
            .recv()
            .await
            .expect("second transaction must report its completed read");
    })
    .await
    .expect("both transactions must complete their reads");

    write_permits.add_permits(2);

    let (first_outcome, second_outcome) =
        tokio::join!(first_removal, second_removal);

    let first_outcome =
        first_outcome.expect("first transaction task must join");

    let second_outcome =
        second_outcome.expect("second transaction task must join");

    let outcomes = [first_outcome, second_outcome];

    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);

    let error = outcomes
        .into_iter()
        .find_map(Result::err)
        .expect("one concurrent admin removal must fail");

    assert!(matches!(error, BaseError::Retryable { .. }));

    let member_list_spec = MemberListSpec::Team {
        team_id: team_fixture.team_entry.id,
        fuzzy_nickname: None,
        role: Some(RoleField::ADMIN),
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let admin_member_infos = repo
        .run(&ListMemberInfos::Spec {
            spec: &member_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(admin_member_infos.len(), 1);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

// Removes one admin role after the test releases both completed reads.
async fn remove_admin_role_after_read(
    shared: RdbCore,
    team_id: String,
    member_id: String,
    ready_tx: mpsc::Sender<()>,
    write_permits: Arc<Semaphore>,
) -> Result<(), BaseError> {
    //
    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<Serial>::new(shared);

    nucl.coord(async move |context| {
        //
        let member_list_spec = MemberListSpec::Team {
            team_id,
            fuzzy_nickname: None,
            role: None,
            incl_opt: Vec::new(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        };

        let member_infos = ListMemberInfos::Spec {
            spec: &member_list_spec,
        }
        .step_on(&repo, context)
        .await?;

        let subject_member_info = member_infos
            .iter()
            .find(|member_info| member_info.id == member_id)
            .expect("subject admin member must exist");

        let roles = RoleMask::from(RoleField::REVIEWER);

        assert!(MemberComplex::team_has_admin_after_role_update(
            &member_infos,
            subject_member_info,
            roles,
        ));

        ready_tx
            .send(())
            .await
            .expect("test read coordinator must remain available");

        let write_permit = write_permits
            .acquire()
            .await
            .expect("test write coordinator must remain available");

        write_permit.forget();

        let member_role_update = MemberRoleRepl {
            id: member_id,
            roles,
        };

        UpdateMember::Role {
            update: &member_role_update,
        }
        .step_on(&repo, context)
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .map_err(Into::into)
}
