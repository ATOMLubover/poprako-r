// export_translation(export)(positive): assignee atomically exports both formats from one loaded chapter snapshot, records one export, and triggers typeset/redraw once.

use super::*;

use time::OffsetDateTime;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::unit::UnitCoord;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_port::ExportFormatSpec;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn comic(id: &str) -> ComicInfo {
    //
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

fn workset(id: &str) -> WorksetInfo {
    //
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

fn chapter(id: &str) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: "comic-1".into(),
        is_pinned: true,
        index: 3,
        subtitle: "Arrival".into(),
        page_count: 2,
        total_unit_count: 2,
        translated_unit_count: 2,
        proofread_unit_count: 1,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        comic: None,
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
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

fn member(user_id: &str) -> MemberInfo {
    //
    MemberInfo {
        id: format!("member-{user_id}"),

        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),

        team_id: "team-1".into(),

        user: None,
        team: None,

        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn page(id: &str, index: usize, _image_uploaded: bool) -> PageInfo {
    //
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        total_unit_count: 1,
        translated_unit_count: 1,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn unit(
    id: &str,
    page_id: &str,
    next_id: Option<&str>,
    text: &str,
    proofread_text: Option<&str>,
) -> UnitInfo {
    //
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: page_id.into(),
        next_id: next_id.map(str::to_string),
        is_bubble: true,
        is_proofread: proofread_text.is_some(),
        coord: UnitCoord {
            x_coord: 0.25,
            y_coord: 0.75,
        },
        translated_text: Some(text.into()),
        last_translator_id: Some("translator-1".into()),
        proofread_text: proofread_text.map(Into::into),
        last_proofreader_id: Some("proofreader-1".into()),
        hidden_at: None,
        created_at: time,
        updated_at: time,
    }
}

fn seed_scope(mock: &Mock) {
    //
    mock.seed_workset(workset("workset-1"));

    mock.seed_comic(comic("comic-1"));

    mock.seed_chapter(chapter("chapter-1"));

    mock.seed_page(page("page-1", 0, true));

    mock.seed_page_image_obj("page-1", "png");

    mock.seed_page(page("page-2", 1, false));

    mock.seed_page_image_obj("page-2", "png");

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));
}

#[tokio::test]
async fn export_returns_both_formats_and_records_one_export() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_unit(unit("unit-b", "page-1", None, "beta", None));

    mock.seed_unit(unit(
        "unit-a",
        "page-1",
        Some("unit-b"),
        "alpha",
        Some("alpha proof"),
    ));

    let exported = export_translation(
        (&mock, &mock, &mock),
        token("user-1"),
        "chapter-1".into(),
        ExportFormatSpec::BOTH,
        false,
    )
    .await;

    let exported = match exported {
        //
        Ok(exported) => exported,

        Err(_) => panic!("expected export success"),
    };

    let poprako = exported.poprako.unwrap();

    assert_eq!(poprako.chapter_id, "chapter-1");

    assert_eq!(poprako.chapter_index, 3);

    assert_eq!(poprako.chapter_subtitle, Some("Arrival".into()));

    assert_eq!(poprako.comic_title, "Pop Comic");

    assert_eq!(poprako.pages.len(), 2);

    assert_eq!(poprako.pages[0].units.len(), 2);

    assert_eq!(poprako.pages[0].units[0].unit_id, "unit-a");

    assert_eq!(
        poprako.pages[0].units[0].proofread_text,
        Some("alpha proof".into())
    );

    let label_plus = exported.label_plus.unwrap();

    assert!(label_plus.contains("Exported by PopRaKo Web"));

    assert!(label_plus.contains(">>>>>>>>[000.png]<<<<<<<<"));

    assert!(
        label_plus
            .contains("----------------[1]----------------[0.2500,0.7500,1]")
    );

    assert!(label_plus.contains("alpha proof"));

    assert!(
        mock.snapshot().chapters[0]
            .stages
            .has_phase(Stage::TypesetRedraw, StagePhase::Active,)
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.chapter_workflow_records.len(), 2);

    assert!(matches!(
        &snapshot.chapter_workflow_records[0].payload,
        ChapterWorkflowRecordPayload::TranslationExported { formats }
            if *formats == ExportFormatSpec::BOTH
    ));

    assert!(matches!(
        &snapshot.chapter_workflow_records[1].payload,
        ChapterWorkflowRecordPayload::StageTransitioned {
            stage: Stage::TypesetRedraw,
            previous_phase: StagePhase::Pending,
            next_phase: StagePhase::Active,
            ..
        }
    ));
}

