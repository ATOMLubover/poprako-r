//! OpenAPI documentation assembly.
//!
//! Handler-level `#[utoipa::path]` attributes specify full paths (including
//! the `/api/v1` prefix). Swagger UI and the OpenAPI JSON are mounted in debug
//! builds only, outside `/api/v1`.

use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::result::HttpError;
use crate::data::instr::announcement::{
    CreateAnnouncementInstr, UpdateAnnouncementInfoInstr,
};
use crate::data::instr::assignment::{
    JoinChapterAssignmentInstr, UpdateAssignmentRolesInstr,
};
use crate::data::instr::assignment_invitation::{
    CreateAssignmentInvitationInstr, JoinAssignmentInvitationInstr,
};
use crate::data::instr::auth::{LoginAuthInstr, RegisterAuthInstr};
use crate::data::instr::chapter::{
    ChapterStageInstr, ChapterStageOperInstr, CreateChapterInstr,
    UpdateChapterInfoInstr, UpdateChapterStageInstr,
};
use crate::data::instr::chapter_port::{
    ChapterTranslationFormatInstr, ChapterTranslationImportModeInstr,
    ImportChapterTranslationInstr,
};
use crate::data::instr::comic::{
    AllocComicCoverInstr, CreateComicInstr, MarkComicCoverUploadedInstr,
    UpdateComicInfoInstr,
};
use crate::data::instr::comment::CreateCommentInstr;
use crate::data::instr::member::{
    CreateMemberInstr, JoinTeamInstr, UpdateMemberRolesInstr,
};
use crate::data::instr::member_invitation::{
    CreateMemberInvitationInstr, UpdateMemberInvitationRolesInstr,
};
use crate::data::instr::page::{
    AllocChapterPagesInstr, AllocPageImageInstr, MarkPageImageUploadedInstr,
    PageImageInstr,
};
use crate::data::instr::system_mail::MarkSystemMailReadInstr;
use crate::data::instr::team::{
    AllocTeamAvatarInstr, CreateTeamInstr, MarkTeamAvatarUploadedInstr,
    UpdateTeamInfoInstr,
};
use crate::data::instr::term::{CreateTermInstr, UpdateTermInfoInstr};
use crate::data::instr::termbase::{
    CreateTermbaseInstr, UpdateTermbaseInfoInstr,
};
use crate::data::instr::termbase_port::{ImportTermInstr, ImportTermbaseInstr};
use crate::data::instr::unit::{
    TransformChapterUnitsInstr, UnitCoordInstr, UnitEditInstr,
    UnitRevisionInstr, UnitTextTransformInstr, UnitTransformInstr,
    UnitTranslationInstr,
};
use crate::data::instr::user::{
    AllocUserAvatarInstr, MarkUserAvatarUploadedInstr, UpdateUserInfoInstr,
    UpdateUserPasswordInstr,
};
use crate::data::instr::workset::{CreateWorksetInstr, UpdateWorksetInfoInstr};
use crate::data::val::announcement::CreateAnnouncementVal;
use crate::data::val::assignment_invitation::CreateAssignmentInvitationVal;
use crate::data::val::auth::{LoginAuthVal, RegisterAuthVal};
use crate::data::val::chapter::CreateChapterVal;
use crate::data::val::chapter_port::ImportChapterTranslationVal;
use crate::data::val::comic::{AllocComicCoverVal, CreateComicVal};
use crate::data::val::comic_archive::ArchiveComicVal;
use crate::data::val::comic_list::ListComicInfosVal;
use crate::data::val::comment::CreateCommentVal;
use crate::data::val::member::CreateMemberVal;
use crate::data::val::member_invitation::CreateMemberInvitationVal;
use crate::data::val::page::{
    AllocChapterPagesVal, AllocatedPageVal, ListEdittedDiffPageIdsVal,
};
use crate::data::val::team::AllocTeamAvatarVal;
use crate::data::val::term::CreateTermVal;
use crate::data::val::termbase::CreateTermbaseVal;
use crate::data::val::termbase_port::{ExportTermbaseVal, ImportTermbaseVal};
use crate::data::val::unit::ListPageUnitInfosVal;
use crate::data::val::user::AllocUserAvatarVal;
use crate::data::val::workset::CreateWorksetVal;
use crate::data::view::announcement::AnnouncementInfoView;
use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::assignment_invitation::AssignmentInvitationInfoView;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::data::view::chapter_workflow_record::{
    ChapterWorkflowRecordEventView, ChapterWorkflowRecordInfoView,
    ChapterWorkflowRecordOriginView, ChapterWorkflowRecordStagePhaseView,
    ChapterWorkflowRecordStageView, ChapterWorkflowRecordTranslationFormatView,
};
use crate::data::view::comic::ComicInfoView;
use crate::data::view::comment::CommentInfoView;
use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::member::MemberInfoView;
use crate::data::view::member_invitation::MemberInvitationInfoView;
use crate::data::view::page::PageInfoView;
use crate::data::view::page_port::PageTranslationPortView;
use crate::data::view::system_mail::SystemMailInfoView;
use crate::data::view::team::TeamInfoView;
use crate::data::view::term::TermInfoView;
use crate::data::view::termbase::TermbaseInfoView;
use crate::data::view::termbase_port::TermbaseTermView;
use crate::data::view::unit::UnitInfoView;
use crate::data::view::unit_port::UnitTranslationPortView;
use crate::data::view::user::UserInfoView;
use crate::data::view::workset::WorksetInfoView;
use crate::value::announcement::AnnouncementInclOpt;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::chapter::ChapterInclOpt;
use crate::value::comic::{ComicInclOpt, ComicStatus, ComicWithOpt};
use crate::value::comment::CommentInclOpt;
use crate::value::member::MemberInclOpt;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleField;
use crate::value::unit::UnitTextPart;

