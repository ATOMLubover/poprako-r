//! Repository trait for the page domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::page::{
    ApplyPageManifest, DeletePages, GetPageInfo, GetPageInfoExcluded,
    GetPageUnitScope, GetPageUnitScopeExcluded, ListEdittedDiffPageIds,
    ListFirstPageInfos, ListPageInfos, ListPageInfosExcluded,
    ListPageRawIdentInfos, SetPageRawIdents, SetPageUnitCountMetrics,
    ShiftPageIndexesTemporary,
};
use crate::result::BaseError;

/// Page repository operations over standalone runs and coordinated steps.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListPageRawIdentInfos<'a>,
        for<'a> GetPageInfo<'a>,
        for<'a> GetPageUnitScope<'a>,
        for<'a> ListPageInfos<'a>,
        for<'a> ListFirstPageInfos<'a>,
        for<'a> ListEdittedDiffPageIds<'a>,
    ),
    step(
        for<'a> SetPageRawIdents<'a>,
        for<'a> GetPageInfo<'a>,
        for<'a> GetPageUnitScope<'a>,
        for<'a> ListPageInfos<'a>,
        for<'a> ListPageInfosExcluded<'a>,
        for<'a> GetPageInfoExcluded<'a>,
        for<'a> GetPageUnitScopeExcluded<'a>,
        for<'a> SetPageUnitCountMetrics<'a>,
        for<'a> ShiftPageIndexesTemporary<'a>,
        for<'a> ApplyPageManifest<'a>,
        for<'a> DeletePages<'a>,
    ),
)]
pub trait PageRepo<C> {}
