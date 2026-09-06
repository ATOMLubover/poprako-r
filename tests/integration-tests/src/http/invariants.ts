import * as assert from "@std/assert";

import { stagePhase } from "../state/stages.ts";
import {
    exportPoprako,
    getChapter,
    getComic,
    getPinnedChapter,
    getTeam,
    getWorkset,
    listChapterAssignments,
    listChapterPages,
    listComicChapters,
    listPageUnits,
    listSystemMails,
    listTeamWorksets,
    listWorksetComics,
} from "./fixtures.ts";
import type { ApiClient } from "./apiClient.ts";
import type {
    ChapterInfoView,
    ComicInfoView,
    MemberInfoView,
    PageInfoView,
    SystemMailInfoView,
    UnitInfoView,
    WorksetInfoView,
} from "./types.ts";
import type { StageName } from "../state/stages.ts";

// Reusable invariant assertions (J1–J6). Each invariant
// is a pure function over API state: call it after any mutation to verify the
// domain counters/indexes/uniqueness stayed consistent. They never mutate.

// ---------- J1. Team invariant ----------

export async function assertTeamInvariant(
    api: ApiClient,
    teamId: string,
    opts: { memberCanAccess: boolean } = { memberCanAccess: true },
): Promise<void> {
    await getTeam(api, teamId);

    const worksets = await listTeamWorksets(api, teamId);

    // active workset ids and indexes are unique
    const worksetIds = new Set<string>();
    const worksetIndexes = new Set<number>();

    for (const ws of worksets) {
        assert.assert(!worksetIds.has(ws.id), `duplicate workset id ${ws.id}`);
        worksetIds.add(ws.id);

        assert.assert(!worksetIndexes.has(ws.index), `duplicate workset index ${ws.index}`);
        worksetIndexes.add(ws.index);

        assert.assertEquals(ws.team_id, teamId);
    }

    if (!opts.memberCanAccess) {
        // Caller asserts the 403 separately; nothing more to check here.
        return;
    }
}

// ---------- J2. Workset invariant ----------

export async function assertWorksetInvariant(api: ApiClient, worksetId: string): Promise<void> {
    const workset = await getWorkset(api, worksetId);

    const comics = await listWorksetComics(api, worksetId);

    assert.assertEquals(
        workset.comic_count,
        comics.length,
        `comic_count ${workset.comic_count} != active comics ${comics.length}`,
    );

    const comicIds = new Set<string>();
    const comicIndexes = new Set<number>();

    for (const comic of comics) {
        assert.assert(!comicIds.has(comic.id), `duplicate comic id ${comic.id}`);
        comicIds.add(comic.id);

        assert.assert(!comicIndexes.has(comic.index), `duplicate comic index ${comic.index}`);
        comicIndexes.add(comic.index);

        assert.assertEquals(comic.workset_id, worksetId);
    }
}

// ---------- J3. Comic invariant ----------

export async function assertComicInvariant(api: ApiClient, comicId: string): Promise<void> {
    const comic = await getComic(api, comicId);

    const chapters = await listComicChapters(api, comicId);

    assert.assertEquals(
        comic.chapter_count,
        chapters.length,
        `chapter_count ${comic.chapter_count} != active chapters ${chapters.length}`,
    );

    const chapterIds = new Set<string>();
    const chapterIndexes = new Set<number>();
    let pinnedCount = 0;
    let pinnedId: string | null = null;

    for (const chapter of chapters) {
        assert.assert(!chapterIds.has(chapter.id), `duplicate chapter id ${chapter.id}`);
        chapterIds.add(chapter.id);

        assert.assert(!chapterIndexes.has(chapter.index), `duplicate chapter index ${chapter.index}`);
        chapterIndexes.add(chapter.index);

        assert.assertEquals(chapter.comic_id, comicId);

        if (chapter.is_pinned) {
            pinnedCount += 1;
            pinnedId = chapter.id;
        }
    }

    assert.assert(pinnedCount <= 1, `at most one pinned chapter, found ${pinnedCount}`);

    // pinned endpoint consistency
    const pinnedEndpoint = await getPinnedChapter(api, comicId);

    if (pinnedId) {
        assert.assertEquals(pinnedEndpoint?.id ?? null, pinnedId, "pinned endpoint mismatch");
    } else {
        assert.assert(pinnedEndpoint === null, "pinned endpoint should be null when none pinned");
    }
}

// ---------- J4. Chapter invariant ----------

