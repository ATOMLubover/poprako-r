// Test suite for chapter import mapping and perm checks.
#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::chapter_port::import_translation::ChapterTranslationImportComplex;
use crate::complex::chapter_port::perm::ChapterPortPermComplex;
use crate::complex::unit::UnitComplex;
use crate::data::instr::chapter_port::ImportChapterTranslationInstr;
use crate::data::val::chapter_port::ImportChapterTranslationVal;
use crate::model::artifact::translation_import::{
    PageTranslationImport, UnitTranslationImport,
};
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitOrder};
use crate::model::shared::user::UserToken;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCountMetrics,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    ListPageInfosExcluded, SetPageUnitCountMetrics,
};
use crate::part::repo::oper::unit::{ApplyUnitEdits, ListUnitOrders};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::stage::start_pending_stages;
use crate::value::chapter::stage::Stage;
use crate::value::chapter_port::{
    ChapterTranslationImportMode, TranslationFormat,
};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};
use crate::value::role::RoleField;
use crate::value::unit::UnitEditPerm;

/// Imports chapter translation content through the Unit edit pipeline.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn import_translation<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: ImportChapterTranslationInstr,
    chapter_id: String,
) -> BaseRest<ImportChapterTranslationVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    let (edit_perm, format, mode, imported_pages, stages) =
        prepare_import(repo, &token, &instr, &chapter_id).await?;

    let val = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let page_scopes = ListPageInfosExcluded {
                chapter_id: &chapter_id,
            }
            .step_on(repo, context)
            .await?;

            ChapterTranslationImportComplex::validate_page_count(
                imported_pages.len(),
                page_scopes.len(),
            )?;

            let (
                final_page_count_metrics,
                imported_page_count,
                imported_unit_count,
            ) = import_pages(
                repo,
                context,
                &page_scopes,
                &imported_pages,
                &token.user_id,
                edit_perm,
                mode,
            )
            .await?;

            let chapter_count_metrics =
                final_page_count_metrics.iter().copied().fold(
                    UnitCountMetrics::default(),
                    |mut count_metrics, page_count_metrics| {
                        //
                        count_metrics.total += page_count_metrics.total;

                        count_metrics.translated +=
                            page_count_metrics.translated;

                        count_metrics.proofread += page_count_metrics.proofread;

                        count_metrics
                    },
                );

            SetChapterPageCountMetrics {
                id: &chapter_info.id,
                page_count: page_scopes.len(),
                total_unit_count: chapter_count_metrics.total,
                translated_unit_count: chapter_count_metrics.translated,
                proofread_unit_count: chapter_count_metrics.proofread,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let import_val = ImportChapterTranslationVal {
                imported_page_count,
                imported_unit_count,
            };

            let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                chapter_info.id.clone(),
                Some(token.user_id.clone()),
                ChapterWorkflowRecordPayload::TranslationImported {
                    format,
                    imported_page_count: import_val.imported_page_count,
                    imported_unit_count: import_val.imported_unit_count,
                },
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&workflow_record_entry),
            }
            .step_on(repo, context)
            .await?;

            start_pending_stages(
                repo,
                context,
                &chapter_info.id,
                Some(token.user_id.clone()),
                ChapterWorkflowRecordOrigin::TranslationImport,
                &stages,
            )
            .await?;

            accept(import_val)
        })
        .await?;

    accept(val)
}

