// import_translation(import)(positive): proofreader imports real LabelPlus material transactionally.
// import_translation(import)(negative): page-count mismatch rejects import and leaves units and counters unchanged.

use super::*;

use time::OffsetDateTime;

use crate::data::instr::chapter_port::ImportChapterTranslationInstr;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::unit::UnitCoord;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter_port::{
    ChapterTranslationImportMode, TranslationFormat,
};
use crate::value::role::{RoleField, RoleMask};

// LabelPlus fixture content used for chapter import integration tests.
const LABEL_PLUS_MATERIAL: &str =
    include_str!("../../../../tests/materials/translations.lp.txt");

// Small LabelPlus fixture containing one populated page and one empty page.
const LABEL_PLUS_WITH_EMPTY_PAGE: &str = concat!(
    "1,0\n",
    "-\n",
    "框内\n",
    "框外\n",
    "-\n",
    "note\n",
    ">>>>>>>>[000.jpg]<<<<<<<<\n",
    "----------------[1]----------------[0.1,0.2,1]\n",
    "new text\n",
    ">>>>>>>>[001.jpg]<<<<<<<<\n",
);

// Small LabelPlus fixture containing one empty page followed by one populated page.
const LABEL_PLUS_WITH_POPULATED_SECOND_PAGE: &str = concat!(
    "1,0\n",
    "-\n",
    "框内\n",
    "框外\n",
    "-\n",
    "note\n",
    ">>>>>>>>[000.jpg]<<<<<<<<\n",
    ">>>>>>>>[001.jpg]<<<<<<<<\n",
    "----------------[1]----------------[0.1,0.2,1]\n",
    "new second-page text\n",
);

// Build a token fixture for chapter import authorization.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

// Build comic fixture referenced by imported chapter.
fn comic(id: &str) -> ComicInfo {
    //
    // Compose a stable comic fixture used by chapter/import perm checks.
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: "workset-1".into(),
        index: 0,
        title: "Pop Comic".into(),
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

// Build workset fixture for chapter import scenarios.
fn workset(id: &str) -> WorksetInfo {
    //
    // Compose a stable workset fixture for assignment and chapter binding.
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: "team-1".into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        created_at: time,
        updated_at: time,
    }
}

// Build chapter fixture with provided unit and proofread counters.
fn chapter(
    page_count: usize,
    total_unit_count: usize,
    proofread_unit_count: usize,
) -> ChapterInfo {
    //
    // Build a chapter fixture with configurable unit counters.
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        is_pinned: true,
        index: 3,
        subtitle: "Arrival".into(),
        page_count,
        total_unit_count,
        translated_unit_count: total_unit_count,
        proofread_unit_count,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        comic: None,
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

// Build assignment fixture for import perm checks.
fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    // Create a member assignment record for import perm scenarios.
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

// Build page fixture and image metadata for import material.
fn page(
    id: &str,
    index: usize,
    total_unit_count: usize,
    proofread_unit_count: usize,
) -> PageInfo {
    //
    // Build one page fixture and pre-seed image metadata.
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        total_unit_count,
        translated_unit_count: total_unit_count,
        proofread_unit_count,
        created_at: time,
        updated_at: time,
    }
}

// Build unit fixture with translator metadata and optional translated content.
fn unit(id: &str, page_id: &str, _index: usize, text: &str) -> UnitInfo {
    //
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: page_id.into(),
        next_id: None,
        is_bubble: true,
        is_proofread: false,
        coord: UnitCoord {
            x_coord: 0.25,
            y_coord: 0.75,
        },
        translated_text: Some(text.into()),
        last_translator_id: Some("translator-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
        hidden_at: None,
        created_at: time,
        updated_at: time,
    }
}

// Seed workset/comic/chapter/assignment baseline for chapter import.
fn seed_base(
    mock: &Mock,
    page_count: usize,
    total_unit_count: usize,
    proofread_unit_count: usize,
) {
    //
    // Seed the minimal base graph (workset, comic, chapter, assignment).
    mock.seed_workset(workset("workset-1"));

    mock.seed_comic(comic("comic-1"));

    mock.seed_chapter(chapter(
        page_count,
        total_unit_count,
        proofread_unit_count,
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));
}

