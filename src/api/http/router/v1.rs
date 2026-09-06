use axum::Router;
use axum::routing::{delete, get, post, put};

use crate::api::http::handler::{
    announcement, assignment, assignment_invitation, auth, chapter,
    chapter_port, comic, comment, member, member_invitation, page, system_mail,
    team, term, termbase, termbase_port, unit, user, workset,
};
use crate::api::http::state::AppHarn;

/// Builds public version-one routes.
pub fn v1_public_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
}

/// Builds user routes.
pub fn v1_user_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/users/me", get(user::get_my_info))
        .route(
            "/users/{user_id}",
            get(user::get_info)
                .put(user::update_info)
                .delete(user::delete),
        )
        .route("/users/{user_id}/password", put(user::update_password))
        .route("/users/{user_id}/avatar/alloc", post(user::alloc_avatar))
        .route(
            "/users/{user_id}/avatar/mark-uploaded",
            post(user::mark_avatar_uploaded),
        )
}

/// Builds team routes.
pub fn v1_team_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/teams", get(team::list_infos).post(team::create))
        .route(
            "/teams/{team_id}",
            get(team::get_info)
                .put(team::update_info)
                .delete(team::delete),
        )
        .route("/teams/{team_id}/avatar/alloc", post(team::alloc_avatar))
        .route(
            "/teams/{team_id}/avatar/mark-uploaded",
            post(team::mark_avatar_uploaded),
        )
        .route(
            "/teams/{team_id}/online-users",
            get(team::list_online_user_ids),
        )
        .route(
            "/teams/{team_id}/mark-self-online",
            put(team::mark_self_online),
        )
        .route(
            "/teams/{team_id}/member-invitations",
            get(member_invitation::list_infos),
        )
        .route("/teams/{team_id}/worksets", get(workset::list_infos))
        .route("/teams/{team_id}/termbases", get(termbase::list_team_infos))
        .route(
            "/teams/{team_id}/termbases/import",
            post(termbase_port::import_team),
        )
}

/// Builds member routes.
pub fn v1_member_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/members", get(member::list_infos).post(member::create))
        .route("/members/me", get(member::list_my_infos))
        .route("/members/join", post(member::join))
        .route("/members/{member_id}/roles", put(member::update_roles))
        .route("/members/{member_id}", delete(member::delete))
}

/// Builds member invitation routes.
pub fn v1_member_invitation_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/member-invitations", post(member_invitation::create))
        .route(
            "/member-invitations/{member_invitation_id}/roles",
            put(member_invitation::update_roles),
        )
        .route(
            "/member-invitations/{member_invitation_id}",
            delete(member_invitation::delete),
        )
}

/// Builds workset routes.
pub fn v1_workset_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/worksets", post(workset::create))
        .route(
            "/worksets/{workset_id}",
            get(workset::get_info)
                .put(workset::update_info)
                .delete(workset::delete),
        )
}

/// Builds comic routes.
pub fn v1_comic_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/comics", post(comic::create))
        .route("/worksets/{workset_id}/comics", get(comic::list_infos))
        .route(
            "/comics/{comic_id}/termbases",
            get(termbase::list_comic_infos),
        )
        .route(
            "/comics/{comic_id}/termbases/import",
            post(termbase_port::import_comic),
        )
        .route(
            "/comics/{comic_id}",
            get(comic::get_info)
                .put(comic::update_info)
                .delete(comic::delete),
        )
        .route("/comics/{comic_id}/cover/alloc", post(comic::alloc_cover))
        .route(
            "/comics/{comic_id}/cover/mark-uploaded",
            post(comic::mark_cover_uploaded),
        )
        .route("/comics/{comic_id}/archive", post(comic::archive))
}

/// Builds comic archive routes.
pub fn v1_comic_archive_router() -> Router<AppHarn> {
    //
    Router::new().route(
        "/teams/{team_id}/comic-archives/export",
        get(comic::export_archives),
    )
}

