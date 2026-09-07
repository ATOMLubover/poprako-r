// it_03 — Page reserve, image mark-uploaded, page index/counters, delete+rebuild.
//
// Preconditions:
//   - it_00 + it_01 + it_02 have run.
//   - `ctx.main` is `星尘旅人 / 第 2 话 月面信号` with no pages yet.
//   - `ctx.auxChapters.get("cascade")` exists (2 comics on 归档池).
//
// Postconditions:
//   - `ctx.main.pageIds` filled with 8 page ids (index 0..7), each image
//     marked uploaded.
//   - `ctx.auxChapters.get("cascade")` has 2 pages reserved on its chapter
//     (and on the other cascade chapter) for it_10.
//   - `ctx.auxChapters.get("d3")` set to a `钢铁魔女 / 第 2 话 D3辅助`
//     chapter with 3 pages + 6 units, then deleted-and-rebuilt to 2 pages.
//
// Covers test-plan: D1, D2, D3.
//
// Grounded perm pins:
//   - page reserve (batch + single): assignment with RAW_PROVIDER or REVIEWER.
//   - page image mark-uploaded: assignment with RAW_PROVIDER.
//   - page delete-all: team ADMIN.
//   - sadmin's auto-assignment from chapter create is ADMIN only, so sadmin
//     CANNOT reserve pages until granted RAW_PROVIDER. We reuse
//     `grantChapterWorkerRoles` (sets raw_provider + translator timestamps)
//     to add RAW_PROVIDER to sadmin's admin assignment on the main chapter.
//   - guest_01 has no assignment on main -> reserve/mark -> 403/4.
//
// Status: IMPLEMENTED.

import * as assert from "@std/assert";

import { grantChapterWorkerRoles } from "../db/seed.ts";
import { expectError, expectStatus } from "../http/assertions.ts";
import type { ErrorBody } from "../http/apiClient.ts";
import { assertChapterInvariant, assertChapterPageCountMetricsConsistent } from "../http/invariants.ts";
import {
    createChapter,
    deleteChapterPages,
    getChapter,
    getComic,
    listChapterPages,
    listComicChapters,
    listWorksetComics,
    newBubbleUnit,
    newPageManifest,
    reserveChapterPages,
    reservePageImage,
    savePageUnits,
} from "../http/fixtures.ts";
import { titled } from "../state/prefix.ts";
import type { ChapterRefs, RunCtx } from "../state/runCtx.ts";
import { cascadeExtraIds } from "./it_02_workset_comic_chapter_index.ts";

export const IMPLEMENTED = true as const;

