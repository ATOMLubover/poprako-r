//! Team use cases — create, read, update, avatar management, and deletion.

/// Team deletion orchestration.
pub mod delete;
/// Process-local online-user lease use cases.
pub mod online;
/// Non-transactional team read use cases.
pub mod read;
/// Team presentation assembly.
pub mod view;

#[cfg(test)]
// Unit and integration tests for team management policies.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::key::ObjGen;
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::oper::{GenObjSlot, MarkObjUploaded};
use poprako_obj_dept::{ObjDept, ObjDeptView};
use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::member::MemberComplex;
use crate::complex::team::{TeamComplex, TeamPermComplex};
use crate::config::image::ImageConfig;
use crate::data::instr::team::{
    AllocTeamAvatarInstr, CreateTeamInstr, MarkTeamAvatarUploadedInstr,
    UpdateTeamInfoInstr,
};
use crate::data::val::team::AllocTeamAvatarVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::team::TeamInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::member::MemberEntry;
use crate::model::write::team::{TeamEntry, TeamRepl};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::TeamAvatar;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::{CreateMember, FindMemberInfo};
use crate::part::repo::oper::team::{
    CreateTeam, GetTeamInfoExcluded, UpdateTeam,
};
use crate::part::repo::oper::user::{GetUserInfo, GetUserInfoExcluded};
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::team::view::team_info_view;
use crate::value::image::{ImageKind, TeamAvatarKey};
use crate::value::role::{RoleField, RoleMask};

/// Creates a new team.
///
/// Transactional — inserts the team and makes the creator an admin member.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `O` — Resolves the avatar signed URL through `ObjDept`.
#[instrument(level = "info", skip(nucl, repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn create<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    instr: CreateTeamInstr,
) -> BaseRest<TeamInfoView>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C> + UserRepo<C> + MemberRepo<C> + Send + Sync,
    O: ObjDeptView<TeamAvatar, C> + Sync,
{
    let user_info = GetUserInfo { id: &token.user_id }.run_on(repo).await?;

    TeamPermComplex::ensure_user_can_create(&user_info)?;

    let team_entry = TeamEntry {
        id: TeamComplex::gen_id(),
        name: instr.name,
        description: instr.description,
    };

    let team_info = nucl
        .coord(async move |context| {
            //
            let user_info = GetUserInfoExcluded { id: &token.user_id }
                .step_on(repo, context)
                .await?;

            let team_info = CreateTeam { entry: &team_entry }
                .step_on(repo, context)
                .await?;

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: token.user_id,
                user_nickname: user_info.nickname,
                team_id: team_info.id.clone(),
                roles: RoleMask::from(RoleField::ADMIN),
            };

            CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(team_info)
        })
        .await?;

    // FIXME: no need to use info val in create.
    team_info_view(obj_dept, team_info).await
}

/// Updates a team's name and description.
///
/// Non-transactional single-row update.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: UpdateTeamInfoInstr,
) -> BaseRest<()>
where
    C: Context,
    R: TeamRepo<C> + MemberRepo<C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &instr.id,
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

    TeamPermComplex::ensure_user_can_update_info(&member_info)?;

    let team_repl = TeamRepl {
        id: instr.id,
        name: instr.name,
        description: instr.description,
    };

    UpdateTeam { repl: &team_repl }.run_on(repo).await?;

    accept(())
}

/// Allocates a new avatar upload slot for a team.
///
/// Transactional flow:
///
/// 1. Calls [`AllocTeamAvatarInstr`] — updates the avatar key, increments
///    the version, and returns any previous avatar key for cleanup.
/// 2. If replacing an existing avatar, defers an immediate image-delete payload.
/// 3. Defers an image upload-check payload with a 15-minute delay.
///
/// After commit, generates a signed PUT URL for the client to upload to.
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Coordination nucleus.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred image opers.
/// * `O: ObjDept` — Allocates the avatar object and its signed upload URL.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, image_config, token),
    fields(actor_user_id = %token.user_id),
)]
pub async fn alloc_avatar<N, C, R, O>(
    (nucl, repo, obj_dept, image_config): (&N, &R, &O, &ImageConfig),
    token: UserToken,
    id: String,
    instr: AllocTeamAvatarInstr,
) -> BaseRest<AllocTeamAvatarVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: TeamRepo<C> + MemberRepo<C> + Send + Sync,
    O: ObjDept<TeamAvatar, C> + Send + Sync,
{
    ImageComplex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::TeamAvatar,
    )?;

    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &id,
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

    TeamPermComplex::ensure_user_can_alloc_avatar(&member_info)?;

    let obj_slot = nucl
        .coord(async move |context| {
            //
            GetTeamInfoExcluded { id: &id }
                .step_on(repo, context)
                .await?;

            let obj_spec = ObjSlotSpec {
                dom: TeamAvatarKey {
                    team_id: id.clone(),
                    ext: instr.ext,
                },
                hash: instr.image_hash.as_bytes(),
                content_type: instr.ext.content_type(),
                byte_len: instr.new_byte_len,
            };

            GenObjSlot::<TeamAvatar>::new(&obj_spec)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)
        })
        .await?;

    let slot = obj_slot.map(|obj_slot| ImageUploadSlotView {
        put_url: obj_slot.url.to_string(),
        image_ver: obj_slot.key.ver,
        headers: obj_slot.headers,
    });

    accept(AllocTeamAvatarVal { slot })
}

/// Optimistically marks the requested current avatar generation as uploaded.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn mark_avatar_uploaded<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    id: String,
    instr: MarkTeamAvatarUploadedInstr,
) -> BaseRest<()>
where
    C: Context,
    R: MemberRepo<C>,
    O: ObjDept<TeamAvatar, C> + Sync,
{
    let member_info = FindMemberInfo {
        user_id: &token.user_id,
        team_id: &id,
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

    TeamPermComplex::ensure_user_can_mark_avatar_uploaded(&member_info)?;

    // SAFETY: This is an optimistic exact-generation transition. It does not
    // synchronously prove PUT success, object presence, or content integrity;
    // the delayed actor may reset this generation after a failed HEAD check.
    let avatar_key = ObjGen {
        id,
        ver: instr.image_ver,
    };

    let marked = MarkObjUploaded::<TeamAvatar>::new(&avatar_key)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    marked.then_some(()).ok_or_else(|| BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-stale-avatar-upload"),
    })
}
