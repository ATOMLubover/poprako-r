use axum::extract::Query;
use axum::http::Uri;

use crate::data::instr::chapter_port::ExportChapterTranslationInstr;
use crate::value::chapter_port::ExportFormatSpec;

// translation_export_query_accepts_combined_formats(ExportChapterTranslationInstr)(positive): one export query selects both output formats.
#[test]
fn translation_export_query_accepts_combined_formats() {
    let uri = "http://localhost/translations/export?format=poprako,label_plus"
        .parse::<Uri>()
        .unwrap();

    let instr = Query::<ExportChapterTranslationInstr>::try_from_uri(&uri)
        .unwrap()
        .0;

    assert_eq!(instr.format, ExportFormatSpec::BOTH);
    assert!(!instr.with_raw_ident);
}

// translation_export_query_rejects_duplicate_format(ExportChapterTranslationInstr)(negative): one format cannot occur twice in the export spec.
#[test]
fn translation_export_query_rejects_duplicate_format() {
    let uri = "http://localhost/translations/export?format=poprako,poprako"
        .parse::<Uri>()
        .unwrap();

    assert!(
        Query::<ExportChapterTranslationInstr>::try_from_uri(&uri).is_err()
    );
}

#[test]
fn translation_export_query_parses_raw_ident_switch() {
    for (value, expected) in [("true", true), ("false", false)] {
        let uri = format!("http://localhost/translations/export?format=label_plus&with_raw_ident={value}").parse::<Uri>().unwrap();

        let instr = Query::<ExportChapterTranslationInstr>::try_from_uri(&uri)
            .unwrap()
            .0;

        assert_eq!(instr.with_raw_ident, expected);
    }

    let uri = "http://localhost/translations/export?format=label_plus&with_raw_ident=invalid".parse::<Uri>().unwrap();

    assert!(
        Query::<ExportChapterTranslationInstr>::try_from_uri(&uri).is_err()
    );
}
