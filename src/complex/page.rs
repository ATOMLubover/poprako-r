//! Pure rules for page entities.

/// Pure chapter-page manifest matching.
pub mod manifest;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee, check_user_is_team_admin,
    check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::role::RoleField;

// Build and log one expected page-role permission error.
fn reject_role(message_key: &str, event: &'static str) -> BaseRest<()> {
    //
    let err_message = trl(message_key);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Perm,
        err_message = %err_message,
        event,
        "expected page permission error",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: err_message,
    })
}

// Verify that assignment evidence permits allocating page images.
fn check_alloc_role(assignment_info: &AssignmentInfo) -> BaseRest<()> {
    //
    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER, RoleField::REVIEWER])
    {
        return reject_role(
            "error-page-alloc-role-required",
            "page_alloc_role_missing",
        );
    }

    accept(())
}

// Verify that assignment evidence permits uploading page images.
fn check_upload_role(assignment_info: &AssignmentInfo) -> BaseRest<()> {
    //
    if !assignment_info
        .roles
        .has_any_role(&[RoleField::RAW_PROVIDER])
    {
        return reject_role(
            "error-page-upload-role-required",
            "page_upload_role_missing",
        );
    }

    accept(())
}

/// Pure domain operations for page entities.
pub struct PageComplex;

impl PageComplex {
    /// Rejects empty names, control characters, and path separators.
    pub fn ensure_raw_ident(raw_ident: Option<&str>) -> BaseRest<()> {
        //
        let Some(value) = raw_ident else {
            return accept(());
        };

        if value.trim().is_empty()
            || value.chars().any(|character| {
                character.is_control() || matches!(character, '/' | '\\')
            })
        {
            //
            let message = trl("error-invalid-page-raw-ident");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %message,
                "invalid original page filename",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }

        accept(())
    }

    /// Generate a unique page identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

/// Evidence that grants page-list access.
pub enum PageListAccess<'a> {
    //
    /// Access through team membership.
    Member {
        /// Team membership used to establish access.
        member_info: &'a MemberInfo,
    },

    /// Access through an assignment on the chapter.
    Assignee {
        /// Chapter assignment used to establish access.
        assignment_info: &'a AssignmentInfo,
    },
}

/// Pure permission rules for page entities.
pub struct PagePermComplex;

impl PagePermComplex {
    /// Verify the caller may allocate page images for the chapter.
    pub fn ensure_user_can_alloc(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        check_alloc_role(assignment_info)
    }

    /// Verify the caller may list pages under a chapter.
    pub const fn ensure_user_can_list_infos(
        access: &PageListAccess<'_>,
    ) -> BaseRest<()> {
        //
        match access {
            //
            PageListAccess::Member { member_info } => {
                check_user_is_team_member(member_info)
            }

            PageListAccess::Assignee { assignment_info } => {
                check_user_is_chapter_assignee(assignment_info)
            }
        }
    }

    /// Verify the caller may confirm a page image upload.
    pub fn ensure_user_can_mark_image_uploaded(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        check_upload_role(assignment_info)
    }

    /// Verify the caller may delete all pages under the chapter.
    pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_admin(member_info)
    }
}
