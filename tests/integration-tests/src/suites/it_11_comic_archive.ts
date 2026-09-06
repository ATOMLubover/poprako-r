// it_11 — Immutable comic archive endpoint.
//
// Preconditions:
//   - it_00..it_10 have run and the default super-admin remains active.
//   - ctx.users has 14 non-admin member personas from it_01.
//
// Postconditions:
//   - An independent archive workset remains active for final cleanup.
//   - The archived comic remains as a read-only management header while its
//     active subtree is replaced by one immutable JSON-text archive row.
//
// Covers archive creation, perm rejection (non-admin member),
// repeated-archive failure, month export, child-resource
// inaccessibility, audit fields, object delete tasks, active-data cleanup,
// and stable workset comic counts.

import * as assert from "@std/assert";

import { grantChapterWorkerRoles, withDatabaseClient } from "../db/seed.ts";
import { expectError, expectSuccessData } from "../http/assertions.ts";
import type { ErrorBody, SuccessBody } from "../http/apiClient.ts";
import {
    archiveComic,
    createChapter,
    createComic,
    createWorkset,
    getWorkset,
    listWorksetComics,
    newPageManifest,
    reserveChapterPages,
    reserveComicCover,
} from "../http/fixtures.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

interface ActiveObjectVersion {
    f_obj_id: string;
    f_version: string;
}

interface ActiveObjectVersions {
    cover: ActiveObjectVersion;
    pages: ActiveObjectVersion[];
}

interface ArchiveAuditRow {
    f_archiver_id: string;
    f_source_comic_id: string;
    f_created_at: Date;
}

