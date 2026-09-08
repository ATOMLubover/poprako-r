//! User presentation assembly.

use std::collections::HashMap;

use poprako_orchestra::Context;

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::model::url::ObjUrls;

use crate::data::view::user::UserInfoView;
use crate::model::read::proj::user::UserInfo;
use crate::part::obj_dept::UserAvatar;
use crate::result::{BaseRest, accept};
use crate::usecase::internal::view::load_obj_urls;

/// Resolves one user model with its avatar origin and thumbnail URLs.
pub async fn user_info_view<C, O>(
    obj_dept: &O,
    model: UserInfo,
) -> BaseRest<UserInfoView>
where
    C: Context,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let user_ids = [model.id.as_str()];

    let avatar_urls = avatar_urls::<C, O>(obj_dept, &user_ids).await?;

    let urls = avatar_urls.get(&model.id);

    accept(user_info_view_from_urls(model, urls))
}

/// Renders one user with URLs from an already-loaded object snapshot.
pub fn user_info_view_from_urls(
    model: UserInfo,
    urls: Option<&ObjUrls>,
) -> UserInfoView {
    //
    UserInfoView::from_model(
        model,
        urls.and_then(|urls| urls.origin_url.as_ref())
            .map(ToString::to_string),
        urls.and_then(|urls| urls.thumbnail_url.as_ref())
            .map(ToString::to_string),
    )
}

/// Resolves current avatar URLs from one metadata query for the supplied user IDs.
pub async fn avatar_urls<C, O>(
    obj_dept: &O,
    user_ids: &[&str],
) -> BaseRest<HashMap<String, ObjUrls>>
where
    C: Context,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    load_obj_urls::<C, O, UserAvatar>(obj_dept, user_ids).await
}