// Validates permissions and parses the submitted translation content.
async fn prepare_import<C, R>(
    repo: &R,
    token: &UserToken,
    instr: &ImportChapterTranslationInstr,
    chapter_id: &str,
) -> BaseRest<(
    UnitEditPerm,
    TranslationFormat,
    ChapterTranslationImportMode,
    Vec<PageTranslationImport>,
    Vec<Stage>,
)>
where
    C: Context,
    R: AssignmentRepo<C> + Sync,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?
    .ok_or_else(|| {
        //
        let err_message = trl("error-chapter-port-import-perm-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id,
            user_id = %token.user_id,
            operation = "import chapter translation",
            "expected error: chapter import assignment required",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        }
    })?;

    ChapterPortPermComplex::ensure_user_can_import_translation(
        &assignment_info,
    )?;

    let edit_perm = UnitEditPerm {
        can_translate: assignment_info
            .roles
            .has_any_role(&[RoleField::TRANSLATOR]),
        can_proofread: assignment_info
            .roles
            .has_any_role(&[RoleField::PROOFREADER]),
    };

    let format = TranslationFormat::from(instr.format);

    let mode = ChapterTranslationImportMode::from(instr.mode);

    let imported_pages = match format {
        //
        TranslationFormat::LabelPlus => {
            ChapterTranslationImportComplex::parse_label_plus(&instr.content)?
        }

        TranslationFormat::PopRaKo => {
            ChapterTranslationImportComplex::parse_poprako(&instr.content)?
        }
    };

    let stages = import_stages(edit_perm);

    accept((edit_perm, format, mode, imported_pages, stages))
}

// Applies imported pages and collects their counters and import totals.
async fn import_pages<C, R>(
    repo: &R,
    context: &mut C,
    page_scopes: &[PageInfo],
    imported_pages: &[PageTranslationImport],
    user_id: &str,
    edit_perm: UnitEditPerm,
    mode: ChapterTranslationImportMode,
) -> BaseRest<(Vec<UnitCountMetrics>, usize, usize)>
where
    C: Context,
    R: PageRepo<C> + UnitRepo<C> + Sync,
{
    let mut imported_page_count = 0;

    let mut imported_unit_count = 0;

    let mut final_page_count_metrics = Vec::with_capacity(page_scopes.len());

    for page_scope in page_scopes {
        //
        let imported_page = imported_pages
            .iter()
            .find(|page| page.page_index == page_scope.index)
            .ok_or_else(|| BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-chapter-import-content"),
            })?;

        let page_import_outcome = replace_page_units(
            repo,
            context,
            page_scope,
            imported_page,
            user_id,
            edit_perm,
            mode,
        )
        .await?;

        final_page_count_metrics.push(page_import_outcome.count_metrics);

        imported_page_count += page_import_outcome.imported_page_count;

        imported_unit_count += page_import_outcome.imported_unit_count;
    }

    accept((
        final_page_count_metrics,
        imported_page_count,
        imported_unit_count,
    ))
}

// Selects workflow stages that the importing assignment can edit.
fn import_stages(edit_perm: UnitEditPerm) -> Vec<Stage> {
    //
    let mut stages = Vec::with_capacity(2);

    if edit_perm.can_translate {
        stages.push(Stage::Translate);
    }

    if edit_perm.can_proofread {
        stages.push(Stage::Proofread);
    }

    stages
}

// Captures the page-level results of one import application decision.
#[derive(Debug)]
struct PageImportOutcome {
    //
    // Final visible Unit counters for the page.
    count_metrics: UnitCountMetrics,
    // Whether this page's visible content changed through the import.
    imported_page_count: usize,
    // Number of source Units created through the import.
    imported_unit_count: usize,
}

