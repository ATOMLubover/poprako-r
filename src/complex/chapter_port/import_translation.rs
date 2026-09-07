// LabelPlus chapter import rules.
mod label_plus;

#[cfg(test)]
// Test cases for chapter-import parsing, translation assembly, and validation.
mod tests;

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::chapter_port::import_translation::label_plus::parse_label_plus;
use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::model::artifact::translation_import::{
    PageTranslationImport, UnitTranslationImport, UnitTranslationImportSource,
};
use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;
use crate::value::unit::MAX_PAGE_UNIT_COUNT;

// Construct a localized limit error for PopRaKo input.
fn invalid_import_limit(key: &str, limit: usize, condition: &str) -> BaseError {
    //
    let args = HashMap::from([("limit".into(), limit.into())]);

    let err_message = trl_kv(key, &args);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        limit,
        condition,
        "expected error: chapter import limit exceeded",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

// Construct the stable invalid-content error for PopRaKo input.
fn invalid_poprako_content(condition: &str) -> BaseError {
    //
    let err_message = trl("error-invalid-chapter-import-content");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        condition,
        "expected error: chapter import content is invalid",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

// Normalize an optional PopRaKo text field without changing non-empty text.
fn normalize_optional_poprako_text(text: Option<String>) -> Option<String> {
    text.and_then(normalize_poprako_text)
}

// Convert the shared PopRaKo document into normalized import pages.
fn convert_poprako_document(
    project: ChapterTranslationPortView,
) -> BaseRest<Vec<PageTranslationImport>> {
    //
    if project.pages.len() > MAX_CHAPTER_PAGE_COUNT {
        //
        return Err(invalid_import_limit(
            "error-chapter-import-page-count",
            MAX_CHAPTER_PAGE_COUNT,
            "PopRaKo document has too many pages",
        ));
    }

    let page_count = project.pages.len();

    let mut page_indexes = HashSet::with_capacity(project.pages.len());

    let mut pages = Vec::with_capacity(project.pages.len());

    for page in project.pages {
        //
        if page.page_index >= page_count
            || !page_indexes.insert(page.page_index)
        {
            return Err(invalid_poprako_content("invalid PopRaKo page index"));
        }

        if page.units.len() > MAX_PAGE_UNIT_COUNT {
            //
            return Err(invalid_import_limit(
                "error-chapter-import-unit-count",
                MAX_PAGE_UNIT_COUNT,
                "PopRaKo page has too many units",
            ));
        }

        let mut unit_indexes = HashSet::with_capacity(page.units.len());

        let mut units = Vec::with_capacity(page.units.len());

        for unit in page.units {
            //
            if !unit_indexes.insert(unit.unit_index) {
                //
                return Err(invalid_poprako_content(
                    "invalid PopRaKo unit index",
                ));
            }

            if !unit.x_coord.is_finite() || !unit.y_coord.is_finite() {
                //
                return Err(invalid_poprako_content(
                    "PopRaKo coordinate is not finite",
                ));
            }

            units.push(UnitTranslationImport {
                index: unit.unit_index,
                x_coord: unit.x_coord,
                y_coord: unit.y_coord,
                is_bubble: unit.is_bubble,
                source: UnitTranslationImportSource::PopRaKo {
                    translated_text: normalize_optional_poprako_text(
                        unit.translated_text,
                    ),
                    proofread_text: normalize_optional_poprako_text(
                        unit.proofread_text,
                    ),
                    is_proofread: unit.is_proofread,
                },
            });
        }

        units.sort_by_key(|unit| unit.index);

        pages.push(PageTranslationImport {
            page_index: page.page_index,
            units,
        });
    }

    pages.sort_by_key(|page| page.page_index);

    if pages
        .iter()
        .enumerate()
        .any(|(index, page)| page.page_index != index)
    {
        return Err(invalid_poprako_content(
            "PopRaKo page indexes are incomplete",
        ));
    }

    accept(pages)
}

// Build translated text when the current user has translator permission.
fn build_translation(
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    can_translate: bool,
) -> Option<UnitTranslation> {
    //
    if !can_translate {
        return None;
    }

    let translated_text = match &parsed_unit.source {
        //
        UnitTranslationImportSource::LabelPlus { text } => text.clone(),

        UnitTranslationImportSource::PopRaKo {
            translated_text, ..
        } => translated_text.clone(),
    };

    translated_text.map(|translated_text| UnitTranslation {
        translated_text,
        last_translator_id: user_id.to_string(),
    })
}

// Build proofread text when the current user has proofreader permission.
fn build_revision(
    parsed_unit: &UnitTranslationImport,
    user_id: &str,
    can_proofread: bool,
) -> Option<UnitRevision> {
    //
    if !can_proofread {
        return None;
    }

    let (proofread_text, is_proofread) = match &parsed_unit.source {
        //
        UnitTranslationImportSource::LabelPlus { text } => {
            (text.clone(), text.is_some())
        }

        //
        UnitTranslationImportSource::PopRaKo {
            proofread_text,
            is_proofread,
            ..
        } => (proofread_text.clone(), *is_proofread),
    };

    if proofread_text.is_none() && !is_proofread {
        return None;
    }

    Some(UnitRevision {
        is_proofread,
        proofread_text,
        last_proofreader_id: user_id.to_string(),
    })
}

// Normalize PopRaKo text while preserving non-empty whitespace and lines.
fn normalize_poprako_text(text: String) -> Option<String> {
    //
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

// Parse and validate a complete PopRaKo JSON document.
fn parse_poprako(content: &str) -> BaseRest<Vec<PageTranslationImport>> {
    //
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);

    let project = serde_json::from_str::<ChapterTranslationPortView>(content)
        .map_err(|error| {
        //
        let err_message = trl("error-invalid-chapter-import-content");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            input_length = content.len(),
            parse_err = ?error,
            operation = "parse_poprako",
            "expected error: chapter import JSON is invalid",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    convert_poprako_document(project)
}

// Build one unit creation edit from imported content.
fn build_unit_create(
    parsed_unit: &UnitTranslationImport,
    unit_id: String,
    user_id: &str,
    can_translate: bool,
    can_proofread: bool,
) -> UnitEdit {
    //
    let (translation, revision) = (
        build_translation(parsed_unit, user_id, can_translate),
        build_revision(parsed_unit, user_id, can_proofread),
    );

    UnitEdit::Create {
        id: unit_id,
        next_id: None,
        is_bubble: parsed_unit.is_bubble,
        coord: UnitCoord {
            x_coord: parsed_unit.x_coord,
            y_coord: parsed_unit.y_coord,
        },
        translation,
        revision,
    }
}

/// Chapter import parsing and payload merge rules.
pub struct ChapterTranslationImportComplex;

impl ChapterTranslationImportComplex {
    /// Parses `LabelPlus` text into chapter import pages.
    pub fn parse_label_plus(
        content: &str,
    ) -> BaseRest<Vec<PageTranslationImport>> {
        parse_label_plus(content)
    }

    /// Parses `PopRaKo` JSON text into chapter import pages.
    pub fn parse_poprako(
        content: &str,
    ) -> BaseRest<Vec<PageTranslationImport>> {
        parse_poprako(content)
    }

    /// Returns an error when imported pages do not match existing pages.
    pub fn validate_page_count(
        imported_page_count: usize,
        existing_page_count: usize,
    ) -> BaseRest<()> {
        //
        if imported_page_count != existing_page_count {
            //
            let err_message = trl("error-chapter-import-page-count-mismatch");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                imported_page_count = imported_page_count,
                existing_page_count = existing_page_count,
                "expected error: chapter import page count mismatch",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(())
    }

    /// Builds one Unit Create from parsed import content.
    pub fn build_unit_create(
        parsed_unit: &UnitTranslationImport,
        unit_id: String,
        user_id: &str,
        can_translate: bool,
        can_proofread: bool,
    ) -> UnitEdit {
        //
        build_unit_create(
            parsed_unit,
            unit_id,
            user_id,
            can_translate,
            can_proofread,
        )
    }
}
