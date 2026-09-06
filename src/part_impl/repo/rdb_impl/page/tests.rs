// page_roundtrip_uses_testcontainer(SetPageUnitCountMetrics, ListPageInfos)(positive): page repo persists and updates page counts in an isolated PostgreSQL container.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::PageManifestEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    ApplyPageManifest, GetPageInfo, ListEdittedDiffPageIds, ListFirstPageInfos,
    ListPageInfos, ListPageInfosExcluded, SetPageUnitCountMetrics,
    ShiftPageIndexesTemporary,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_chapter;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::{BaseError, ExpectedVariant};
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;

const PREFIX: &str = "rdb-test-page-domain-";

/// Verifies page roundtrip via testcontainers.
/// Verifies page roundtrip via testcontainers.
pub async fn page_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let unit_count_metrics = UnitCountMetrics {
        total: 2,
        translated: 1,
        proofread: 1,
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &SetPageUnitCountMetrics {
                id: &page_fixture.page_entry.id,
                count_metrics: unit_count_metrics,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(page_infos.len(), 1);

    assert_eq!(page_infos[0].total_unit_count, 2);

    assert_eq!(page_infos[0].translated_unit_count, 1);

    assert_eq!(page_infos[0].proofread_unit_count, 1);

    let retained_created_at = page_infos[0].created_at;

    let second_page_entry = PageManifestEntry {
        id: format!("{}page-later", PREFIX),
        chapter_id: page_fixture.chapter_entry.id.clone(),
        index: 1,
    };

    let new_page_info = nucl
        .coord(async |context| {
            //
            let page_infos = repo
                .step(
                    context,
                    &ApplyPageManifest {
                        entries: std::slice::from_ref(&second_page_entry),
                    },
                )
                .await?;

            page_infos.into_iter().next().ok_or_else(|| {
                BaseError::Unrecoverable {
                    message: "page creation returned no row".into(),
                }
            })
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(new_page_info.total_unit_count, 0);

    assert_eq!(new_page_info.translated_unit_count, 0);

    assert_eq!(new_page_info.proofread_unit_count, 0);

    let new_created_at = new_page_info.created_at;

    assert!(new_created_at <= time::OffsetDateTime::now_utc());

    let manifest_entries = vec![
        PageManifestEntry {
            id: second_page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 0,
        },
        PageManifestEntry {
            id: page_fixture.page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 1,
        },
    ];

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &ShiftPageIndexesTemporary {
                chapter_id: &page_fixture.chapter_entry.id,
            },
        )
        .await?;

        let page_infos = repo
            .step(
                context,
                &ApplyPageManifest {
                    entries: &manifest_entries,
                },
            )
            .await?;

        assert_eq!(page_infos.len(), 2);

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let reordered_page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(reordered_page_infos[0].id, second_page_entry.id);

    assert_eq!(reordered_page_infos[0].total_unit_count, 0);

    assert_eq!(reordered_page_infos[0].translated_unit_count, 0);

    assert_eq!(reordered_page_infos[0].proofread_unit_count, 0);

    assert_eq!(reordered_page_infos[0].created_at, new_created_at);

    assert_eq!(reordered_page_infos[1].id, page_fixture.page_entry.id);

    assert_eq!(reordered_page_infos[1].total_unit_count, 2);

    assert_eq!(reordered_page_infos[1].translated_unit_count, 1);

    assert_eq!(reordered_page_infos[1].proofread_unit_count, 1);

    assert_eq!(reordered_page_infos[1].created_at, retained_created_at);

    let chapter_ids = vec![page_fixture.chapter_entry.id.as_str()];

    let first_page_infos = repo
        .run(&ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        })
        .await
        .ok()
        .unwrap();

    let first_page_info = first_page_infos
        .iter()
        .find(|page_info| page_info.chapter_id == page_fixture.chapter_entry.id)
        .expect("first page info for the chapter");

    assert_eq!(first_page_info.id, second_page_entry.id);

    let rollback_entries = vec![
        PageManifestEntry {
            id: page_fixture.page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 0,
        },
        PageManifestEntry {
            id: second_page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 1,
        },
    ];

    let rollback_result = nucl
        .coord(async |context| {
            //
            repo.step(
                context,
                &ShiftPageIndexesTemporary {
                    chapter_id: &page_fixture.chapter_entry.id,
                },
            )
            .await?;

            repo.step(
                context,
                &ApplyPageManifest {
                    entries: &rollback_entries,
                },
            )
            .await?;

            Err::<(), BaseError>(BaseError::Unrecoverable {
                message: "force page-manifest rollback".into(),
            })
        })
        .await;

    assert!(rollback_result.is_err());

    let rolled_back_page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(rolled_back_page_infos[0].id, second_page_entry.id);

    assert_eq!(rolled_back_page_infos[0].created_at, new_created_at);

    assert_eq!(rolled_back_page_infos[0].total_unit_count, 0);

    assert_eq!(rolled_back_page_infos[0].translated_unit_count, 0);

    assert_eq!(rolled_back_page_infos[0].proofread_unit_count, 0);

    assert_eq!(rolled_back_page_infos[1].id, page_fixture.page_entry.id);

    assert_eq!(rolled_back_page_infos[1].created_at, retained_created_at);

    assert_eq!(rolled_back_page_infos[1].total_unit_count, 2);

    assert_eq!(rolled_back_page_infos[1].translated_unit_count, 1);

    assert_eq!(rolled_back_page_infos[1].proofread_unit_count, 1);

    let mut conn = shared.get().await.unwrap();

    diesel::update(
        t_chapter::table
            .filter(t_chapter::f_id.eq(&page_fixture.chapter_entry.id)),
    )
    .set(t_chapter::f_deleted_at.eq(Some(time::OffsetDateTime::now_utc())))
    .execute(&mut conn)
    .await
    .unwrap();

    drop(conn);

    let page_error = repo
        .run(&GetPageInfo {
            id: &page_fixture.page_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(
        page_error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));

    let overflow_entries = (2..=MAX_CHAPTER_PAGE_COUNT)
        .map(|index| PageManifestEntry {
            id: format!("{PREFIX}page-overflow-{index:03}"),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index,
        })
        .collect::<Vec<_>>();

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &ApplyPageManifest {
                entries: &overflow_entries,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let list_error = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(list_error, BaseError::Unrecoverable { .. }));

    let diff_error = repo
        .run(&ListEdittedDiffPageIds {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(diff_error, BaseError::Unrecoverable { .. }));

    let excluded_error = nucl
        .coord(async |context| {
            repo.step(
                context,
                &ListPageInfosExcluded {
                    chapter_id: &page_fixture.chapter_entry.id,
                },
            )
            .await
        })
        .await
        .map_err(BaseError::from)
        .err()
        .unwrap();

    assert!(matches!(excluded_error, BaseError::Unrecoverable { .. }));

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

/// Verifies original filename upsert, rollback, foreign keys, and page cascades.
pub async fn raw_ident_roundtrip_uses_testcontainer(shared: RdbCore) {
    use crate::model::write::page::PageRawIdentReplacement;
    use crate::part::repo::oper::page::{
        DeletePages, ListPageRawIdentInfos, SetPageRawIdents,
    };

    const RAW_PREFIX: &str = "rdb-test-page-raw-ident-";

    test_shared::reset(&shared, RAW_PREFIX).await;

    let fixture = test_shared::seed_page(&shared, RAW_PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let page_id = fixture.page_entry.id.as_str();

    let page_ids = [page_id];

    let mut created_at = None;

    for (replacement, expected) in [
        (Some("原稿 01.JPG".into()), Some("原稿 01.JPG")),
        (Some("原稿 01.JPG".into()), Some("原稿 01.JPG")),
        (Some("renamed.png".into()), Some("renamed.png")),
        (None, None),
        (None, None),
    ] {
        let replacements = [PageRawIdentReplacement {
            page_id: page_id.into(),
            raw_ident: replacement,
        }];

        nucl.coord(async |context| {
            repo.step(
                context,
                &SetPageRawIdents {
                    replacements: &replacements,
                },
            )
            .await
        })
        .await
        .unwrap();

        let infos = repo
            .run(&ListPageRawIdentInfos {
                page_ids: &page_ids,
            })
            .await
            .unwrap();

        assert_eq!(
            infos.as_slice().first().map(|info| info.raw_ident.as_str()),
            expected
        );

        if let Some(info) = infos.as_slice().first() {
            assert_eq!(
                *created_at.get_or_insert(info.created_at),
                info.created_at
            );
            assert!(info.updated_at >= info.created_at);
        }
    }

    let replacements = [
        PageRawIdentReplacement {
            page_id: page_id.into(),
            raw_ident: Some("rollback.png".into()),
        },
        PageRawIdentReplacement {
            page_id: "missing-page".into(),
            raw_ident: Some("invalid.png".into()),
        },
    ];

    assert!(
        nucl.coord(async |context| repo
            .step(
                context,
                &SetPageRawIdents {
                    replacements: &replacements
                }
            )
            .await)
            .await
            .is_err()
    );
    assert!(
        repo.run(&ListPageRawIdentInfos {
            page_ids: &page_ids
        })
        .await
        .unwrap()
        .is_empty()
    );

    let replacements = [PageRawIdentReplacement {
        page_id: page_id.into(),
        raw_ident: Some("delete.png".into()),
    }];

    nucl.coord(async |context| {
        repo.step(
            context,
            &SetPageRawIdents {
                replacements: &replacements,
            },
        )
        .await
    })
    .await
    .unwrap();

    let ids = [page_id.to_owned()];

    nucl.coord(async |context| {
        repo.step(context, &DeletePages::Ids { ids: &ids }).await
    })
    .await
    .unwrap();

    assert!(
        repo.run(&ListPageRawIdentInfos {
            page_ids: &page_ids
        })
        .await
        .unwrap()
        .is_empty()
    );
    assert!(
        repo.run(&ListPageRawIdentInfos { page_ids: &[] })
            .await
            .unwrap()
            .is_empty()
    );

    test_shared::cleanup(&shared, RAW_PREFIX).await.unwrap();
}
