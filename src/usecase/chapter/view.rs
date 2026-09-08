//! Chapter presentation assembly.

use poprako_orchestra::{Context, Run};

use poprako_obj_dept::ObjDeptView;

use crate::data::view::chapter::ChapterInfoView;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::view::{ObjViewIds, ObjViewSnapshot};

/// Resolves chapter models from one request-scoped object URL snapshot.
pub async fn chapter_info_views<C, R, O>(
    repo: &R,
    obj_dept: &O,
    models: Vec<ChapterInfo>,
) -> BaseRest<Vec<ChapterInfoView>>
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

    ids.collect_chapters(&models);

    let snapshot = ObjViewSnapshot::load_with_comic_fallbacks::<C, R, O>(
        repo, obj_dept, ids, None,
    )
    .await?;

    accept(
        models
            .into_iter()
            .map(|model| snapshot.chapter(model))
            .collect(),
    )
}
