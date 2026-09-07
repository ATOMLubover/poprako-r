use super::*;

use serde_json::json;

// import_chapter_translation_instr_uses_snake_case_format(ImportChapterTranslationInstr)(positive): JSON body formats deserialize from snake_case.
#[test]
fn import_chapter_translation_instr_uses_snake_case_format() {
    let instr =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label_plus",
            "mode": "keep",
            "content": "content",
        }))
        .unwrap();

    assert!(matches!(
        instr.format,
        ChapterTranslationFormatInstr::LabelPlus
    ));

    assert!(matches!(
        instr.mode,
        ChapterTranslationImportModeInstr::Keep
    ));
}

// import_chapter_translation_instr_rejects_kebab_case_format(ImportChapterTranslationInstr)(negative): kebab-case is not accepted by a JSON body DTO.
#[test]
fn import_chapter_translation_instr_rejects_kebab_case_format() {
    let result =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label-plus",
            "mode": "overwrite",
            "content": "content",
        }));

    assert!(result.is_err());
}

// import_chapter_translation_instr_requires_mode(ImportChapterTranslationInstr)(negative): import callers must explicitly choose whether existing page content is preserved or replaced.
#[test]
fn import_chapter_translation_instr_requires_mode() {
    let result =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label_plus",
            "content": "content",
        }));

    assert!(result.is_err());
}

// import_chapter_translation_instr_rejects_invalid_mode(ImportChapterTranslationInstr)(negative): only the documented snake_case import modes are accepted.
#[test]
fn import_chapter_translation_instr_rejects_invalid_mode() {
    let result =
        serde_json::from_value::<ImportChapterTranslationInstr>(json!({
            "format": "label_plus",
            "mode": "replace",
            "content": "content",
        }));

    assert!(result.is_err());
}

// artwork_hash_validation(AllocChapterArtworkInstr)(negative): arbitrary strings and non-SHA-256 Base64 hashes are rejected at the transport boundary.
#[test]
fn artwork_hash_validation_rejects_invalid_identities() {
    for hash in ["", "not-a-hash", "YQ==", "a".repeat(64).as_str()] {
        let instr = serde_json::from_value::<AllocChapterArtworkInstr>(json!({
            "artwork_hash": hash, "new_byte_len": 1024, "ext": "zip",
        }));

        assert!(instr.is_err());
    }

    let instr = serde_json::from_value::<AllocChapterArtworkInstr>(json!({
        "artwork_hash": crate::value::artwork::ArtworkHash::new([7; 32]),
        "new_byte_len": 1024, "ext": "zip",
    }))
    .unwrap();

    assert_eq!(instr.artwork_hash.as_bytes(), &[7; 32]);
}
