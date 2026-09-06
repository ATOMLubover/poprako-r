//! Pure chapter artwork allocation and permission rules.

use poprako_util::i18n::trl;

use crate::config::artwork::ArtworkConfig;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::role::RoleField;

/// Pure rules for artwork submissions.
pub struct ChapterArtworkComplex;

impl ChapterArtworkComplex {
    /// Validates arbitrary file suffixes without treating them as path segments.
    pub fn valid_extension(ext: &str) -> bool {
        //
        !ext.is_empty()
            && ext.len() <= 32
            && ext.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
            })
    }

    /// Constructs one classified, translated artwork error at its production leaf.
    pub fn artwork_error(variant: ExpectedVariant, key: &str) -> BaseError {
        //
        let message = trl(key);

        tracing::warn!(err_variant = ?variant, err_message = %message, "chapter artwork rejected");

        BaseError::Expected { variant, message }
    }

    /// Checks the exact upload size and extension before allocating an object.
    pub fn ensure_allocation(
        config: ArtworkConfig,
        byte_len: u64,
        ext: &str,
    ) -> BaseRest<()> {
        //
        let limit = config.chapter_artwork_limit.checked_mul(1024 * 1024);

        if byte_len == 0
            || limit.is_none_or(|limit| byte_len > limit)
            || byte_len > i64::MAX as u64
            || !Self::valid_extension(ext)
        {
            //
            return Err(Self::artwork_error(
                ExpectedVariant::Args,
                "error-invalid-artwork-upload",
            ));
        }

        accept(())
    }

    /// Requires a chapter typesetter, redrawer, or administrator assignment.
    pub fn ensure_user_can_upload(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        //
        let can_upload = assignment_info.roles.has_any_role(&[
            RoleField::TYPESETTER,
            RoleField::REDRAWER,
            RoleField::ADMIN,
        ]);

        if !can_upload {
            //
            return Err(Self::artwork_error(
                ExpectedVariant::Perm,
                "error-artwork-upload-role-required",
            ));
        }

        accept(())
    }
}