export async function assertChapterInvariant(api: ApiClient, chapterId: string): Promise<void> {
    const chapter = await getChapter(api, chapterId);

    const pages = await listChapterPages(api, chapterId);

    assert.assertEquals(
        chapter.page_count,
        pages.length,
        `page_count ${chapter.page_count} != active pages ${pages.length}`,
    );

    const pageIds = new Set<string>();
    const pageIndexes = new Set<number>();
    let sumTotal = 0;
    let sumTranslated = 0;
    let sumProofread = 0;

    for (const page of pages) {
        assert.assert(!pageIds.has(page.id), `duplicate page id ${page.id}`);
        pageIds.add(page.id);

        assert.assert(!pageIndexes.has(page.index), `duplicate page index ${page.index}`);
        pageIndexes.add(page.index);

        assert.assertEquals(page.chapter_id, chapterId);

        sumTotal += page.total_unit_count;
        sumTranslated += page.translated_unit_count;
        sumProofread += page.proofread_unit_count;
    }

    assert.assertEquals(
        chapter.total_unit_count,
        sumTotal,
        `chapter total_unit_count ${chapter.total_unit_count} != sum(pages) ${sumTotal}`,
    );
    assert.assertEquals(
        chapter.translated_unit_count,
        sumTranslated,
        `chapter translated_unit_count ${chapter.translated_unit_count} != sum(pages) ${sumTranslated}`,
    );
    assert.assertEquals(
        chapter.proofread_unit_count,
        sumProofread,
        `chapter proofread_unit_count ${chapter.proofread_unit_count} != sum(pages) ${sumProofread}`,
    );

    // page indexes contiguous from 0
    if (pages.length > 0) {
        const sorted = [...pages].sort((a, b) => a.index - b.index);

        sorted.forEach((page, i) => {
            assert.assertEquals(page.index, i, `page index gap at ${i}: got ${page.index}`);
        });
    }

    // stages mask is a non-negative integer within 12-bit range
    assert.assert(Number.isInteger(chapter.stages), "stages must be an integer");
    assert.assert(chapter.stages >= 0, "stages must be non-negative");
    assert.assert(chapter.stages < 1 << 12, "stages must fit 12 bits");

    // assignment (chapter_id, user_id) uniqueness
    const assignments = await listChapterAssignments(api, chapterId);

    const assignmentUserKeys = new Set<string>();

    for (const assignment of assignments) {
        const key = `${assignment.chapter_id}:${assignment.user_id}`;

        assert.assert(!assignmentUserKeys.has(key), `duplicate assignment key ${key}`);
        assignmentUserKeys.add(key);
    }
}

// ---------- J5. Page/unit invariant ----------

export async function assertPageUnitInvariant(api: ApiClient, pageId: string): Promise<void> {
    const units = await listPageUnits(api, pageId);

    const unitIds = new Set<string>();

    let translatedByText = 0;
    let proofreadByFlag = 0;

    for (const unit of units.unit_infos) {
        assert.assert(!unitIds.has(unit.id), `duplicate unit id ${unit.id}`);
        unitIds.add(unit.id);

        assert.assertEquals(unit.page_id, pageId);

        if (
            (unit.translated_text != null && unit.translated_text.trim() !== "") ||
            (unit.proofread_text != null && unit.proofread_text.trim() !== "")
        ) {
            translatedByText += 1;
        }

        if (unit.is_proofread) {
            proofreadByFlag += 1;
        }
    }

    assert.assertEquals(
        units.total_unit_count,
        units.unit_infos.length,
        `total_unit_count ${units.total_unit_count} != unit_infos.length ${units.unit_infos.length}`,
    );

    // Translated means either translation or revision has non-empty text.
    assert.assertEquals(
        units.translated_unit_count,
        translatedByText,
        `translated_unit_count ${units.translated_unit_count} != translated/revision text ${translatedByText}`,
    );

    // proofread count must match is_proofread flag count (NOT proofread_text != null)
    assert.assertEquals(
        units.proofread_unit_count,
        proofreadByFlag,
        `proofread_unit_count ${units.proofread_unit_count} != is_proofread count ${proofreadByFlag}`,
    );
}

// Cross-check list-units against the poprako export for the parent chapter.
// Use this when you know the chapter id and want strict unit_index assertions.
export async function assertPageExportInvariant(
    api: ApiClient,
    chapterId: string,
    pageId: string,
): Promise<void> {
    const units = await listPageUnits(api, pageId);

    const exportVal = await exportPoprako(api, chapterId);

    const exportPage = exportVal.pages.find((page) => page.page_id === pageId);

    assert.assert(exportPage, `export missing page ${pageId}`);

    // unit ids must match exactly between list and export
    const listIds = new Set(units.unit_infos.map((unit: UnitInfoView) => unit.id));
    const exportIds = new Set(exportPage.units.map((unit) => unit.unit_id));

    assert.assertEquals(
        [...listIds].sort(),
        [...exportIds].sort(),
        "export unit ids differ from list unit ids",
    );

    // unit_index must be contiguous 0..n-1
    const sortedExportUnits = [...exportPage.units].sort((a, b) => a.unit_index - b.unit_index);

    sortedExportUnits.forEach((unit, i) => {
        assert.assertEquals(unit.unit_index, i, `export unit_index gap at ${i}: got ${unit.unit_index}`);
    });
}