// Seed a full set of placeholder pages used by import material input.
fn seed_material_pages(mock: &Mock) {
    for index in 0..9 {
        mock.seed_page(page(&format!("page-{}", index + 1), index, 0, 0));
    }
}

#[tokio::test]
async fn import_label_plus_material_updates_units_and_counts() {
    //
    // Verify import applies unit text/proofread content and updates chapter counters.
    let mock = Mock::new();

    seed_base(&mock, 9, 0, 0);

    seed_material_pages(&mock);

    let imported = import_translation(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Overwrite.into(),
            content: LABEL_PLUS_MATERIAL.into(),
        },
        "chapter-1".into(),
    )
    .await;

    let imported = match imported {
        //
        // Convert transport errors into explicit panics in this happy-path unit test.
        Ok(imported) => imported,

        Err(_) => panic!("expected import success"),
    };

    let snapshot = mock.snapshot();

    let first_unit = snapshot
        .units
        .iter()
        .find(|unit_info| unit_info.page_id == "page-1")
        .unwrap();

    let last_unit = snapshot
        .units
        .iter()
        .rfind(|unit_info| unit_info.page_id == "page-9")
        .unwrap();

    let chapter_info = snapshot
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == "chapter-1")
        .unwrap();

    assert_eq!(imported.imported_page_count, 9);

    assert_eq!(imported.imported_unit_count, 65);

    assert_eq!(snapshot.units.len(), 65);

    assert_eq!(first_unit.proofread_text, Some("喂 游斗哥".into()));

    assert_eq!(first_unit.last_proofreader_id, Some("user-1".into()));

    assert_eq!(
        last_unit.proofread_text,
        Some("哥哥对次女可爱的\n小心思毫无察觉".into())
    );

    assert_eq!(chapter_info.total_unit_count, 65);

    assert_eq!(chapter_info.proofread_unit_count, 65);

    let imported_again = import_translation(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Overwrite.into(),
            content: LABEL_PLUS_MATERIAL.into(),
        },
        "chapter-1".into(),
    )
    .await
    .expect("repeated import should replace visible units");

    let repeated_snapshot = mock.snapshot();
    let visible_unit_count = repeated_snapshot
        .units
        .iter()
        .filter(|unit_info| unit_info.hidden_at.is_none())
        .count();

    assert_eq!(imported_again.imported_unit_count, 65);
    assert_eq!(visible_unit_count, 65);
    assert_eq!(repeated_snapshot.units.len(), 130);
    assert!(
        repeated_snapshot
            .units
            .iter()
            .filter(|unit_info| unit_info.hidden_at.is_some())
            .all(|unit_info| unit_info.last_proofreader_id.is_some())
    );
}

#[tokio::test]
async fn import_rejects_page_count_mismatch_without_mutation() {
    //
    // Verify mismatched page counts fail atomically and avoid mutating seeded data.
    let mock = Mock::new();

    seed_base(&mock, 2, 1, 0);

    mock.seed_page(page("page-1", 0, 1, 0));

    mock.seed_page(page("page-2", 1, 0, 0));

    mock.seed_unit(unit("unit-a", "page-1", 0, "old"));

    let err = import_translation(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Overwrite.into(),
            content: LABEL_PLUS_MATERIAL.into(),
        },
        "chapter-1".into(),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(snapshot.units.len(), 1);

    assert_eq!(snapshot.units[0].translated_text, Some("old".into()));

    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
}