export async function runIt11Module(ctx: RunCtx): Promise<void> {
    const workset = await createWorkset(
        ctx.sadmin,
        ctx.ids.defaultTeamId,
        "archive endpoint fixture",
    );
    const comic = await createComic(
        ctx.sadmin,
        workset.id,
        "archive endpoint comic",
        "archive endpoint author",
        "archive endpoint chapter",
    );

    // Create a second chapter so the archive snapshot is multi-chapter.
    const chapter2 = await createChapter(ctx.sadmin, comic.id, "second chapter");

    await grantChapterWorkerRoles(comic.chapter_id, ctx.ids.defaultUserId);
    await grantChapterWorkerRoles(chapter2.id, ctx.ids.defaultUserId);

    // Reserve one page in each chapter.
    await reserveChapterPages(ctx.sadmin, comic.chapter_id, newPageManifest(1, "png"));
    await reserveChapterPages(ctx.sadmin, chapter2.id, newPageManifest(1, "png"));

    await reserveComicCover(ctx.sadmin, comic.id, "png");

    // ---------- perm: non-admin member cannot archive ----------

    const guest = ctx.users.get("guest_01");

    assert.assert(guest, "guest_01 must be registered by it_01");

    expectError(
        await guest.api.post<ErrorBody>(`/api/v1/comics/${comic.id}/archive`),
        403,
        4,
    );

    // ---------- snapshot pre-archive state ----------

    const workset_before_archive = await getWorkset(ctx.sadmin, workset.id);
    const active_object_versions: ActiveObjectVersions = await withDatabaseClient(async (client) => {
        const [cover_result, page_result] = await Promise.all([
            client.queryObject<ActiveObjectVersion>(
                `SELECT "f_id" AS f_obj_id, "f_version" FROM "t_comic_cover" WHERE "f_id" = $1`,
                [comic.id],
            ),
            client.queryObject<ActiveObjectVersion>(
                `
                      SELECT page_image."f_id" AS f_obj_id, page_image."f_version"
                      FROM "t_page_image" page_image
                      JOIN "t_page" page ON page."f_id" = page_image."f_id"
                      WHERE page."f_chapter_id" = ANY($1)
                      ORDER BY page."f_chapter_id", page."f_index"
                    `,
                [[comic.chapter_id, chapter2.id]],
            ),
        ]);

        return {
            cover: cover_result.rows[0]!,
            pages: page_result.rows,
        };
    });

    assert.assertEquals(active_object_versions.cover.f_obj_id, comic.id);
    assert.assertEquals(active_object_versions.pages.length, 2);

    // Archive requires every chapter to have completed publish. Set the
    // completed state directly so this scenario can retain object versions and
    // verify archive-side cleanup of the remaining sources.
    await withDatabaseClient(async (client) => {
        await client.queryObject(
            `UPDATE "t_chapter" SET "f_published_at" = NOW() WHERE "f_id" = ANY($1)`,
            [[comic.chapter_id, chapter2.id]],
        );
    });

    // ---------- archive the comic ----------

    const archive_comic_val = await archiveComic(ctx.sadmin, comic.id);

    assert.assertNotEquals(archive_comic_val.archived_id, comic.id);

    // ---------- export selected archive month ----------

    const archive_month = new Date().toISOString().slice(0, 7);
    const export_response = await ctx.sadmin.get<
        SuccessBody<Record<string, string[]>>
    >(
        `/api/v1/teams/${ctx.ids.defaultTeamId}/comic-archives/export?month=${archive_month}`,
    );
    const exported_months = expectSuccessData<Record<string, string[]>>(
        export_response,
        200,
    );
    const exported_comics = exported_months[archive_month]!;

    assert.assertEquals(exported_comics.length, 1);
    assert.assertEquals(JSON.parse(exported_comics[0]!).source_comic_id, comic.id);

    // ---------- archived comic header remains available; children are removed ----------

    const archived_comic = await ctx.sadmin.get<SuccessBody<{ is_archived: boolean }>>(
        `/api/v1/comics/${comic.id}`,
    );

    assert.assertEquals(expectSuccessData(archived_comic, 200).is_archived, true);

    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comic.chapter_id}`), 422, 2);
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${chapter2.id}`), 422, 2);

    // ---------- child resources are inaccessible ----------

    assert.assertEquals(
        expectSuccessData(
            await ctx.sadmin.get<SuccessBody<unknown[]>>(
                `/api/v1/comics/${comic.id}/chapters?offset=0&limit=20`,
            ),
            200,
        ),
        [],
    );
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comic.chapter_id}/pages?offset=0&limit=20`),
        422,
        2,
    );
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${chapter2.id}/pages?offset=0&limit=20`),
        422,
        2,
    );

    // ---------- repeated archive on the same comic fails ----------

    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/comics/${comic.id}/archive`),
        422,
        2,
    );

    // ---------- workset comic count unchanged (archive does not decrement) ----------

    const archived_workset = await getWorkset(ctx.sadmin, workset.id);
    const default_comics = await listWorksetComics(ctx.sadmin, workset.id);
    const active_comics = await listWorksetComics(ctx.sadmin, workset.id, "&status=active");

    assert.assertEquals(archived_workset.comic_count, workset_before_archive.comic_count);
    assert.assert(default_comics.some((comic_info) => comic_info.id === comic.id));
    assert.assert(!active_comics.some((active_comic) => active_comic.id === comic.id));

    // ---------- archive audit rows and object delete tasks ----------

    const archive_rows = await withDatabaseClient(async (client) => {
        const [comic_rows, delete_rows] = await Promise.all([
            client.queryObject<ArchiveAuditRow>(
                `SELECT "f_archiver_id", "f_source_comic_id", "f_created_at" FROM "t_comic_archive" WHERE "f_id" = $1`,
                [archive_comic_val.archived_id],
            ),
            client.queryObject<ActiveObjectVersion & { f_topic: string }>(
                `
                  SELECT "f_topic", "f_obj_id", "f_version"
                  FROM "t_obj_prom_task"
                  WHERE "f_oper" = 'obj_prom_oper:delete'
                `,
            ),
        ]);

        return { comic_rows, delete_rows };
    });

    assert.assertEquals(archive_rows.comic_rows.rows.length, 1);
    assert.assertEquals(archive_rows.comic_rows.rows[0]!.f_archiver_id, ctx.ids.defaultUserId);
    assert.assertEquals(archive_rows.comic_rows.rows[0]!.f_source_comic_id, comic.id);
    assert.assert(Number.isFinite(archive_rows.comic_rows.rows[0]!.f_created_at.getTime()));

    const hasDeleteTask = (topic: string, object: ActiveObjectVersion) =>
        archive_rows.delete_rows.rows.some(
            (task) =>
                task.f_topic === topic &&
                task.f_obj_id === object.f_obj_id &&
                task.f_version === object.f_version,
        );

    assert.assert(hasDeleteTask("comic_cover", active_object_versions.cover));

    assert.assert(
        active_object_versions.pages.every((page_image) => hasDeleteTask("page_image", page_image)),
        "all reserved page images must have delete tasks",
    );
}