// Assert every workflow stage in the chapter `stages` mask is consistent with
// the pipeline ordering: a stage may be Completed only if every earlier stage
// is Completed; one-shot stages never report Active.
export function assertStagesPipelineConsistent(chapter: ChapterInfoView): void {
    let prevCompleted = true;

    for (
        const stage of ["raw-provide", "translate", "proofread", "typeset-redraw", "review", "publish"] as StageName[]
    ) {
        const phase = stagePhase(chapter.stages, stage);

        // A stage may only advance if the previous stage is Completed.
        if (!prevCompleted) {
            assert.assertEquals(
                phase,
                0,
                `stage ${stage} must be pending because a prior stage is not completed`,
            );
        }

        // One-shot stages must never report Active (phase 1).
        if (stage === "raw-provide" || stage === "review" || stage === "publish") {
            assert.assertNotEquals(phase, 1, `one-shot stage ${stage} must not be Active`);
        }

        prevCompleted = phase === 2;
    }
}

// ---------- J6. Mail invariant ----------

export async function assertMailInvariant(api: ApiClient): Promise<void> {
    const mails = await listSystemMails(api);

    const mailIds = new Set<string>();

    for (const mail of mails) {
        assert.assert(!mailIds.has(mail.id), `duplicate mail id ${mail.id}`);
        mailIds.add(mail.id);

        assert.assert(typeof mail.created_at === "number" && Number.isInteger(mail.created_at));
        assert.assert(typeof mail.is_read === "boolean");
    }
}

// Variant of J6 that asserts the unread/read filter is consistent with the
// `is_read` flag on each returned mail.
export async function assertMailReadFilterInvariant(api: ApiClient): Promise<void> {
    const unread = await listSystemMails(api, "&is_read=false");
    const read = await listSystemMails(api, "&is_read=true");

    for (const mail of unread) {
        assert.assertEquals(mail.is_read, false, `is_read=false list returned read mail: ${mail.id}`);
    }

    for (const mail of read) {
        assert.assertEquals(mail.is_read, true, `is_read=true list returned unread mail: ${mail.id}`);
    }
}

// Convenience: run J2+J3+J4 for a full workset->comic->chapter subtree.
export async function assertSubtreeInvariants(
    api: ApiClient,
    worksetId: string,
): Promise<void> {
    await assertWorksetInvariant(api, worksetId);

    const comics = await listWorksetComics(api, worksetId);

    for (const comic of comics) {
        await assertComicInvariant(api, comic.id);

        const chapters = await listComicChapters(api, comic.id);

        for (const chapter of chapters) {
            await assertChapterInvariant(api, chapter.id);
        }
    }
}

// Convenience: verify a list of members has unique (user_id, team_id) pairs
// and every member carries the required fields.
export function assertMemberListWellFormed(members: MemberInfoView[]): void {
    const keys = new Set<string>();

    for (const member of members) {
        const key = `${member.team_id}:${member.user_id}`;

        assert.assert(!keys.has(key), `duplicate member key ${key}`);
        keys.add(key);

        assert.assert(member.id, "member.id missing");
        assert.assert(member.user_id, "member.user_id missing");
        assert.assert(member.team_id, "member.team_id missing");
        assert.assert(typeof member.roles === "number");
        assert.assert(typeof member.last_active_at === "number");
    }
}

// Convenience: verify page counters are mutually consistent with the chapter.
export async function assertChapterPageCountMetricsConsistent(
    api: ApiClient,
    chapterId: string,
): Promise<void> {
    const chapter = await getChapter(api, chapterId);
    const pages: PageInfoView[] = await listChapterPages(api, chapterId);

    assert.assertEquals(chapter.page_count, pages.length);

    const sumTotal = pages.reduce((acc, page) => acc + page.total_unit_count, 0);
    const sumTranslated = pages.reduce((acc, page) => acc + page.translated_unit_count, 0);
    const sumProofread = pages.reduce((acc, page) => acc + page.proofread_unit_count, 0);

    assert.assertEquals(chapter.total_unit_count, sumTotal);
    assert.assertEquals(chapter.translated_unit_count, sumTranslated);
    assert.assertEquals(chapter.proofread_unit_count, sumProofread);
}

// Re-export the value-types this module's helpers expose in their signatures,
// so callers can import them alongside the invariant functions.
export type { ComicInfoView, SystemMailInfoView, WorksetInfoView };
