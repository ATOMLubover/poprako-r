// team_roundtrip_uses_testcontainer(ListTeamInfos, UpdateTeam, GetTeamInfo)(positive): team repo persists, lists, and updates a team in an isolated PostgreSQL container.
// resolve_team_id_uses_testcontainer(ResolveTeamId)(positive/negative): comic and chapter ownership resolves in and out of transactions while missing roots retain resource-specific errors.

use poprako_orchestra::{Nucl as _, Run, Step};

use poprako_rdb_core::RdbCore;
use poprako_util::i18n::trl;

use crate::model::read::spec::team::TeamListSpec;
use crate::model::write::team::TeamRepl;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::team::{
    GetTeamInfo, ListTeamInfos, ResolveTeamId, UpdateTeam,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::{BaseError, ExpectedVariant};

const PREFIX: &str = "rdb-test-team-domain-";

const RESOLVE_PREFIX: &str = "rdb-test-team-resolve-";

fn assert_expected(error: BaseError, message_key: &str) {
    let BaseError::Expected { variant, message } = error else {
        panic!("expected client-visible resource error");
    };

    assert!(matches!(variant, ExpectedVariant::Args));

    assert_eq!(message, trl(message_key));
}

/// Verifies team roundtrip via testcontainers.
/// Verifies team roundtrip via testcontainers.
pub async fn team_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let team_info_list_spec = TeamListSpec {
        user_id: None,
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let team_infos = repo
        .run(&ListTeamInfos {
            spec: &team_info_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert!(
        team_infos
            .iter()
            .any(|team_info| team_info.id == team_fixture.team_entry.id)
    );

    let team_repl = TeamRepl {
        id: team_fixture.team_entry.id.clone(),
        name: "RDB Team Updated".into(),
        description: "updated".into(),
    };

    repo.run(&UpdateTeam { repl: &team_repl })
        .await
        .ok()
        .unwrap();

    let team_info = repo
        .run(&GetTeamInfo {
            id: &team_fixture.team_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(team_info.name, "RDB Team Updated");

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

/// Verifies comic and chapter ownership projection in both repository modes.
pub async fn resolve_team_id_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, RESOLVE_PREFIX).await;

    let chapter_fixture =
        test_shared::seed_chapter(&shared, RESOLVE_PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let comic_team_id = repo
        .run(&ResolveTeamId::Comic {
            id: &chapter_fixture.comic_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(comic_team_id, chapter_fixture.team_entry.id);

    let chapter_team_id = repo
        .run(&ResolveTeamId::Chapter {
            id: &chapter_fixture.chapter_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(chapter_team_id, chapter_fixture.team_entry.id);

    let missing_comic_error = repo
        .run(&ResolveTeamId::Comic {
            id: "missing-comic",
        })
        .await
        .err()
        .unwrap();

    assert_expected(missing_comic_error, "error-comic-not-found");

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    nucl.coord(async |context| {
        //
        let chapter_team_id = repo
            .step(
                context,
                &ResolveTeamId::Chapter {
                    id: &chapter_fixture.chapter_entry.id,
                },
            )
            .await?;

        assert_eq!(chapter_team_id, chapter_fixture.team_entry.id);

        let comic_team_id = repo
            .step(
                context,
                &ResolveTeamId::Comic {
                    id: &chapter_fixture.comic_entry.id,
                },
            )
            .await?;

        assert_eq!(comic_team_id, chapter_fixture.team_entry.id);

        let missing_chapter_error = repo
            .step(
                context,
                &ResolveTeamId::Chapter {
                    id: "missing-chapter",
                },
            )
            .await
            .err()
            .unwrap();

        assert_expected(missing_chapter_error, "error-chapter-not-found");

        let missing_comic_error = repo
            .step(
                context,
                &ResolveTeamId::Comic {
                    id: "missing-comic",
                },
            )
            .await
            .err()
            .unwrap();

        assert_expected(missing_comic_error, "error-comic-not-found");

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    test_shared::cleanup(&shared, RESOLVE_PREFIX)
        .await
        .ok()
        .unwrap();

    test_shared::assert_no_leftovers(&shared, RESOLVE_PREFIX)
        .await
        .ok()
        .unwrap();
}