#[tokio::test]
async fn import_replaces_units_and_clears_empty_pages() {
    let mock = Mock::new();

    seed_base(&mock, 2, 2, 0);
    mock.seed_page(page("page-1", 0, 1, 0));
    mock.seed_page(page("page-2", 1, 1, 0));
    mock.seed_unit(unit("unit-a", "page-1", 0, "old page one"));
    mock.seed_unit(unit("unit-b", "page-2", 0, "old page two"));

    let imported = import_translation(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Overwrite.into(),
            content: LABEL_PLUS_WITH_EMPTY_PAGE.into(),
        },
        "chapter-1".into(),
    )
    .await
    .expect("import with an empty page should succeed");

    let snapshot = mock.snapshot();
    let visible_page_one = snapshot
        .units
        .iter()
        .filter(|unit_info| {
            unit_info.page_id == "page-1" && unit_info.hidden_at.is_none()
        })
        .collect::<Vec<_>>();
    let visible_page_two = snapshot
        .units
        .iter()
        .filter(|unit_info| {
            unit_info.page_id == "page-2" && unit_info.hidden_at.is_none()
        })
        .count();

    assert_eq!(imported.imported_page_count, 2);
    assert_eq!(imported.imported_unit_count, 1);
    assert_eq!(visible_page_one.len(), 1);
    assert_eq!(visible_page_one[0].proofread_text, Some("new text".into()));
    assert_eq!(visible_page_two, 0);
    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
    assert_eq!(snapshot.chapters[0].proofread_unit_count, 1);
}

#[tokio::test]
async fn import_keep_preserves_visible_page_units() {
    let mock = Mock::new();

    seed_base(&mock, 2, 2, 0);
    mock.seed_page(page("page-1", 0, 1, 0));
    mock.seed_page(page("page-2", 1, 1, 0));
    mock.seed_unit(unit("unit-a", "page-1", 0, "old page one"));
    mock.seed_unit(unit("unit-b", "page-2", 0, "old page two"));

    let imported = import_translation(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Keep.into(),
            content: LABEL_PLUS_WITH_EMPTY_PAGE.into(),
        },
        "chapter-1".into(),
    )
    .await
    .expect("keep import should preserve populated pages");

    let snapshot = mock.snapshot();
    let visible_units = snapshot
        .units
        .iter()
        .filter(|unit_info| unit_info.hidden_at.is_none())
        .collect::<Vec<_>>();

    assert_eq!(imported.imported_page_count, 0);
    assert_eq!(imported.imported_unit_count, 0);
    assert_eq!(visible_units.len(), 2);
    assert_eq!(
        visible_units[0].translated_text,
        Some("old page one".into())
    );
    assert_eq!(
        visible_units[1].translated_text,
        Some("old page two".into())
    );
    assert_eq!(snapshot.chapters[0].total_unit_count, 2);

    assert!(matches!(
        snapshot.chapter_workflow_records[0].payload,
        ChapterWorkflowRecordPayload::TranslationImported {
            imported_page_count: 0,
            imported_unit_count: 0,
            ..
        }
    ));
}

#[tokio::test]
async fn import_keep_reuses_page_with_only_hidden_units() {
    let mock = Mock::new();

    seed_base(&mock, 2, 0, 0);
    mock.seed_page(page("page-1", 0, 0, 0));
    mock.seed_page(page("page-2", 1, 0, 0));

    let mut hidden_unit = unit("unit-hidden", "page-2", 0, "historical");
    hidden_unit.hidden_at = Some(OffsetDateTime::now_utc());
    mock.seed_unit(hidden_unit);

    let imported = import_translation(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Keep.into(),
            content: LABEL_PLUS_WITH_POPULATED_SECOND_PAGE.into(),
        },
        "chapter-1".into(),
    )
    .await
    .expect("keep import should reuse pages with only hidden Units");

    let snapshot = mock.snapshot();
    let visible_page_two = snapshot
        .units
        .iter()
        .filter(|unit_info| {
            unit_info.page_id == "page-2" && unit_info.hidden_at.is_none()
        })
        .collect::<Vec<_>>();

    assert_eq!(imported.imported_page_count, 1);
    assert_eq!(imported.imported_unit_count, 1);
    assert_eq!(visible_page_two.len(), 1);
    assert_eq!(
        visible_page_two[0].proofread_text,
        Some("new second-page text".into())
    );
    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
    assert!(
        snapshot
            .units
            .iter()
            .any(|unit_info| unit_info.id == "unit-hidden"
                && unit_info.hidden_at.is_some())
    );
}
