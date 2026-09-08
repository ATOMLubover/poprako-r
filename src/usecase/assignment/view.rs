//! Assignment presentation assembly.

use poprako_orchestra::{Context, Run};

use poprako_obj_dept::ObjDeptView;

use crate::data::view::assignment::AssignmentInfoView;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::view::{ObjViewIds, ObjViewSnapshot};

/// Resolves one assignment model and its included models.
pub async fn assignment_info_view<C, O>(
    obj_dept: &O,
    model: AssignmentInfo,
) -> BaseRest<AssignmentInfoView>
where
    C: Context,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_assignments(std::slice::from_ref(&model));

    let snapshot = ObjViewSnapshot::load::<C, O>(obj_dept, &ids).await?;

    accept(snapshot.assignment(model))
}

/// Resolves assignment models from one request-scoped object URL snapshot.
pub async fn assignment_info_views<C, R, O>(
    repo: &R,
    obj_dept: &O,
    models: Vec<AssignmentInfo>,
) -> BaseRest<Vec<AssignmentInfoView>>
where
    C: Context,
    R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
        + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_assignments(&models);

    let snapshot = ObjViewSnapshot::load_with_comic_fallbacks::<C, R, O>(
        repo, obj_dept, ids, None,
    )
    .await?;

    accept(
        models
            .into_iter()
            .map(|model| snapshot.assignment(model))
            .collect(),
    )
}
