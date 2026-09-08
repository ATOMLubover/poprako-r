//! Announcement use cases.

/// Announcement presentation assembly.
pub mod view;

#[cfg(test)]
// Unit tests for announcement usecase behavior.
mod tests;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::announcement::{
    AnnouncementComplex, AnnouncementPermComplex,
};
use crate::data::instr::announcement::{
    CreateAnnouncementInstr, ListAnnouncementInfosInstr,
    UpdateAnnouncementInfoInstr,
};
use crate::data::val::announcement::CreateAnnouncementVal;
use crate::data::view::announcement::AnnouncementInfoView;
use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::announcement::{AnnouncementEntry, AnnouncementRepl};
use crate::part::obj_dept::UserAvatar;
use crate::part::repo::announcement::AnnouncementRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, DeleteAnnouncement, GetAnnouncementInfo,
    ListAnnouncementInfos, UpdateAnnouncement,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::announcement::view::announcement_info_views;

/// Lists announcements under a team.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListAnnouncementInfosInstr,
) -> BaseRest<Vec<AnnouncementInfoView>>
where
    C: Context,
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let announcement_list_spec = Into::<AnnouncementListSpec>::into(instr);

    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &announcement_list_spec.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    };

    AnnouncementPermComplex::ensure_user_can_list_infos(&member_info)?;

    let announcement_infos = ListAnnouncementInfos {
        spec: &announcement_list_spec,
    }
    .run_on(repo)
    .await?;

    let announcement_info_vals =
        announcement_info_views(obj_dept, announcement_infos).await?;

    accept(announcement_info_vals)
}

/// Creates an announcement under a team.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn create<C, R>(
    repo: &R,
    token: UserToken,
    instr: CreateAnnouncementInstr,
) -> BaseRest<CreateAnnouncementVal>
where
    C: Context,
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    AnnouncementPermComplex::ensure_user_can_create(&member_info)?;

    let announcement_entry = AnnouncementEntry {
        id: AnnouncementComplex::gen_id(),
        team_id: instr.team_id,
        user_id: token.user_id,
        title: instr.title,
        content: instr.content,
    };

    let announcement_info = CreateAnnouncement {
        entry: &announcement_entry,
    }
    .run_on(repo)
    .await?;

    accept(CreateAnnouncementVal {
        id: announcement_info.id,
    })
}

/// Replaces an announcement's editable fields.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    instr: UpdateAnnouncementInfoInstr,
) -> BaseRest<()>
where
    C: Context,
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
{
    let announcement_info =
        GetAnnouncementInfo { id: &instr.id }.run_on(repo).await?;

    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &announcement_info.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            team_id = %announcement_info.team_id,
            user_id = %token.user_id,
            announcement_id = %announcement_info.id,
            "expected error: announcement updater membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    AnnouncementPermComplex::ensure_user_can_update_info(&member_info)?;

    let announcement_repl = AnnouncementRepl {
        id: instr.id,
        title: instr.title,
        content: instr.content,
    };

    UpdateAnnouncement {
        update: &announcement_repl,
    }
    .run_on(repo)
    .await?;

    accept(())
}

/// Deletes an announcement.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context,
    R: AnnouncementRepo<C> + MemberRepo<C> + Sync,
{
    let announcement_info =
        GetAnnouncementInfo { id: &id }.run_on(repo).await?;

    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &announcement_info.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            team_id = %announcement_info.team_id,
            user_id = %token.user_id,
            announcement_id = %announcement_info.id,
            "expected error: announcement deleter membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    AnnouncementPermComplex::ensure_user_can_delete(&member_info)?;

    DeleteAnnouncement {
        id: &announcement_info.id,
    }
    .run_on(repo)
    .await?;

    accept(())
}
