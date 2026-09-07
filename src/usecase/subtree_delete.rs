//! Relational hierarchy sweep orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::DeleteObjs;

use crate::model::read::proj::subtree_delete::SubtreeDeleteSweepTarget;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar,
};
use crate::part::repo::oper::subtree_delete::{
    ClaimSubtreeSweep, ListSubtreePageIds, SweepSubtree,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::value::subtree_delete::SubtreeSweepLevel;

/// Sweeps one eligible hierarchy claim for the requested level.
#[instrument(level = "info", skip_all)]
pub async fn sweep<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    level: SubtreeSweepLevel,
) -> BaseRest<bool>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: SubtreeRepo<C> + Send + Sync,
    O: ObjDept<ChapterArtwork, C>
        + ObjDept<PageImage, C>
        + ObjDept<ComicCover, C>
        + ObjDept<TeamAvatar, C>
        + Send
        + Sync,
{
    let swept = nucl
        .coord(async |context| {
            //
            let target =
                ClaimSubtreeSweep { level }.step_on(repo, context).await?;

            let Some(target) = target else {
                return accept(false);
            };

            match &target {
                //
                SubtreeDeleteSweepTarget::Chapter { id } => {
                    //
                    DeleteObjs::<ChapterArtwork>::new(std::slice::from_ref(id))
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;

                    let page_ids = ListSubtreePageIds { chapter_id: id }
                        .step_on(repo, context)
                        .await?;

                    DeleteObjs::<PageImage>::new(&page_ids)
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Comics { ids } => {
                    //
                    DeleteObjs::<ComicCover>::new(ids)
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Team { id } => {
                    //
                    DeleteObjs::<TeamAvatar>::new(std::slice::from_ref(id))
                        .step_on(obj_dept, context)
                        .await
                        .map_err(BaseError::from)?;
                }

                SubtreeDeleteSweepTarget::Worksets { .. } => {}
            }

            SweepSubtree { target: &target }
                .step_on(repo, context)
                .await?;

            accept(true)
        })
        .await?;

    accept(swept)
}
