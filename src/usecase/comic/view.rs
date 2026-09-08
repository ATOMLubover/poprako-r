//! Comic presentation assembly.

use poprako_orchestra::{Context, Run};

use poprako_obj_dept::ObjDeptView;

use crate::data::view::comic::ComicInfoView;
use crate::model::read::proj::comic::ComicInfo;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::view::{ObjViewIds, ObjViewSnapshot};

/// Resolves one comic model and every included object-backed model.
pub async fn comic_info_view<C, R, O>(
    repo: &R,
    obj_dept: &O,
    model: ComicInfo,
) -> BaseRest<ComicInfoView>
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

    ids.collect_comics(std::slice::from_ref(&model));

    let snapshot = ObjViewSnapshot::load_with_comic_fallbacks::<C, R, O>(
        repo, obj_dept, ids, None,
    )
    .await?;

    accept(snapshot.comic(model))
}
