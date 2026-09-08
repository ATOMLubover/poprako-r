//! Team presentation assembly.

use std::collections::HashMap;

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::model::url::ObjUrls;

use crate::data::view::team::TeamInfoView;
use crate::model::read::proj::team::TeamInfo;
use crate::part::obj_dept::TeamAvatar;
use crate::result::{BaseRest, accept};
use crate::usecase::internal::view::load_obj_urls;

/// Resolves one team model with its avatar origin and thumbnail URLs.
pub async fn team_info_view<C, O>(
    obj_dept: &O,
    model: TeamInfo,
) -> BaseRest<TeamInfoView>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + Sync,
{
    let team_ids = [model.id.as_str()];

    let avatar_urls = avatar_urls::<C, O>(obj_dept, &team_ids).await?;

    let urls = avatar_urls.get(&model.id);

    accept(team_info_view_from_urls(model, urls))
}

/// Renders one team with URLs from an already-loaded object snapshot.
pub fn team_info_view_from_urls(
    model: TeamInfo,
    urls: Option<&ObjUrls>,
) -> TeamInfoView {
    //
    TeamInfoView::from_model(
        model,
        urls.and_then(|urls| urls.origin_url.as_ref())
            .map(ToString::to_string),
        urls.and_then(|urls| urls.thumbnail_url.as_ref())
            .map(ToString::to_string),
    )
}

/// Resolves team models with one avatar metadata query.
pub async fn team_info_views<C, O>(
    obj_dept: &O,
    models: Vec<TeamInfo>,
) -> BaseRest<Vec<TeamInfoView>>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + Sync,
{
    let team_ids = models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<Vec<_>>();

    let avatar_urls = avatar_urls::<C, O>(obj_dept, &team_ids).await?;

    accept(
        models
            .into_iter()
            .map(|model| {
                //
                let urls = avatar_urls.get(&model.id);

                team_info_view_from_urls(model, urls)
            })
            .collect(),
    )
}

/// Resolves current avatar URLs from one metadata query for the supplied team IDs.
pub async fn avatar_urls<C, O>(
    obj_dept: &O,
    team_ids: &[&str],
) -> BaseRest<HashMap<String, ObjUrls>>
where
    C: Context,
    O: ObjDeptView<TeamAvatar, C> + Sync,
{
    load_obj_urls::<C, O, TeamAvatar>(obj_dept, team_ids).await
}
