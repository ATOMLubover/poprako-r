use poprako_orchestra::Oper;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::spec::assignment::AssignmentListSpec;
use crate::model::write::assignment::{AssignmentEntry, AssignmentRoleRepl};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

/// Finds an optional assignment.
#[derive(Oper)]
#[oper(output = Option<AssignmentInfo>)]
pub enum FindAssignmentInfo<'a, 'b> {
    //
    /// Finds by chapter and user.
    ChapterUser {
        /// Chapter identifier.
        chapter_id: &'a str,
        /// User identifier.
        user_id: &'a str,
    },

    /// Finds by user and comic.
    UserComic {
        /// User identifier.
        user_id: &'a str,
        /// Comic identifier.
        comic_id: &'a str,
        /// Assignment inclusion options.
        incls: &'b [AssignmentInclOpt],
    },
}

/// Gets an assignment that must exist.
#[derive(Oper)]
#[oper(output = AssignmentInfo)]
pub struct GetAssignmentInfo<'a, 'b> {
    //
    /// Assignment identifier.
    pub id: &'a str,
    /// Assignment inclusion options.
    pub incls: &'b [AssignmentInclOpt],
}

/// Lists assignments selected by a query specification or chapter set.
#[derive(Oper)]
#[oper(output = Vec<AssignmentInfo>)]
pub enum ListAssignmentInfos<'a, 'b> {
    //
    /// Lists by a query specification.
    Spec {
        /// Query specification for filtering assignments.
        spec: &'a AssignmentListSpec,
    },

    /// Lists by a single chapter.
    Chapter {
        /// Chapter identifier.
        chapter_id: &'a str,
        /// Optional role filter.
        role: Option<RoleField>,
        /// Assignment inclusion options.
        incls: &'b [AssignmentInclOpt],
    },

    /// Lists by a set of chapters.
    Chapters {
        /// Chapter identifiers.
        chapter_ids: &'a [&'a str],
        /// Assignment inclusion options.
        incls: &'b [AssignmentInclOpt],
    },
}

/// Lists and exclusively locks all assignment rows under a chapter.
#[derive(Oper)]
#[oper(output = Vec<AssignmentInfo>)]
pub struct ListAssignmentInfosExcluded<'a> {
    /// Chapter identifier.
    pub chapter_id: &'a str,
}

/// Creates an assignment.
#[derive(Oper)]
#[oper(output = AssignmentInfo)]
pub struct CreateAssignment<'a> {
    /// The assignment entry data.
    pub entry: &'a AssignmentEntry,
}

/// Replaces the roles assigned to an assignment.
#[derive(Oper)]
#[oper(output = AssignmentInfo)]
pub struct UpdateAssignmentRoles<'a> {
    /// The role update data.
    pub update: &'a AssignmentRoleRepl,
}

/// Deletes assignments selected by identifier or chapter.
#[derive(Oper)]
#[oper(output = ())]
pub enum DeleteAssignments<'a> {
    //
    /// Deletes by assignment identifier.
    Id {
        /// Assignment identifier.
        id: &'a str,
    },

    /// Deletes by chapter.
    Chapter {
        /// Chapter identifier.
        chapter_id: &'a str,
    },
}
