use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee,
    check_user_is_chapter_translator_or_proofreader, check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Concrete evidence that grants chapter export access.
pub enum ChapterExportAccess<'a> {
    //
    /// Access through team membership.
    Member {
        /// Team membership used to establish access.
        member_info: &'a MemberInfo,
    },

    /// Access through a chapter assignment.
    Assignee {
        /// Chapter assignment used to establish access.
        assignment_info: &'a AssignmentInfo,
    },
}

/// Chapter import and export perm rules.
pub struct ChapterPortPermComplex;

impl ChapterPortPermComplex {
    /// Verify the caller may export chapter translations.
    pub fn ensure_user_can_export_translation(
        access: &ChapterExportAccess<'_>,
    ) -> BaseRest<()> {
        //
        let access_check = match access {
            //
            ChapterExportAccess::Member { member_info } => {
                check_user_is_team_member(member_info)
            }

            ChapterExportAccess::Assignee { assignment_info } => {
                check_user_is_chapter_assignee(assignment_info)
            }
        };

        match access_check {
            //
            Ok(()) => accept(()),

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => {
                //
                let err_message =
                    trl("error-chapter-port-export-perm-required");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    operation = "export",
                    "expected error: chapter port export perm required",
                );

                Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                })
            }

            Err(e) => Err(e),
        }
    }

    /// Verify the caller may import chapter translations.
    pub fn ensure_user_can_import_translation(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        //
        match check_user_is_chapter_translator_or_proofreader(assignment_info) {
            //
            Ok(()) => accept(()),

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => {
                //
                let err_message =
                    trl("error-chapter-port-import-perm-required");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    operation = "import",
                    "expected error: chapter port import perm required",
                );

                Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                })
            }

            Err(e) => Err(e),
        }
    }
}
