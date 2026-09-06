// it_13 — Single-slot chapter artwork allocation, optimistic completion, and cleanup.
// Runs after bootstrap/member suites; creates its own independent chapter fixtures.

import * as assert from "@std/assert";
import { withDatabaseClient } from "../db/seed.ts";
import { expectError, expectNoContent, expectSuccessData } from "../http/assertions.ts";
import type { ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { advanceStage, archiveComic, createComic, createWorkset } from "../http/fixtures.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

interface ArtworkAllocation {
    artwork_version: number;
    slot: { put_url: string; headers: Record<string, string> } | null;
}

interface ArtworkExport {
    artwork_version: number;
    artwork_hash: string;
    ext: string;
    download_url: string;
}

const artworkHash = (byte: number): string => btoa(String.fromCharCode(...new Uint8Array(32).fill(byte)));
const allocationBody = (byte: number, size = 1024) => ({
    artwork_hash: artworkHash(byte),
    new_byte_len: size,
    ext: "zip",
});

export async function runIt13Module(ctx: RunCtx): Promise<void> {
    const workset = await createWorkset(ctx.sadmin, ctx.ids.defaultTeamId, "artwork port fixtures");
    const comic = await createComic(ctx.sadmin, workset.id, "artwork", "author", "artwork chapter");
    const chapterId = comic.chapter_id;
    const base = `/api/v1/chapters/${chapterId}/artwork`;
    const guest = ctx.users.get("guest_01")!;

    await withDatabaseClient(async (client) => {
        await client.queryObject(
            `UPDATE t_assignment SET f_assigned_publisher_at = NOW() WHERE f_chapter_id = $1 AND f_user_id = $2`,
            [chapterId, ctx.ids.defaultUserId],
        );
    });

    // AW1: argument and assignment permissions, with no allocated object on rejection.
    expectError(await ctx.sadmin.get<ErrorBody>(`${base}/export`), 422, 2);
    expectError(await guest.api.post<ErrorBody>(`${base}/alloc`, allocationBody(1)), 403, 4);
    expectError(await guest.api.post<ErrorBody>(`${base}/mark-uploaded`, { artwork_version: 1 }), 403, 4);

    for (const size of [0, 512 * 1024 * 1024 + 1]) {
        expectError(await ctx.sadmin.post<ErrorBody>(`${base}/alloc`, allocationBody(1, size)), 422, 2);
    }

    expectError(await ctx.sadmin.post<ErrorBody>(`${base}/alloc`, { ...allocationBody(1), ext: "../zip" }), 422, 2);

    // AW2: signed direct upload capability and immediate optimistic availability.
    const allocated = expectSuccessData<ArtworkAllocation>(
        await ctx.sadmin.post<SuccessBody<ArtworkAllocation>>(`${base}/alloc`, allocationBody(1)),
        200,
    );
    assert.assert(allocated.slot);
    const putUrl = new URL(allocated.slot.put_url);
    assert.assertEquals(putUrl.protocol, "https:");
    assert.assertEquals(allocated.slot.headers["content-length"], "1024");
    assert.assertEquals(allocated.slot.headers["content-type"], "application/octet-stream");
    const signedHeaders = putUrl.searchParams.get("X-Amz-SignedHeaders") ?? "";
    assert.assert(signedHeaders.includes("content-length"), "upload signature must bind the exact size");
    expectError(await ctx.sadmin.get<ErrorBody>(`${base}/export`), 422, 2);

    for (let repeat = 0; repeat < 2; repeat++) {
        expectNoContent(await ctx.sadmin.post(`${base}/mark-uploaded`, { artwork_version: allocated.artwork_version }));
    }

    const exported = expectSuccessData<ArtworkExport>(
        await guest.api.get<SuccessBody<ArtworkExport>>(`${base}/export`),
        200,
    );
    assert.assertEquals(exported.artwork_version, allocated.artwork_version);
    assert.assertEquals(exported.artwork_hash, artworkHash(1));
    assert.assert(exported.download_url.includes("chapter_artwork/"));
    assert.assert(!exported.download_url.includes("cdn-cgi/image"));

    await assertCompletedOnce(chapterId);

    const duplicate = expectSuccessData<ArtworkAllocation>(
        await ctx.sadmin.post<SuccessBody<ArtworkAllocation>>(`${base}/alloc`, allocationBody(1)),
        200,
    );
    assert.assertEquals(duplicate.slot, null);
    assert.assertEquals(duplicate.artwork_version, allocated.artwork_version);

    // AW3: old confirmations cannot confirm a replacement generation.
    const replacement = expectSuccessData<ArtworkAllocation>(
        await ctx.sadmin.post<SuccessBody<ArtworkAllocation>>(`${base}/alloc`, allocationBody(2)),
        200,
    );
    assert.assert(replacement.artwork_version > allocated.artwork_version);
    expectError(
        await ctx.sadmin.post<ErrorBody>(`${base}/mark-uploaded`, { artwork_version: allocated.artwork_version }),
        422,
        2,
    );
    expectError(await ctx.sadmin.get<ErrorBody>(`${base}/export`), 422, 2);
    expectNoContent(await ctx.sadmin.post(`${base}/mark-uploaded`, { artwork_version: replacement.artwork_version }));
    await assertCompletedOnce(chapterId);

    // AW4: publication clears the slot and freezes writes, including delayed confirms.
    await advanceStage(ctx.sadmin, chapterId, "publish");
    expectError(await ctx.sadmin.get<ErrorBody>(`${base}/export`), 422, 2);
    expectError(await ctx.sadmin.post<ErrorBody>(`${base}/alloc`, allocationBody(3)), 422, 2);
    expectError(
        await ctx.sadmin.post<ErrorBody>(`${base}/mark-uploaded`, { artwork_version: replacement.artwork_version }),
        422,
        2,
    );
    await assertRetired(chapterId, replacement.artwork_version, false);

    // AW5: archival and ancestor deletion remove chapter object associations.
    const archivedComic = await createComic(ctx.sadmin, workset.id, "archive artwork", "author", "chapter");
    const archivedBase = `/api/v1/chapters/${archivedComic.chapter_id}/artwork`;
    const archivedSlot = expectSuccessData<ArtworkAllocation>(
        await ctx.sadmin.post<SuccessBody<ArtworkAllocation>>(`${archivedBase}/alloc`, allocationBody(4)),
        200,
    );
    // Seed a published chapter with a retained object to exercise archive cleanup independently of publication cleanup.
    await withDatabaseClient(async (client) => {
        await client.queryObject(`UPDATE t_chapter SET f_published_at = NOW() WHERE f_id = $1`, [
            archivedComic.chapter_id,
        ]);
    });
    await archiveComic(ctx.sadmin, archivedComic.id);
    await assertRetired(archivedComic.chapter_id, archivedSlot.artwork_version, true);

    const deletedComic = await createComic(ctx.sadmin, workset.id, "deleted artwork", "author", "chapter");
    const deletedBase = `/api/v1/chapters/${deletedComic.chapter_id}/artwork`;
    const deletedSlot = expectSuccessData<ArtworkAllocation>(
        await ctx.sadmin.post<SuccessBody<ArtworkAllocation>>(`${deletedBase}/alloc`, allocationBody(5)),
        200,
    );
    expectNoContent(await ctx.sadmin.delete(`/api/v1/comics/${deletedComic.id}`));
    expectError(
        await ctx.sadmin.post<ErrorBody>(`${deletedBase}/mark-uploaded`, {
            artwork_version: deletedSlot.artwork_version,
        }),
        422,
        2,
    );
    await assertRetired(deletedComic.chapter_id, deletedSlot.artwork_version, true);
}

async function assertCompletedOnce(chapterId: string): Promise<void> {
    await withDatabaseClient(async (client) => {
        const stages = await client.queryObject<{ completed: boolean }>(
            `SELECT f_typeset_at IS NOT NULL AS completed FROM t_chapter WHERE f_id = $1`,
            [chapterId],
        );
        assert.assertEquals(stages.rows[0]?.completed, true);
        const records = await client.queryObject<{ count: string }>(
            `SELECT count(*)::text AS count FROM t_chapter_workflow_record WHERE f_chapter_id = $1 AND f_payload->>'origin' = 'artwork-upload'`,
            [chapterId],
        );
        assert.assertEquals(records.rows[0]?.count, "1");
    });
}

async function assertRetired(chapterId: string, version: number, removed: boolean): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt++) {
        const retired = await withDatabaseClient(async (client) => {
            const rows = await client.queryObject<{ f_key: string | null }>(
                `SELECT f_key FROM t_chapter_artwork WHERE f_id = $1`,
                [chapterId],
            );
            return removed ? rows.rows.length === 0 : rows.rows.length === 1 && rows.rows[0]?.f_key === null;
        });
        if (retired) {
            await withDatabaseClient(async (client) => {
                const tasks = await client.queryObject<{ count: string }>(
                    `SELECT count(*)::text AS count FROM t_obj_prom_task WHERE f_topic = 'chapter_artwork' AND f_obj_id = $1 AND f_version = $2 AND f_oper = 'obj_prom_oper:delete'`,
                    [chapterId, version],
                );
                assert.assert(Number(tasks.rows[0]?.count) >= 1);
            });
            return;
        }
        await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`chapter artwork ${chapterId} was not retired`);
}
