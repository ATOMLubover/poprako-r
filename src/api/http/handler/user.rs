//! User handlers: profile read/update, deletion, and avatar upload flow.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::{
    ensure_current_user, ensure_path_matches_body_id,
};

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::user::{
    AllocUserAvatarInstr, MarkUserAvatarUploadedInstr, UpdateUserInfoInstr,
    UpdateUserPasswordInstr,
};
use crate::data::val::user::AllocUserAvatarVal;
use crate::data::view::user::UserInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;

/// `GET /api/v1/users/me` — current user's profile.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "users",
    responses(
        (status = 200, description = "Current user profile", body = HttpBody<UserInfoView>),
        (status = 401, description = "Authentication required"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_my_info(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<UserInfoView> {
    //
    let id = user_token.user_id.clone();

    usecase::user::get_info::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/users/{user_id}` — a user's profile by id.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID")),
    responses(
        (status = 200, description = "User profile retrieved", body = HttpBody<UserInfoView>),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "User not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(token): Extension<UserToken>,
) -> HttpResult<UserInfoView> {
    //
    usecase::user::get_info::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        token,
        user_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/users/{user_id}` — update a user's profile.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID")),
    request_body = UpdateUserInfoInstr,
    responses(
        (status = 204, description = "Profile updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "Cannot modify another user's profile"),
        (status = 409, description = "QID already taken"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateUserInfoInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&user_id, &instr.id)?;

    usecase::user::update_info::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `PUT /api/v1/users/{user_id}/password` — replace the current user's password.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/users/{user_id}/password",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID (must match authenticated user)")),
    request_body = UpdateUserPasswordInstr,
    responses(
        (status = 204, description = "Password updated"),
        (status = 401, description = "Current password is incorrect"),
        (status = 403, description = "Cannot modify another user's password"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_password(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateUserPasswordInstr>,
) -> HttpNoContent {
    //
    usecase::user::update_password::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        user_id,
        instr,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/users/{user_id}` — delete a user account.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 403, description = "Cannot delete another user or the last admin of a team"),
        (status = 409, description = "Serializable conflict; retry the complete request"),
        (status = 404, description = "User not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::user::delete::delete::<_, RdbContext<Serial>, HybRepo, _>(
        (harn.nucl().serial(), harn.repo(), harn.obj_dept()),
        user_token,
        user_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/users/{user_id}/avatar/alloc` — allocate an avatar upload slot.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/users/{user_id}/avatar/alloc",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID (must match authenticated user)")),
    request_body = AllocUserAvatarInstr,
    responses(
        (status = 200, description = "Avatar upload URL allocated", body = HttpBody<AllocUserAvatarVal>),
        (status = 403, description = "Cannot modify another user's avatar"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_avatar(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<AllocUserAvatarInstr>,
) -> HttpResult<AllocUserAvatarVal> {
    //
    ensure_current_user(&user_id, &user_token)?;

    usecase::user::alloc_avatar::<_, RdbContext<ReptRead>, HybRepo, _>(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            &harn.config().image,
        ),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/users/{user_id}/avatar/mark-uploaded` — confirm an avatar upload.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/users/{user_id}/avatar/mark-uploaded",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID (must match authenticated user)")),
    request_body = MarkUserAvatarUploadedInstr,
    responses(
        (status = 204, description = "Avatar upload confirmed"),
        (status = 403, description = "Cannot confirm another user's avatar"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_avatar_uploaded(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkUserAvatarUploadedInstr>,
) -> HttpNoContent {
    //
    usecase::user::mark_avatar_uploaded::<RdbContext, _>(
        (harn.obj_dept(),),
        user_token,
        user_id,
        instr,
    )
    .await?;

    no_content()
}