#[tokio::test]
async fn export_by_unassigned_team_member_does_not_start_typeset_redraw() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_member(member("team-member"));

    let exported = export_translation(
        (&mock, &mock, &mock),
        token("team-member"),
        "chapter-1".into(),
        ExportFormatSpec::POPRAKO,
        false,
    )
    .await;

    assert!(exported.is_ok());

    let snapshot = mock.snapshot();

    assert!(
        snapshot.chapters[0]
            .stages
            .has_phase(Stage::TypesetRedraw, StagePhase::Pending)
    );

    assert_eq!(snapshot.chapter_workflow_records.len(), 1);

    assert!(matches!(
        &snapshot.chapter_workflow_records[0].payload,
        ChapterWorkflowRecordPayload::TranslationExported { formats }
            if *formats == ExportFormatSpec::POPRAKO
    ));
}

#[tokio::test]
async fn export_raw_ident_is_opt_in_with_per_page_fallback_and_duplicate_names()
{
    use poprako_orchestra::{Nucl as _, OperStep as _};

    use crate::model::write::page::PageRawIdentReplacement;
    use crate::part::repo::oper::page::SetPageRawIdents;

    let mock = Mock::new();

    seed_scope(&mock);

    let replacements = [PageRawIdentReplacement {
        page_id: "page-1".into(),
        raw_ident: Some("原稿 01.JPG".into()),
    }];

    mock.coord(async |context| {
        SetPageRawIdents {
            replacements: &replacements,
        }
        .step_on(&mock, context)
        .await
    })
    .await
    .unwrap();

    let default_export = export_translation(
        (&mock, &mock, &mock),
        token("user-1"),
        "chapter-1".into(),
        ExportFormatSpec::BOTH,
        false,
    )
    .await
    .unwrap();

    let raw_export = export_translation(
        (&mock, &mock, &mock),
        token("user-1"),
        "chapter-1".into(),
        ExportFormatSpec::BOTH,
        true,
    )
    .await
    .unwrap();

    assert!(
        default_export
            .label_plus
            .unwrap()
            .contains(">>>>>>>>[000.png]<<<<<<<<")
    );

    let label_plus = raw_export.label_plus.unwrap();

    assert!(label_plus.contains(">>>>>>>>[原稿 01.JPG]<<<<<<<<"));
    assert!(label_plus.contains(">>>>>>>>[001.png]<<<<<<<<"));
    assert_eq!(
        serde_json::to_value(default_export.poprako).unwrap(),
        serde_json::to_value(raw_export.poprako).unwrap()
    );

    let replacements = [PageRawIdentReplacement {
        page_id: "page-2".into(),
        raw_ident: Some("原稿 01.JPG".into()),
    }];

    mock.coord(async |context| {
        SetPageRawIdents {
            replacements: &replacements,
        }
        .step_on(&mock, context)
        .await
    })
    .await
    .unwrap();

    let duplicate_export = export_translation(
        (&mock, &mock, &mock),
        token("user-1"),
        "chapter-1".into(),
        ExportFormatSpec::LABEL_PLUS,
        true,
    )
    .await
    .unwrap();

    assert_eq!(
        duplicate_export
            .label_plus
            .unwrap()
            .matches(">>>>>>>>[原稿 01.JPG]<<<<<<<<")
            .count(),
        2
    );
    assert!(duplicate_export.poprako.is_none());

    let native_export = export_translation(
        (&mock, &mock, &mock),
        token("user-1"),
        "chapter-1".into(),
        ExportFormatSpec::POPRAKO,
        true,
    )
    .await
    .unwrap();

    assert!(native_export.label_plus.is_none());
    assert!(native_export.poprako.is_some());
}
