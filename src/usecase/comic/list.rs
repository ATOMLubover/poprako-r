use std::collections::HashMap;

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::comic::ComicPermComplex;
use crate::data::instr::comic::ListComicInfosInstr;
use crate::data::val::comic_list::ListComicInfosVal;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part::repo::oper::comic::ListComicInfos;
use crate::part::repo::page::PageRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::page::PinnedChapterSnapshot;
use crate::usecase::internal::util::LoadMode;
use crate::usecase::internal::view::{ObjViewIds, ObjViewSnapshot};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::comic::ComicWithOpt;

/// Lists comics for a workset with optional filters and derived data.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListComicInfosInstr,
) -> BaseRest<ListComicInfosVal>
where
    C: Context,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let (with_pinned_chapter, with_pinned_chapter_assignment) = (
        instr.with_opt.contains(&ComicWithOpt::PinnedChapter),
        instr
            .with_opt
            .contains(&ComicWithOpt::PinnedChapterAssignment),
    );

    validate_with_options(
        &instr,
        &token.user_id,
        with_pinned_chapter,
        with_pinned_chapter_assignment,
    )?;

    let member_info = MemberLoader::load_info_from_workset(
        repo,
        LoadMode::Run,
        &token.user_id,
        &instr.workset_id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_list_infos(&member_info)?;

    let spec = instr.try_into()?;

    let comic_infos = ListComicInfos { spec: &spec }.run_on(repo).await?;

    let comic_ids = comic_infos
        .iter()
        .map(|comic_info| comic_info.id.as_str())
        .collect::<Vec<_>>();

    // NOTE: `with` cannot be executed elegantly by repo layer,
    // so we have to handle it in usecase layer.
    let pinned_chapter_snapshot = match (with_pinned_chapter,) {
        //
        (true,) => Some(
            PinnedChapterSnapshot::load_from_comics(repo, &comic_ids).await?,
        ),

        (false,) => None,
    };

    let pinned_chapter_infos = pinned_chapter_snapshot
        .as_ref()
        .map(PinnedChapterSnapshot::infos_by_comic_id);

    let pinned_chapter_assignment_infos = if with_pinned_chapter_assignment {
        //
        let chapter_ids = pinned_chapter_infos
            .into_iter()
            .flat_map(|pinned_chapter_infos| pinned_chapter_infos.values())
            .map(|chapter_info| chapter_info.id.as_str())
            .collect::<Vec<_>>();

        let assignment_incls = [AssignmentInclOpt::User];

        let assignment_infos = ListAssignmentInfos::Chapters {
            chapter_ids: &chapter_ids,
            incls: &assignment_incls,
        }
        .run_on(repo)
        .await?;

        let mut assignment_infos_by_chapter = HashMap::new();

        for assignment_info in assignment_infos {
            //
            assignment_infos_by_chapter
                .entry(assignment_info.chapter_id.clone())
                .or_insert_with(Vec::new)
                .push(assignment_info);
        }

        assignment_infos_by_chapter
    } else {
        HashMap::new()
    };

    let mut obj_view_ids = ObjViewIds::default();

    obj_view_ids.collect_comics(&comic_infos);

    obj_view_ids.collect_chapters(
        pinned_chapter_infos
            .into_iter()
            .flat_map(|infos| infos.values()),
    );

    obj_view_ids.collect_assignments(
        pinned_chapter_assignment_infos.values().flatten(),
    );

    let obj_view_snapshot =
        ObjViewSnapshot::load_with_comic_fallbacks::<C, R, O>(
            repo,
            obj_dept,
            obj_view_ids,
            pinned_chapter_snapshot.as_ref(),
        )
        .await?;

    accept(build_list_val(
        &obj_view_snapshot,
        comic_infos,
        pinned_chapter_snapshot
            .map(PinnedChapterSnapshot::into_infos_by_comic_id)
            .unwrap_or_default(),
        pinned_chapter_assignment_infos,
    ))
}

// Validate dependencies between optional pinned-chapter response fields.
fn validate_with_options(
    instr: &ListComicInfosInstr,
    user_id: &str,
    with_pinned_chapter: bool,
    with_pinned_chapter_assignment: bool,
) -> BaseRest<()> {
    //
    if !with_pinned_chapter_assignment || with_pinned_chapter {
        return accept(());
    }

    let err_message = trl("error-pinned-chapter-with-required");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        workset_id = %instr.workset_id,
        user_id = %user_id,
        with_pinned_chapter,
        with_pinned_chapter_assignment,
        "expected error: pinned chapter assignment requires pinned chapter",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    })
}

// Build aligned comic, pinned-chapter, and assignment response vectors.
fn build_list_val(
    obj_view_snapshot: &ObjViewSnapshot,
    comic_infos: Vec<ComicInfo>,
    mut pinned_chapter_infos: HashMap<String, ChapterInfo>,
    mut assignment_infos_by_chapter: HashMap<String, Vec<AssignmentInfo>>,
) -> ListComicInfosVal {
    //
    let mut comic_info_views = Vec::with_capacity(comic_infos.len());

    let mut pinned_chapter_views = Vec::with_capacity(comic_infos.len());

    let mut pinned_chapter_assignment_views =
        Vec::with_capacity(comic_infos.len());

    for comic_info in comic_infos {
        //
        let chapter_info = pinned_chapter_infos.remove(&comic_info.id);

        let assignment_infos = chapter_info
            .as_ref()
            .and_then(|chapter_info| {
                assignment_infos_by_chapter.remove(&chapter_info.id)
            })
            .unwrap_or_default();

        let assignment_views = assignment_infos
            .into_iter()
            .map(|assignment_info| {
                obj_view_snapshot.assignment(assignment_info)
            })
            .collect();

        let pinned_chapter_view = chapter_info
            .map(|chapter_info| obj_view_snapshot.chapter(chapter_info));

        comic_info_views.push(obj_view_snapshot.comic(comic_info));

        pinned_chapter_views.push(pinned_chapter_view);

        pinned_chapter_assignment_views.push(assignment_views);
    }

    ListComicInfosVal {
        comics: comic_info_views,
        pinned_chapters: pinned_chapter_views,
        pinned_chapter_assignments: pinned_chapter_assignment_views,
    }
}