/// Top-level `OpenAPI` document for the `PopRaKo` HTTP API.
#[derive(OpenApi)]
#[openapi(
    paths(
        handler::health::check_health,
        handler::health::detailed_metrics,
        handler::auth::register,
        handler::auth::login,
        handler::auth::logout,
        handler::user::get_my_info,
        handler::user::get_info,
        handler::user::update_info,
        handler::user::update_password,
        handler::user::delete,
        handler::user::alloc_avatar,
        handler::user::mark_avatar_uploaded,
        handler::team::create,
        handler::team::list_infos,
        handler::team::get_info,
        handler::team::update_info,
        handler::team::alloc_avatar,
        handler::team::mark_avatar_uploaded,
        handler::team::list_online_user_ids,
        handler::team::mark_self_online,
        handler::team::delete,
        handler::workset::create,
        handler::workset::list_infos,
        handler::workset::get_info,
        handler::workset::update_info,
        handler::workset::delete,
        handler::comic::create,
        handler::comic::list_infos,
        handler::comic::get_info,
        handler::comic::update_info,
        handler::comic::alloc_cover,
        handler::comic::mark_cover_uploaded,
        handler::comic::archive,
        handler::comic::export_archives,
        handler::comic::delete,
        handler::chapter::create,
        handler::chapter::list_infos,
        handler::chapter::get_pinned,
        handler::chapter::get_info,
        handler::chapter::list_workflow_record_infos,
        handler::chapter::update_info,
        handler::chapter::mark_pinned,
        handler::chapter::advance_stage,
        handler::chapter::delete,
        handler::chapter_port::alloc_artwork,
        handler::chapter_port::mark_artwork_uploaded,
        handler::chapter_port::export_artwork,
        handler::chapter_port::import_translation,
        handler::chapter_port::export_translation,
        handler::chapter_port::export_translation_download,
        handler::page::list_infos,
        handler::page::list_editted_diff_page_ids,
        handler::page::get_info,
        handler::page::delete,
        handler::page::alloc_chapter_pages,
        handler::page::alloc_image,
        handler::page::mark_image_uploaded,
        handler::unit::list_infos,
        handler::unit::search_infos,
        handler::unit::save_infos,
        handler::unit::transform,
        handler::assignment::list_infos,
        handler::assignment::update_roles,
        handler::assignment::delete,
        handler::assignment::join,
        handler::assignment_invitation::create,
        handler::assignment_invitation::list_infos,
        handler::assignment_invitation::delete,
        handler::assignment_invitation::join,
        handler::system_mail::list_infos,
        handler::system_mail::mark_read,
        handler::announcement::create,
        handler::announcement::list_infos,
        handler::announcement::update_info,
        handler::announcement::delete,
        handler::comment::create,
        handler::comment::list_infos,
        handler::termbase::create,
        handler::termbase::list_team_infos,
        handler::termbase::list_comic_infos,
        handler::termbase::get_info,
        handler::termbase::update_info,
        handler::termbase::delete,
        handler::termbase_port::export,
        handler::termbase_port::export_download,
        handler::termbase_port::import_team,
        handler::termbase_port::import_comic,
        handler::term::create,
        handler::term::list_infos,
        handler::term::get_info,
        handler::term::update_info,
        handler::term::delete,
        handler::member::create,
        handler::member::list_infos,
        handler::member::list_my_infos,
        handler::member::update_roles,
        handler::member::delete,
        handler::member::join,
        handler::member_invitation::create,
        handler::member_invitation::list_infos,
        handler::member_invitation::update_roles,
        handler::member_invitation::delete,
    ),
    components(schemas(
        HttpError,
        RegisterAuthInstr,
        RegisterAuthVal,
        LoginAuthInstr,
        LoginAuthVal,
        UserInfoView,
        UpdateUserInfoInstr,
        UpdateUserPasswordInstr,
        AllocUserAvatarInstr,
        AllocUserAvatarVal,
        MarkUserAvatarUploadedInstr,
        TeamInfoView,
        CreateTeamInstr,
        UpdateTeamInfoInstr,
        AllocTeamAvatarInstr,
        AllocTeamAvatarVal,
        MarkTeamAvatarUploadedInstr,
        WorksetInfoView,
        CreateWorksetInstr,
        CreateWorksetVal,
        UpdateWorksetInfoInstr,
        ComicInfoView,
        ComicStatus,
        ListComicInfosVal,
        CreateComicInstr,
        CreateComicVal,
        UpdateComicInfoInstr,
        AllocComicCoverInstr,
        AllocComicCoverVal,
        MarkComicCoverUploadedInstr,
        ArchiveComicVal,
        ChapterInfoView,
        ChapterWorkflowRecordEventView,
        ChapterWorkflowRecordInfoView,
        ChapterWorkflowRecordOriginView,
        ChapterWorkflowRecordStagePhaseView,
        ChapterWorkflowRecordStageView,
        ChapterWorkflowRecordTranslationFormatView,
        CreateChapterInstr,
        CreateChapterVal,
        UpdateChapterInfoInstr,
        ChapterStageInstr,
        ChapterStageOperInstr,
        UpdateChapterStageInstr,
        ChapterTranslationPortView,
        PageTranslationPortView,
        UnitTranslationPortView,
        ChapterTranslationFormatInstr,
        ChapterTranslationImportModeInstr,
        ImportChapterTranslationInstr,
        ImportChapterTranslationVal,
        PageInfoView,
        PageImageInstr,
        ImageUploadSlotView,
        AllocChapterPagesInstr,
        AllocChapterPagesVal,
        AllocPageImageInstr,
        AllocatedPageVal,
        MarkPageImageUploadedInstr,
        UnitInfoView,
        ListPageUnitInfosVal,
        ListEdittedDiffPageIdsVal,
        UnitEditInstr,
        UnitCoordInstr,
        UnitTranslationInstr,
        UnitRevisionInstr,
        UnitTextPart,
        TransformChapterUnitsInstr,
        UnitTransformInstr,
        UnitTextTransformInstr,
        AssignmentInfoView,
        JoinChapterAssignmentInstr,
        UpdateAssignmentRolesInstr,
        AssignmentInvitationInfoView,
        CreateAssignmentInvitationInstr,
        CreateAssignmentInvitationVal,
        JoinAssignmentInvitationInstr,
        SystemMailInfoView,
        MarkSystemMailReadInstr,
        AnnouncementInfoView,
        CreateAnnouncementInstr,
        CreateAnnouncementVal,
        UpdateAnnouncementInfoInstr,
        CommentInfoView,
        CreateCommentInstr,
        CreateCommentVal,
        TermbaseInfoView,
        CreateTermbaseInstr,
        CreateTermbaseVal,
        UpdateTermbaseInfoInstr,
        ImportTermInstr,
        ImportTermbaseInstr,
        TermbaseTermView,
        ExportTermbaseVal,
        ImportTermbaseVal,
        TermInfoView,
        CreateTermInstr,
        CreateTermVal,
        UpdateTermInfoInstr,
        MemberInfoView,
        CreateMemberInstr,
        CreateMemberVal,
        JoinTeamInstr,
        UpdateMemberRolesInstr,
        MemberInvitationInfoView,
        CreateMemberInvitationInstr,
        CreateMemberInvitationVal,
        UpdateMemberInvitationRolesInstr,
        ComicInclOpt,
        ComicWithOpt,
        ChapterInclOpt,
        MemberInclOpt,
        MemberInvitationInclOpt,
        AssignmentInclOpt,
        AnnouncementInclOpt,
        CommentInclOpt,
        RoleField,
    )),
    tags(
        (name = "health", description = "Health-check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "teams", description = "Team management endpoints"),
        (name = "worksets", description = "Workset management endpoints"),
        (name = "comics", description = "Comic management endpoints"),
        (name = "chapters", description = "Chapter management endpoints"),
        (name = "chapter-port", description = "Chapter translation and artwork import/export endpoints"),
        (name = "pages", description = "Page management endpoints"),
        (name = "units", description = "Page unit endpoints"),
        (name = "members", description = "Member management endpoints"),
        (
            name = "member-invitations",
            description = "Member invitation endpoints",
        ),
        (name = "assignments", description = "Assignment endpoints"),
        (
            name = "assignment-invitations",
            description = "Assignment invitation endpoints",
        ),
        (name = "system-mails", description = "System mail endpoints"),
        (name = "announcements", description = "Announcement endpoints"),
        (name = "comments", description = "Comment endpoints"),
        (name = "termbase-port", description = "Terminology-base import/export endpoints"),
    )
)]
pub struct ApiDoc;