// Applies imported Units to one page according to the selected strategy.
async fn replace_page_units<C, R>(
    repo: &R,
    context: &mut C,
    page_scope: &PageInfo,
    imported_page: &PageTranslationImport,
    user_id: &str,
    edit_perm: UnitEditPerm,
    mode: ChapterTranslationImportMode,
) -> BaseRest<PageImportOutcome>
where
    C: Context,
    R: PageRepo<C> + UnitRepo<C> + Sync,
{
    let orders = ListUnitOrders {
        page_id: &page_scope.id,
    }
    .step_on(repo, context)
    .await?;

    let visible_unit_ids = orders
        .iter()
        .filter(|order| !order.is_hidden)
        .map(|order| order.id.clone())
        .collect::<Vec<_>>();

    let current_count_metrics = page_unit_count_metrics(page_scope);

    if (visible_unit_ids.is_empty(), imported_page.units.is_empty())
        == (true, true)
    {
        return accept(PageImportOutcome {
            count_metrics: current_count_metrics,
            imported_page_count: 0,
            imported_unit_count: 0,
        });
    }

    match mode {
        //
        ChapterTranslationImportMode::Keep if !visible_unit_ids.is_empty() => {
            //
            return accept(PageImportOutcome {
                count_metrics: current_count_metrics,
                imported_page_count: 0,
                imported_unit_count: 0,
            });
        }

        _ => {}
    }

    let orders = match mode {
        //
        ChapterTranslationImportMode::Overwrite
            if !visible_unit_ids.is_empty() =>
        {
            hide_visible_units(
                repo,
                context,
                &page_scope.id,
                &orders,
                &visible_unit_ids,
            )
            .await?;

            orders
                .into_iter()
                .map(|mut order| {
                    //
                    order.is_hidden = true;

                    order
                })
                .collect()
        }

        ChapterTranslationImportMode::Keep
        | ChapterTranslationImportMode::Overwrite => orders,
    };

    let final_count_metrics = apply_imported_units(
        repo,
        context,
        (&page_scope.id, &orders, imported_page),
        (user_id, edit_perm),
    )
    .await?;

    SetPageUnitCountMetrics {
        id: &page_scope.id,
        count_metrics: final_count_metrics,
    }
    .step_on(repo, context)
    .await?;

    accept(PageImportOutcome {
        count_metrics: final_count_metrics,
        imported_page_count: 1,
        imported_unit_count: imported_page.units.len(),
    })
}

// Returns the stored counters for a page whose visible Units are unchanged.
const fn page_unit_count_metrics(page_info: &PageInfo) -> UnitCountMetrics {
    //
    UnitCountMetrics {
        total: page_info.total_unit_count,
        translated: page_info.translated_unit_count,
        proofread: page_info.proofread_unit_count,
    }
}

// Hides every currently visible Unit before an overwrite import.
async fn hide_visible_units<C, R>(
    repo: &R,
    context: &mut C,
    page_id: &str,
    orders: &[UnitOrder],
    visible_unit_ids: &[String],
) -> BaseRest<()>
where
    C: Context,
    R: UnitRepo<C> + Sync,
{
    let delete_edits = visible_unit_ids
        .iter()
        .map(|id| UnitEdit::Delete { id: id.clone() })
        .collect::<Vec<_>>();

    let base_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let delete_edits = UnitComplex::normalize_edits(&base_ids, delete_edits)?;

    ApplyUnitEdits {
        page_id,
        orders,
        edits: &delete_edits,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}

// Applies imported units against the current page order.
async fn apply_imported_units<C, R>(
    repo: &R,
    context: &mut C,
    (page_id, orders, imported_page): (
        &str,
        &[UnitOrder],
        &PageTranslationImport,
    ),
    (user_id, edit_perm): (&str, UnitEditPerm),
) -> BaseRest<UnitCountMetrics>
where
    C: Context,
    R: UnitRepo<C> + Sync,
{
    let Some(_) = imported_page.units.first() else {
        return accept(UnitCountMetrics::default());
    };

    let edits = build_page_edits(&imported_page.units, user_id, edit_perm);

    let base_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let edits = UnitComplex::normalize_edits(&base_ids, edits)?;

    let final_count_metrics = ApplyUnitEdits {
        page_id,
        orders,
        edits: &edits,
    }
    .step_on(repo, context)
    .await?;

    accept(final_count_metrics)
}

// Builds create edits for all imported units on one page.
fn build_page_edits(
    imported_units: &[UnitTranslationImport],
    user_id: &str,
    edit_perm: UnitEditPerm,
) -> Vec<UnitEdit> {
    //
    imported_units
        .iter()
        .map(|imported_unit| {
            //
            ChapterTranslationImportComplex::build_unit_create(
                imported_unit,
                UnitComplex::gen_id(),
                user_id,
                edit_perm.can_translate,
                edit_perm.can_proofread,
            )
        })
        .collect()
}
