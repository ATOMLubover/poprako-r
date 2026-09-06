// Integration fixture for real LabelPlus import parsing.
// parse_poprako(parse_poprako)(positive): preserves zero-based PopRaKo indexes.
// build_unit_create(build_unit_create)(positive): import always produces a complete Unit Create.

use super::*;

use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::data::view::page_port::PageTranslationPortView;
use crate::data::view::unit_port::UnitTranslationPortView;
use crate::model::artifact::translation_import::UnitTranslationImportSource;

// LabelPlus import fixture used by chapter import parse tests.
const LABEL_PLUS_MATERIAL: &str =
    include_str!("../../../../tests/materials/translations.lp.txt");

#[test]
fn parse_label_plus_parses_real_material() {
    //
    let pages =
        ChapterTranslationImportComplex::parse_label_plus(LABEL_PLUS_MATERIAL);

    let pages = match pages {
        //
        Ok(pages) => pages,

        Err(_) => panic!("expected LabelPlus material parse success"),
    };

    assert_eq!(pages.len(), 9);

    assert_eq!(pages[0].units.len(), 10);

    assert_eq!(pages[8].units.len(), 9);

    assert_eq!(pages[0].units[0].index, 0);

    assert!(matches!(
        &pages[0].units[0].source,
        UnitTranslationImportSource::LabelPlus { text }
            if text.as_deref() == Some("喂 游斗哥")
    ));

    assert!(matches!(
        &pages[8].units[8].source,
        UnitTranslationImportSource::LabelPlus { text }
            if text.as_deref() == Some("哥哥对次女可爱的\n小心思毫无察觉")
    ));
}

#[test]
fn parse_poprako_preserves_zero_based_indexes() {
    //
    let pages = ChapterTranslationImportComplex::parse_poprako(
        r#"{
            "chapter_id": "chapter-1",
            "chapter_index": 0,
            "chapter_subtitle": null,
            "comic_id": "comic-1",
            "comic_title": "title",
            "pages": [
                {
                    "page_id": "page-1",
                    "page_index": 0,
                    "units": [
                        {
                            "unit_id": "unit-1",
                            "unit_index": 7,
                            "page_id": "page-1",
                            "page_index": 0,
                            "x_coord": 0.1,
                            "y_coord": 0.2,
                            "is_bubble": true,
                            "translated_text": "translated",
                            "translator_id": null,
                            "is_proofread": false,
                            "proofread_text": null,
                            "proofreader_id": null
                        }
                    ]
                }
            ]
        }"#,
    );

    let pages = match pages {
        //
        Ok(pages) => pages,

        Err(_) => panic!("expected PopRaKo parse success"),
    };

    assert_eq!(pages[0].units[0].index, 7);

    assert!(matches!(
        &pages[0].units[0].source,
        UnitTranslationImportSource::PopRaKo {
            translated_text: Some(text),
            proofread_text: None,
            is_proofread: false,
        } if text == "translated"
    ));
}

#[test]
fn build_unit_create_produces_a_complete_create() {
    //
    let pages =
        ChapterTranslationImportComplex::parse_label_plus(LABEL_PLUS_MATERIAL)
            .unwrap();

    let edit = ChapterTranslationImportComplex::build_unit_create(
        &pages[0].units[0],
        "unit-new".to_string(),
        "proofreader-1",
        false,
        true,
    );

    assert!(matches!(
        edit,
        UnitEdit::Create {
            id,
            next_id: None,
            is_bubble: true,
            revision: Some(_),
            ..
        } if id == "unit-new"
    ));
}

#[test]
fn parse_label_plus_accepts_bom_crlf_and_structure_trailing_whitespace() {
    let content = concat!(
        "\u{feff}1,0\r\n",
        "- \t\r\n",
        "框内\r\n",
        "框外\r\n",
        "-\t\r\n",
        "note\r\n",
        ">>>>>>>>[000.jpg]<<<<<<<< \t\r\n",
        "----------------[1]----------------[0.1,0.2,1] \t\r\n",
        " translated  \r\n",
    );

    let pages =
        ChapterTranslationImportComplex::parse_label_plus(content).unwrap();

    assert_eq!(pages.len(), 1);
    assert!(matches!(
        &pages[0].units[0].source,
        UnitTranslationImportSource::LabelPlus { text }
            if text.as_deref() == Some(" translated  ")
    ));
}

#[test]
fn parse_poprako_rejects_duplicate_page_indexes() {
    let content = r#"{
        "chapter_id": "chapter-1",
        "chapter_index": 0,
        "chapter_subtitle": null,
        "comic_id": "comic-1",
        "comic_title": "title",
        "pages": [
            { "page_id": "page-1", "page_index": 0, "units": [] },
            { "page_id": "page-2", "page_index": 0, "units": [] }
        ]
    }"#;

    assert!(ChapterTranslationImportComplex::parse_poprako(content).is_err());
}

#[test]
fn parse_poprako_roundtrips_shared_view_and_sorts_indexes() {
    let document = ChapterTranslationPortView {
        chapter_id: "source-chapter".into(),
        chapter_index: 4,
        chapter_subtitle: Some("source subtitle".into()),
        comic_id: "source-comic".into(),
        comic_title: "source title".into(),
        pages: vec![
            PageTranslationPortView {
                page_id: "source-page-1".into(),
                page_index: 1,
                units: vec![UnitTranslationPortView {
                    unit_id: "source-unit-1".into(),
                    unit_index: 1,
                    page_id: "source-page-1".into(),
                    page_index: 1,
                    x_coord: 0.4,
                    y_coord: 0.5,
                    is_bubble: false,
                    translated_text: Some("second".into()),
                    translator_id: Some("source-translator".into()),
                    is_proofread: false,
                    proofread_text: None,
                    proofreader_id: None,
                }],
            },
            PageTranslationPortView {
                page_id: "source-page-0".into(),
                page_index: 0,
                units: vec![
                    UnitTranslationPortView {
                        unit_id: "source-unit-1".into(),
                        unit_index: 1,
                        page_id: "source-page-0".into(),
                        page_index: 0,
                        x_coord: 0.2,
                        y_coord: 0.3,
                        is_bubble: true,
                        translated_text: Some("first".into()),
                        translator_id: None,
                        is_proofread: false,
                        proofread_text: None,
                        proofreader_id: None,
                    },
                    UnitTranslationPortView {
                        unit_id: "source-unit-2".into(),
                        unit_index: 0,
                        page_id: "source-page-0".into(),
                        page_index: 0,
                        x_coord: 0.1,
                        y_coord: 0.2,
                        is_bubble: true,
                        translated_text: None,
                        translator_id: None,
                        is_proofread: false,
                        proofread_text: None,
                        proofreader_id: None,
                    },
                ],
            },
        ],
    };

    let content = serde_json::to_string(&document).unwrap();
    let pages =
        ChapterTranslationImportComplex::parse_poprako(&content).unwrap();

    assert_eq!(pages[0].page_index, 0);
    assert_eq!(pages[0].units[0].index, 0);
    assert_eq!(pages[0].units[1].index, 1);
    assert_eq!(pages[1].page_index, 1);
    assert!(matches!(
        &pages[1].units[0].source,
        UnitTranslationImportSource::PopRaKo {
            translated_text: Some(text),
            ..
        } if text == "second"
    ));
}