export async function runIt03Module(ctx: RunCtx): Promise<void> {
    assert.assert(ctx.main, "it_02 must have set ctx.main");
    assert.assert(ctx.ids.defaultUserId);

    const mainChapterId = ctx.main.chapterId;
    const guest01 = ctx.users.get("guest_01");

    assert.assert(guest01, "it_01 must have registered guest_01");

    // Grant sadmin RAW_PROVIDER (+ TRANSLATOR) on main so page reserve/mark
    // and unit save (translator) are permitted.
    await grantChapterWorkerRoles(mainChapterId, ctx.ids.defaultUserId);

    // ---------- D1. batch reserve 8 pages on main ----------

    const sourceManifest = newPageManifest(8, "jpg");

    sourceManifest[0]!.raw_ident = "原稿 01.PNG";

    const reserveVal = await reserveChapterPages(ctx.sadmin, mainChapterId, sourceManifest);

    assert.assertEquals(reserveVal.pages.length, 8);

    const pageIds: string[] = [];
    const pageVersions = new Map<string, number>();
    const seenPageIds = new Set<string>();

    for (const creation of reserveVal.pages) {
        assert.assert(creation.page_id);
        assert.assert(creation.slot?.put_url.startsWith("http"), "put_url must be an http url");
        assert.assert(Number.isInteger(creation.slot?.image_version) && creation.slot!.image_version > 0);

        assert.assert(!seenPageIds.has(creation.page_id), "page ids must be unique");
        seenPageIds.add(creation.page_id);

        pageIds.push(creation.page_id);
        pageVersions.set(creation.page_id, creation.slot!.image_version);
    }

    ctx.main.pageIds = pageIds;

    const retainedPendingManifest = await reserveChapterPages(
        ctx.sadmin,
        mainChapterId,
        reserveVal.pages.map((page) => ({
            page_id: page.page_id,
            image_hash: page.image_hash,
            ext: page.ext,
            raw_ident: page.index === 0 ? "原稿 01.PNG" : null,
        })),
    );

    assert.assertEquals(
        retainedPendingManifest.pages.map((page) => page.page_id),
        pageIds,
        "pending images with identical hash and extension must retain page identities",
    );
    assert.assert(
        retainedPendingManifest.pages.every((page) => page.slot === null),
        "retained pending identities without new_byte_len must not allocate slots",
    );

    // list pages: 8 pages, index 0..7, all unit counts 0
    const pages = await listChapterPages(ctx.sadmin, mainChapterId);

    assert.assertEquals(pages.length, 8);

    const sortedPages = [...pages].sort((a, b) => a.index - b.index);

    sortedPages.forEach((page, i) => {
        assert.assertEquals(page.index, i, `page index must be ${i}`);
        assert.assertEquals(page.total_unit_count, 0);
        assert.assertEquals(page.translated_unit_count, 0);
        assert.assertEquals(page.proofread_unit_count, 0);
    });

    // chapter counters
    const mainChapter = await getChapter(ctx.sadmin, mainChapterId);

    assert.assertEquals(mainChapter.page_count, 8);
    assert.assertEquals(mainChapter.total_unit_count, 0);
    assert.assertEquals(mainChapter.translated_unit_count, 0);
    assert.assertEquals(mainChapter.proofread_unit_count, 0);

    // duplicate explicit page ids are rejected before the manifest transaction
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/alloc`, {
            chapter_id: mainChapterId,
            pages: [
                {
                    page_id: pageIds[0],
                    image_hash: sortedPages[0]!.image_hash,
                    new_byte_len: 1,
                    ext: sortedPages[0]!.ext,
                },
                {
                    page_id: pageIds[0],
                    image_hash: sortedPages[0]!.image_hash,
                    new_byte_len: 1,
                    ext: sortedPages[0]!.ext,
                },
            ],
        }),
        422,
        2,
    );

    // page_count 0 -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/alloc`, {
            chapter_id: mainChapterId,
            pages: [],
        }),
        422,
        2,
    );

    // path chapter_id / body chapter_id mismatch -> 422 (path/body id rule)
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/alloc`, {
            chapter_id: "not-the-path-id",
            pages: newPageManifest(1, "jpg"),
        }),
        422,
        7,
    );

    await assertChapterInvariant(ctx.sadmin, mainChapterId);

    // ---------- D2. mark-uploaded + single page replace ----------

    // mark all 8 pages uploaded
    for (const pageId of pageIds) {
        const version = pageVersions.get(pageId)!;

        expectStatus(
            await ctx.sadmin.post<null>(`/api/v1/pages/${pageId}/image/mark-uploaded`, {
                image_version: version,
            }),
            204,
        );
    }

    // Mark optimistically exposes the exact current generation. The delayed
    // actor may revoke it if remote presence verification fails.
    const markedPages = await listChapterPages(ctx.sadmin, mainChapterId);

    for (const page of markedPages) {
        assert.assert(
            page.image_url,
            `page ${page.id} image_url must be available after mark`,
        );
        assert.assert(
            page.image_optimized_url,
            `page ${page.id} image_optimized_url must be available after mark`,
        );
        assert.assert(
            page.image_thumbnail_url,
            `page ${page.id} image_thumbnail_url must be available after mark`,
        );
    }

    const firstMarkedPage = markedPages.find((page) => page.index === 0)!;

    const fallbackComic = await getComic(ctx.sadmin, ctx.main.comicId);

    assert.assertEquals(
        fallbackComic.cover_url,
        firstMarkedPage.image_url,
        "comic cover must fall back to the pinned chapter's first uploaded page",
    );
    assert.assertEquals(
        fallbackComic.cover_thumbnail_url,
        firstMarkedPage.image_thumbnail_url,
        "comic cover thumbnail must use the same fallback page generation",
    );

    const worksetComics = await listWorksetComics(
        ctx.sadmin,
        ctx.main.worksetId,
    );
    const fallbackComicInList = worksetComics.find(
        (comic) => comic.id === ctx.main!.comicId,
    )!;

    assert.assertEquals(fallbackComicInList.cover_url, firstMarkedPage.image_url);
    assert.assertEquals(
        fallbackComicInList.cover_thumbnail_url,
        firstMarkedPage.image_thumbnail_url,
    );

    const chaptersWithComic = await listComicChapters(
        ctx.sadmin,
        ctx.main.comicId,
        "&incl=comic",
    );
    const pinnedChapterWithComic = chaptersWithComic.find(
        (chapter) => chapter.id === mainChapterId,
    )!;

    assert.assertEquals(
        pinnedChapterWithComic.comic?.cover_url,
        firstMarkedPage.image_url,
    );
    assert.assertEquals(
        pinnedChapterWithComic.comic?.cover_thumbnail_url,
        firstMarkedPage.image_thumbnail_url,
    );

    const retainedManifest = await reserveChapterPages(
        ctx.sadmin,
        mainChapterId,
        [...markedPages]
            .sort((a, b) => a.index - b.index)
            .map((page) => ({
                page_id: page.id,
                image_hash: page.image_hash!,
                ext: page.ext!,
                raw_ident: sourceManifest[page.index]?.raw_ident ?? null,
            })),
    );

    assert.assert(
        retainedManifest.pages.every((page) => page.slot === null),
        "unchanged uploaded manifest entries without new_byte_len must not receive slots",
    );

    const orderedMarkedPages = [...markedPages].sort(
        (left, right) => left.index - right.index,
    );
    const automaticallyMatchedManifest = await reserveChapterPages(
        ctx.sadmin,
        mainChapterId,
        orderedMarkedPages.map((page, index) => ({
            page_id: index === 0 ? null : page.id,
            image_hash: page.image_hash!,
            new_byte_len: index === 0 ? 1 : undefined,
            ext: page.ext!,
            raw_ident: sourceManifest[index]?.raw_ident ?? null,
        })),
    );

    assert.assertEquals(
        automaticallyMatchedManifest.pages[0]?.page_id,
        orderedMarkedPages[0]?.id,
        "hash-plus-extension auto matching must retain the existing page identity",
    );
    assert.assert(
        automaticallyMatchedManifest.pages.every((page) => page.slot === null),
        "auto matching identical available content must not allocate a new generation",
    );

    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/alloc`, {
            chapter_id: mainChapterId,
            pages: [
                {
                    page_id: pageIds[0],
                    image_hash: markedPages.find((page) => page.id === pageIds[0])!.image_hash,
                    ext: "png",
                },
            ],
        }),
        422,
        2,
    );

    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/alloc`, {
            chapter_id: mainChapterId,
            pages: [
                ...markedPages.map((page) => ({
                    page_id: page.id,
                    image_hash: page.image_hash,
                    ext: page.ext,
                })),
                {
                    page_id: null,
                    image_hash: markedPages[0]!.image_hash,
                    ext: "png",
                },
            ],
        }),
        422,
        2,
    );

    // single page replace on page index 2 (p2)
    const p2Id = pageIds[2]!;
    const p2OldVersion = pageVersions.get(p2Id)!;

    const p2Reserve = await reservePageImage(ctx.sadmin, p2Id, "png", "replacement.JPG");

    assert.assertEquals(p2Reserve.page_id, p2Id);
    assert.assert(p2Reserve.slot?.put_url.startsWith("http"));
    assert.assert(p2Reserve.slot && p2Reserve.slot.image_version > p2OldVersion, "new image_version must exceed old");

    const p2NewVersion = p2Reserve.slot!.image_version;

    // mark new version
    expectStatus(
        await ctx.sadmin.post<null>(`/api/v1/pages/${p2Id}/image/mark-uploaded`, {
            image_version: p2NewVersion,
        }),
        204,
    );

    // Replacement mark immediately exposes the new generation's URLs.
    const p2After = (await listChapterPages(ctx.sadmin, mainChapterId)).find((p) => p.id === p2Id);

    assert.assert(
        p2After?.image_url,
        "p2 image_url must be available after replacement mark",
    );
    assert.assert(
        p2After?.image_thumbnail_url,
        "p2 image_thumbnail_url must be available after replacement mark",
    );

    // stale version mark -> 422/2 (version mismatch)
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/pages/${p2Id}/image/mark-uploaded`, {
            image_version: p2OldVersion,
        }),
        422,
        2,
    );

    // D2.6: non-assigned member (guest_01 has no assignment on main) mark/reserve -> 403/4
    expectError(
        await guest01.api.post<ErrorBody>(`/api/v1/pages/${p2Id}/image/mark-uploaded`, {
            image_version: p2NewVersion,
        }),
        403,
        4,
    );

    expectError(
        await guest01.api.post<ErrorBody>(`/api/v1/pages/${p2Id}/image/alloc`, {
            image_hash: sortedPages[2]!.image_hash,
            new_byte_len: 1,
            ext: sortedPages[2]!.ext,
        }),
        403,
        4,
    );

    // D2.7: non-existent page_id reserve/mark -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/pages/page-does-not-exist/image/alloc", {
            image_hash: sortedPages[0]!.image_hash,
            new_byte_len: 1,
            ext: sortedPages[0]!.ext,
        }),
        422,
        2,
    );

    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/pages/page-does-not-exist/image/mark-uploaded", {
            image_version: 1,
        }),
        422,
        2,
    );

    // ---------- D3. delete all pages + rebuild (on aux chapter, NOT main) ----------

    // Build the d3 aux chapter on 钢铁魔女.
    const gangtieId = ctx.ids.comicIds["钢铁魔女"]!;

    const d3Chapter = await createChapter(ctx.sadmin, gangtieId, titled("第 2 话 D3辅助"));

    const d3ChapterId = d3Chapter.id;

    // grant sadmin worker roles on d3 chapter (gets RAW_PROVIDER + TRANSLATOR)
    await grantChapterWorkerRoles(d3ChapterId, ctx.ids.defaultUserId);

    const d3Refs: ChapterRefs = {
        chapterId: d3ChapterId,
        comicId: gangtieId,
        worksetId: ctx.ids.worksetIds["连载池"]!,
        pageIds: [],
        assignmentIds: {},
    };

    ctx.auxChapters.set("d3", d3Refs);

    // reserve 3 pages on d3
    const d3Reserve = await reserveChapterPages(ctx.sadmin, d3ChapterId, newPageManifest(3, "jpg"));

    assert.assertEquals(d3Reserve.pages.length, 3);

    const d3PageIds = d3Reserve.pages.map((c) => c.page_id);

    d3Refs.pageIds = d3PageIds;

    // write 2 units on each of the 3 pages (sadmin has TRANSLATOR via grant)
    for (const pageId of d3PageIds) {
        const save = await savePageUnits(ctx.sadmin, pageId, [
            newBubbleUnit("d3_u1", 0.1, 0.1),
            newBubbleUnit("d3_u2", 0.2, 0.2),
        ]);

        assert.assertEquals(save.total_unit_count, 2);
    }

    const d3ChapterBefore = await getChapter(ctx.sadmin, d3ChapterId);

    assert.assertEquals(d3ChapterBefore.page_count, 3);
    assert.assertEquals(d3ChapterBefore.total_unit_count, 6);
    assert.assertEquals(d3ChapterBefore.translated_unit_count, 0);
    assert.assertEquals(d3ChapterBefore.proofread_unit_count, 0);

    // delete all pages
    await deleteChapterPages(ctx.sadmin, d3ChapterId);

    // list pages empty
    const d3PagesAfterDelete = await listChapterPages(ctx.sadmin, d3ChapterId);

    assert.assertEquals(d3PagesAfterDelete.length, 0);

    // chapter counters all zero
    const d3ChapterAfter = await getChapter(ctx.sadmin, d3ChapterId);

    assert.assertEquals(d3ChapterAfter.page_count, 0);
    assert.assertEquals(d3ChapterAfter.total_unit_count, 0);
    assert.assertEquals(d3ChapterAfter.translated_unit_count, 0);
    assert.assertEquals(d3ChapterAfter.proofread_unit_count, 0);

    // rebuild: reserve 2 pages
    const d3RebuildReserve = await reserveChapterPages(ctx.sadmin, d3ChapterId, newPageManifest(2, "jpg"));

    assert.assertEquals(d3RebuildReserve.pages.length, 2);

    const d3RebuildPages = await listChapterPages(ctx.sadmin, d3ChapterId);

    assert.assertEquals(d3RebuildPages.length, 2);

    const d3RebuildSorted = [...d3RebuildPages].sort((a, b) => a.index - b.index);

    d3RebuildSorted.forEach((page, i) => {
        assert.assertEquals(page.index, i, `rebuild page index must be ${i}`);
    });

    // old page id units -> 422/2
    const oldD3PageId = d3PageIds[0]!;

    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/pages/${oldD3PageId}/units?offset=0&limit=20`),
        422,
        2,
    );

    // old page id image reserve -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/pages/${oldD3PageId}/image/alloc`, {
            image_hash: sortedPages[0]!.image_hash,
            new_byte_len: 1,
            ext: sortedPages[0]!.ext,
        }),
        422,
        2,
    );

    // update d3Refs pageIds to the rebuilt pages
    d3Refs.pageIds = d3RebuildReserve.pages.map((c) => c.page_id);

    await assertChapterInvariant(ctx.sadmin, d3ChapterId);
    await assertChapterPageCountMetricsConsistent(ctx.sadmin, d3ChapterId);

    // ---------- aux: reserve pages on cascade chapters for it_10 ----------

    const cascadeRefs = ctx.auxChapters.get("cascade");

    assert.assert(cascadeRefs, "it_02 must have set cascade aux chapter");

    // Grant sadmin RAW_PROVIDER on each cascade chapter first (reserve needs
    // RAW_PROVIDER; sadmin's auto-assignment from create is ADMIN only).
    await grantChapterWorkerRoles(cascadeRefs.chapterId, ctx.ids.defaultUserId);
    await grantChapterWorkerRoles(cascadeExtraIds.cascadeComicACh1, ctx.ids.defaultUserId);
    await grantChapterWorkerRoles(cascadeExtraIds.cascadeComicBCh1, ctx.ids.defaultUserId);

    // reserve 2 pages on cascadeComicACh2 (the cascade ChapterRefs chapterId)
    const cascadeACh2Reserve = await reserveChapterPages(
        ctx.sadmin,
        cascadeRefs.chapterId,
        newPageManifest(2, "jpg"),
    );

    assert.assertEquals(cascadeACh2Reserve.pages.length, 2);

    cascadeRefs.pageIds = cascadeACh2Reserve.pages.map((c) => c.page_id);

    // reserve 2 pages on cascadeComicACh1 and cascadeComicBCh1 too
    const reserveACh1 = await reserveChapterPages(
        ctx.sadmin,
        cascadeExtraIds.cascadeComicACh1,
        newPageManifest(2, "jpg"),
    );

    const reserveBCh1 = await reserveChapterPages(
        ctx.sadmin,
        cascadeExtraIds.cascadeComicBCh1,
        newPageManifest(2, "jpg"),
    );

    assert.assertEquals(reserveACh1.pages.length, 2);
    assert.assertEquals(reserveBCh1.pages.length, 2);

    // sanity: main chapter invariant still holds
    await assertChapterInvariant(ctx.sadmin, mainChapterId);
}
