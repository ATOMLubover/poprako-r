//! `LabelPlus` text structure validation and page-unit assembly.

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::model::artifact::translation_import::{
    PageTranslationImport, UnitTranslationImport, UnitTranslationImportSource,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;
use crate::value::unit::MAX_PAGE_UNIT_COUNT;

/// Normalize `LabelPlus` text while preserving non-empty whitespace and lines.
pub fn normalize_label_plus_text(text: String) -> Option<String> {
    //
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

/// Construct the stable invalid-content error for `LabelPlus` input.
pub fn invalid_label_plus_content(condition: &str) -> BaseError {
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

/// Internal representation of a `LabelPlus` unit header.
pub struct LabelPlusUnit {
    //
    /// 0-based index of the unit inside its page.
    index: usize,
    /// X-axis coordinate from the header.
    x_coord: f64,
    /// Y-axis coordinate from the header.
    y_coord: f64,
    /// Whether the unit is a speech bubble.
    is_bubble: bool,
}

/// Remove only spaces and tabs that follow `LabelPlus` structure lines.
pub fn trim_label_plus_structure_line(line: &str) -> &str {
    line.trim_end_matches([' ', '\t'])
}

/// Check whether a line is a complete `LabelPlus` page header.
pub fn is_label_plus_page_header(line: &str) -> bool {
    //
    let line = trim_label_plus_structure_line(line);

    line.strip_prefix(">>>>>>>>[")
        .and_then(|line| line.strip_suffix("]<<<<<<<<"))
        .is_some_and(|filename| !filename.is_empty())
}

/// Parse a `LabelPlus` unit header, including its strict index and flag checks.
pub fn parse_label_plus_unit_header(
    line: &str,
) -> BaseRest<Option<LabelPlusUnit>> {
    //
    let line = trim_label_plus_structure_line(line);

    let Some(rest) = line.strip_prefix("----------------[") else {
        return accept(None);
    };

    let Some((index_text, rest)) = rest.split_once("]----------------[") else {
        //
        return Err(invalid_label_plus_content(
            "invalid LabelPlus unit separator",
        ));
    };

    let Some(coord_text) = rest.strip_suffix(']') else {
        //
        return Err(invalid_label_plus_content(
            "missing LabelPlus coordinate suffix",
        ));
    };

    let mut parts = coord_text.split(',');

    let (Some(x_coord_text), Some(y_coord_text), Some(bubble_text), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        //
        return Err(invalid_label_plus_content(
            "invalid LabelPlus coordinate count",
        ));
    };

    let index = index_text.parse::<usize>().map_err(|_| {
        invalid_label_plus_content("invalid LabelPlus unit index")
    })?;

    if index == 0 {
        //
        return Err(invalid_label_plus_content(
            "LabelPlus unit index is not positive",
        ));
    }

    let x_coord = x_coord_text.parse::<f64>().map_err(|_| {
        invalid_label_plus_content("invalid LabelPlus x coordinate")
    })?;

    let y_coord = y_coord_text.parse::<f64>().map_err(|_| {
        invalid_label_plus_content("invalid LabelPlus y coordinate")
    })?;

    if !x_coord.is_finite() || !y_coord.is_finite() {
        //
        return Err(invalid_label_plus_content(
            "LabelPlus coordinate is not finite",
        ));
    }

    let is_bubble = match bubble_text {
        //
        "1" => true,

        "2" => false,

        _ => {
            //
            return Err(invalid_label_plus_content(
                "invalid LabelPlus bubble flag",
            ));
        }
    };

    accept(Some(LabelPlusUnit {
        index: index - 1,
        x_coord,
        y_coord,
        is_bubble,
    }))
}

/// Flush the buffered `LabelPlus` unit into the current page.
pub fn flush_label_plus_unit(
    current_page: &mut Option<Vec<UnitTranslationImport>>,
    current_unit: &mut Option<LabelPlusUnit>,
    main_text_lines: &mut Vec<String>,
) -> BaseRest<()> {
    //
    let Some(label_plus_unit) = current_unit.take() else {
        return accept(());
    };

    let Some(page_units) = current_page.as_mut() else {
        return Err(invalid_label_plus_content("LabelPlus unit has no page"));
    };

    if page_units.len() >= MAX_PAGE_UNIT_COUNT {
        //
        return Err(invalid_label_plus_limit(
            "error-chapter-import-unit-count",
            MAX_PAGE_UNIT_COUNT,
            "LabelPlus page has too many units",
        ));
    }

    if page_units
        .iter()
        .any(|unit| unit.index == label_plus_unit.index)
    {
        //
        return Err(invalid_label_plus_content(
            "duplicate LabelPlus unit index",
        ));
    }

    page_units.push(UnitTranslationImport {
        index: label_plus_unit.index,
        x_coord: label_plus_unit.x_coord,
        y_coord: label_plus_unit.y_coord,
        is_bubble: label_plus_unit.is_bubble,
        source: UnitTranslationImportSource::LabelPlus {
            text: normalize_label_plus_text(main_text_lines.join("\n")),
        },
    });

    main_text_lines.clear();

    accept(())
}

/// Validate and order one parsed `LabelPlus` page.
pub fn finalize_label_plus_page(
    page: &mut [UnitTranslationImport],
) -> BaseRest<()> {
    //
    if page.len() > MAX_PAGE_UNIT_COUNT {
        //
        return Err(invalid_label_plus_limit(
            "error-chapter-import-unit-count",
            MAX_PAGE_UNIT_COUNT,
            "LabelPlus page has too many units",
        ));
    }

    let mut indexes = HashSet::with_capacity(page.len());

    for unit in page.iter() {
        //
        if !indexes.insert(unit.index) {
            //
            return Err(invalid_label_plus_content(
                "invalid LabelPlus unit index",
            ));
        }
    }

    page.sort_by_key(|unit| unit.index);

    accept(())
}

/// Validate the fixed `LabelPlus` preamble and its separator layout.
pub fn validate_label_plus_header<'a, I>(lines: &mut I) -> BaseRest<()>
where
    I: Iterator<Item = &'a str>,
{
    //
    let Some(ver_line) = lines.next() else {
        //
        return Err(invalid_label_plus_content(
            "LabelPlus version line is missing",
        ));
    };

    if !ver_line
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        //
        return Err(invalid_label_plus_content(
            "LabelPlus version line is invalid",
        ));
    }

    if lines.next().map(trim_label_plus_structure_line) != Some("-") {
        //
        return Err(invalid_label_plus_content(
            "LabelPlus initial separator is missing",
        ));
    }

    let has_content_separator = lines
        .by_ref()
        .map(trim_label_plus_structure_line)
        .any(|line| line == "-");

    if !has_content_separator {
        //
        return Err(invalid_label_plus_content(
            "LabelPlus content separator is missing",
        ));
    }

    if lines.next().is_none() {
        return Err(invalid_label_plus_content("LabelPlus content is missing"));
    }

    accept(())
}

/// Parse a complete `LabelPlus` document into chapter import pages.
pub fn parse_label_plus(content: &str) -> BaseRest<Vec<PageTranslationImport>> {
    //
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);

    let mut lines = content
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));

    validate_label_plus_header(&mut lines)?;

    let (mut pages, mut current_page, mut current_unit, mut main_text_lines) = (
        Vec::new(),
        None::<Vec<UnitTranslationImport>>,
        None::<LabelPlusUnit>,
        Vec::new(),
    );

    for line in lines {
        //
        if is_label_plus_page_header(line) {
            //
            flush_label_plus_unit(
                &mut current_page,
                &mut current_unit,
                &mut main_text_lines,
            )?;

            if let Some(mut units) = current_page.take() {
                //
                finalize_label_plus_page(&mut units)?;

                pages.push(PageTranslationImport {
                    page_index: pages.len(),
                    units,
                });
            }

            current_page = Some(Vec::new());

            continue;
        }

        let structural_line = line.trim_end_matches([' ', '\t']);

        if structural_line.starts_with(">>>>>>>>") {
            //
            let err_message = trl("error-invalid-chapter-import-content");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                "expected error: LabelPlus structure line is malformed",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if let Some(unit) = parse_label_plus_unit_header(line)? {
            //
            if current_page.is_none() {
                //
                let err_message = trl("error-invalid-chapter-import-content");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    line = %line,
                    "expected error: chapter import unit appears before page",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            flush_label_plus_unit(
                &mut current_page,
                &mut current_unit,
                &mut main_text_lines,
            )?;

            current_unit = Some(unit);

            continue;
        }

        if current_unit.is_some() && !line.is_empty() {
            main_text_lines.push(line.to_string());
        }
    }

    flush_label_plus_unit(
        &mut current_page,
        &mut current_unit,
        &mut main_text_lines,
    )?;

    if let Some(mut units) = current_page.take() {
        //
        finalize_label_plus_page(&mut units)?;

        pages.push(PageTranslationImport {
            page_index: pages.len(),
            units,
        });
    }

    if pages.len() > MAX_CHAPTER_PAGE_COUNT {
        //
        return Err(invalid_label_plus_limit(
            "error-chapter-import-page-count",
            MAX_CHAPTER_PAGE_COUNT,
            "LabelPlus document has too many pages",
        ));
    }

    accept(pages)
}

// Construct a localized limit error for LabelPlus input.
fn invalid_label_plus_limit(
    key: &str,
    limit: usize,
    condition: &str,
) -> BaseError {
    //
    let args = HashMap::from([("limit".into(), limit.into())]);

    let err_message = trl_kv(key, &args);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        limit,
        condition,
        "expected error: LabelPlus import limit exceeded",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}
