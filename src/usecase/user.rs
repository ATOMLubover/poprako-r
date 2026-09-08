//! User use cases — profile, avatar management, activity tracking, and deletion.

/// User deletion use case.
pub mod delete;
/// User presentation assembly.
pub mod view;

#[cfg(test)]
// Unit tests for account, role, and membership operations.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::key::ObjGen;
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::oper::{GenObjSlot, MarkObjUploaded};
use poprako_obj_dept::{ObjDept, ObjDeptView};
use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::user::UserComplex;
use crate::config::image::ImageConfig;
use crate::data::instr::user::{
    AllocUserAvatarInstr, MarkUserAvatarUploadedInstr, UpdateUserInfoInstr,
    UpdateUserPasswordInstr,
};
use crate::data::val::user::AllocUserAvatarVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::data::view::user::UserInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::member::MemberNicknameRepl;
use crate::model::write::user::{UserCredsRepl, UserInfoRepl};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::UserAvatar;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::UpdateMember;
use crate::part::repo::oper::user::{
    GetUserCredential, GetUserInfo, GetUserInfoExcluded, UpdateUser,
};
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::user::view::user_info_view;
use crate::value::image::{ImageKind, UserAvatarKey};

/// Fetches a user's profile with avatar URL resolution.
///
/// Non-transactional read without activity updates, including self-reads.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — User storage.
/// * `O` — Resolves the avatar signed URL through `ObjDept`.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn get_info<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<UserInfoView>
where
    C: Context,
    R: UserRepo<C>,
    O: ObjDeptView<UserAvatar, C> + Sync,
{
    let user_info = GetUserInfo::Id { id: &id }.run_on(repo).await?;

    user_info_view(obj_dept, user_info).await
}

/// Updates a user's QQ ID and nickname.
///
/// Transactional flow:
///
/// 1. **Permission check:** the caller (`token.user_id`) must match the
///    target user (`instr.id`). Returns `Perm` error on mismatch.
/// 2. Updates the user's own record via [`UpdateUser::Info`].
/// 3. Propagates the new nickname to all of the user's memberships via
///    [`UpdateMember::UserNickname`].
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateUserInfoInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
{
    // Only the user themselves can update their own profile.
    if token.user_id != instr.id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %instr.id,
            "expected error: user profile ownership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    let user_repl = UserInfoRepl {
        id: token.user_id,
        qid: instr.qid,
        nickname: instr.nickname,
    };

    let member_repl = MemberNicknameRepl {
        user_id: user_repl.id.clone(),
        user_nickname: user_repl.nickname.clone(),
    };

    nucl.coord(async move |context| {
        //
        UpdateUser::Info { repl: &user_repl }
            .step_on(repo, context)
            .await?;

        UpdateMember::UserNickname { repl: &member_repl }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Replaces a user's password after verifying their current password.
#[instrument(
    level = "info",
    skip(nucl, repo, token, instr),
    fields(
        actor_user_id = %token.user_id,
        current_password = "[REDACTED]",
        new_password = "[REDACTED]",
    )
)]
pub async fn update_password<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    user_id: String,
    instr: UpdateUserPasswordInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: UserRepo<C> + Send + Sync,
{
    if token.user_id != user_id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %user_id,
            "expected error: password update ownership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    let user_info = GetUserInfo::Id { id: &user_id }.run_on(repo).await?;

    let user_credential = GetUserCredential::Qid {
        qid: &user_info.qid,
    }
    .run_on(repo)
    .await?;

    if !UserComplex::verify_password(
        &instr.current_password,
        &user_credential.password_hash,
    )
    .await
    {
        let err_message = trl("error-wrong-credentials");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Auth,
            err_message = %err_message,
            user_id = %user_id,
            qid = %user_info.qid,
            "expected error: current password verification failed",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Auth,
            message: err_message,
        });
    }

    let password_hash = UserComplex::hash_password(&instr.new_password).await?;

    let credentials_repl = UserCredsRepl {
        id: user_id,
        password_hash,
    };

    nucl.coord(async move |context| {
        //
        UpdateUser::PasswordHash {
            repl: &credentials_repl,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Allocates a new avatar upload slot for a user.
///
/// Transactional flow (same pattern as [`team::alloc_avatar`]):
///
/// 1. Calls [`AllocUserAvatarInstr`] — generates an object key, increments
///    the version, and returns any previous avatar key.
/// 2. If replacing, defers an image-delete payload.
/// 3. Defers an image upload-check payload with a 15-minute delay.
///
/// After commit, generates a signed PUT URL.
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — User storage.
/// * `P: Prom<C>` — Prom enqueuer.
/// * `O: ObjDept` — Allocates the avatar object and its signed upload URL.
///
/// [`team::alloc_avatar`]: super::team::alloc_avatar
#[instrument(level = "info", skip(nucl, repo, obj_dept, image_config, token), fields(actor_user_id = %token.user_id))]
pub async fn alloc_avatar<N, C, R, O>(
    (nucl, repo, obj_dept, image_config): (&N, &R, &O, &ImageConfig),
    token: UserToken,
    instr: AllocUserAvatarInstr,
) -> BaseRest<AllocUserAvatarVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: UserRepo<C> + Send + Sync,
    O: ObjDept<UserAvatar, C> + Send + Sync,
{
    ImageComplex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::UserAvatar,
    )?;

    let obj_slot = nucl
        .coord(async move |context| {
            //
            GetUserInfoExcluded::Id { id: &token.user_id }
                .step_on(repo, context)
                .await?;

            let obj_spec = ObjSlotSpec {
                dom: UserAvatarKey {
                    user_id: token.user_id.clone(),
                    ext: instr.ext,
                },
                hash: instr.image_hash.as_bytes(),
                content_type: instr.ext.content_type(),
                byte_len: instr.new_byte_len,
            };

            GenObjSlot::<UserAvatar>::new(&obj_spec)
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

    accept(AllocUserAvatarVal { slot })
}

/// Optimistically marks the requested current avatar generation as uploaded.
#[instrument(level = "info", skip(obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn mark_avatar_uploaded<C, O>(
    (obj_dept,): (&O,),
    token: UserToken,
    id: String,
    instr: MarkUserAvatarUploadedInstr,
) -> BaseRest<()>
where
    C: Context,
    O: ObjDept<UserAvatar, C> + Sync,
{
    if token.user_id != id {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    // SAFETY: This is an optimistic exact-generation transition. It does not
    // synchronously prove PUT success, object presence, or content integrity;
    // the delayed actor may reset this generation after a failed HEAD check.
    let avatar_key = ObjGen {
        id,
        ver: instr.image_ver,
    };

    let marked = MarkObjUploaded::<UserAvatar>::new(&avatar_key)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    marked.then_some(()).ok_or_else(|| BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-stale-avatar-upload"),
    })
}