/// Builds chapter routes.
pub fn v1_chapter_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/chapters", post(chapter::create))
        .route("/comics/{comic_id}/chapters", get(chapter::list_infos))
        .route(
            "/comics/{comic_id}/chapters/pinned",
            get(chapter::get_pinned),
        )
        .route(
            "/chapters/{chapter_id}",
            get(chapter::get_info)
                .patch(chapter::update_info)
                .delete(chapter::delete),
        )
        .route(
            "/chapters/{chapter_id}/workflow-records",
            get(chapter::list_workflow_record_infos),
        )
        .route(
            "/chapters/{chapter_id}/stage/advance",
            post(chapter::advance_stage),
        )
        .route(
            "/chapters/{chapter_id}/mark-pinned",
            post(chapter::mark_pinned),
        )
        .route(
            "/chapters/{chapter_id}/translations/import",
            post(chapter_port::import_translation),
        )
        .route(
            "/chapters/{chapter_id}/translations/export",
            get(chapter_port::export_translation),
        )
        .route(
            "/chapters/{chapter_id}/translations/export/download",
            get(chapter_port::export_translation_download),
        )
        .route(
            "/chapters/{chapter_id}/artwork/alloc",
            post(chapter_port::alloc_artwork),
        )
        .route(
            "/chapters/{chapter_id}/artwork/mark-uploaded",
            post(chapter_port::mark_artwork_uploaded),
        )
        .route(
            "/chapters/{chapter_id}/artwork/export",
            get(chapter_port::export_artwork),
        )
}

/// Builds page routes.
pub fn v1_page_router() -> Router<AppHarn> {
    //
    Router::new()
        .route(
            "/chapters/{chapter_id}/pages/editted-diffs",
            get(page::list_editted_diff_page_ids),
        )
        .route(
            "/chapters/{chapter_id}/pages",
            get(page::list_infos).delete(page::delete),
        )
        .route(
            "/chapters/{chapter_id}/pages/alloc",
            post(page::alloc_chapter_pages),
        )
        .route("/pages/{page_id}", get(page::get_info))
        .route("/pages/{page_id}/image/alloc", post(page::alloc_image))
        .route(
            "/pages/{page_id}/image/mark-uploaded",
            post(page::mark_image_uploaded),
        )
}

/// Builds unit routes.
pub fn v1_unit_router() -> Router<AppHarn> {
    //
    Router::new()
        .route(
            "/chapters/{chapter_id}/units/search",
            get(unit::search_infos),
        )
        .route(
            "/chapters/{chapter_id}/units/transform",
            post(unit::transform),
        )
        .route("/pages/{page_id}/units", get(unit::list_infos))
        .route("/pages/{page_id}/units/save", post(unit::save_infos))
}

/// Builds assignment routes.
pub fn v1_assignment_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/assignments", get(assignment::list_infos))
        .route("/assignments/join", post(assignment::join))
        .route(
            "/chapters/{chapter_id}/assignments/{user_id}/roles",
            put(assignment::update_roles),
        )
        .route("/assignments/{assignment_id}", delete(assignment::delete))
}

/// Builds assignment invitation routes.
pub fn v1_assignment_invitation_router() -> Router<AppHarn> {
    //
    Router::new()
        .route(
            "/assignment-invitations",
            post(assignment_invitation::create),
        )
        .route(
            "/assignment-invitations/join",
            post(assignment_invitation::join),
        )
        .route(
            "/chapters/{chapter_id}/assignment-invitations",
            get(assignment_invitation::list_infos),
        )
        .route(
            "/assignment-invitations/{assignment_invitation_id}",
            delete(assignment_invitation::delete),
        )
}

/// Builds system mail routes.
pub fn v1_system_mail_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/system-mails", get(system_mail::list_infos))
        .route("/system-mails/mark-read", post(system_mail::mark_read))
}

/// Builds announcement routes.
pub fn v1_announcement_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/announcements", post(announcement::create))
        .route(
            "/announcements/{announcement_id}",
            put(announcement::update_info).delete(announcement::delete),
        )
        .route(
            "/teams/{team_id}/announcements",
            get(announcement::list_infos),
        )
}

/// Builds comment routes.
pub fn v1_comment_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/comments", post(comment::create))
        .route("/teams/{team_id}/comments", get(comment::list_infos))
}

/// Builds termbase routes.
pub fn v1_termbase_router() -> Router<AppHarn> {
    //
    Router::new()
        .route("/termbases", post(termbase::create))
        .route(
            "/termbases/{termbase_id}",
            get(termbase::get_info)
                .put(termbase::update_info)
                .delete(termbase::delete),
        )
        .route(
            "/termbases/{termbase_id}/export",
            get(termbase_port::export),
        )
        .route(
            "/termbases/{termbase_id}/export/download",
            get(termbase_port::export_download),
        )
        .route("/termbases/{termbase_id}/terms", get(term::list_infos))
}

/// Builds term routes.
pub fn v1_term_router() -> Router<AppHarn> {
    //
    Router::new().route("/terms", post(term::create)).route(
        "/terms/{term_id}",
        get(term::get_info)
            .put(term::update_info)
            .delete(term::delete),
    )
}
