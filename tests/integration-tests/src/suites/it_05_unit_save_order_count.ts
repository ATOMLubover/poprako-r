// it_05 — Unit save: basic creation, counts, export, import/export.
//
// Covers F1/F2 creation and linked ordering plus F10 import/export.

import * as assert from "@std/assert";

import { grantChapterWorkerRoles, withDatabaseClient } from "../db/seed.ts";
import { expectError, expectStatus } from "../http/assertions.ts";
import type { ErrorBody } from "../http/apiClient.ts";
import { assertPageExportInvariant, assertPageUnitInvariant } from "../http/invariants.ts";
import {
    createChapter,
    exportPoprako,
    exportTranslations,
    getChapter,
    importTranslations,
    listEdittedDiffPageIds,
    newBubbleUnit,
    newPageManifest,
    reserveChapterPages,
    savePageUnits,
    searchChapterUnits,
    transformChapterUnits,
} from "../http/fixtures.ts";
import { titled } from "../state/prefix.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

export async function runIt05Module(ctx: RunCtx): Promise<void> {
    assert.assert(ctx.main, "it_02 must have set ctx.main");
    assert.assert(ctx.main.pageIds.length >= 3, "it_03 must have reserved at least 3 pages on main");

    const mainChapterId = ctx.main.chapterId;
    const p0Id = ctx.main.pageIds[0]!;
    const trans01 = ctx.users.get("trans_01")!;
    const proof02 = ctx.users.get("proof_02")!;

    // ---------- F1. create 5 bubble units on p0 ----------

    const f1Edits = Array.from({ length: 5 }, (_, i) => newBubbleUnit(`p0_lu_0${i + 1}`, 0.1 * (i + 1), 0.1 * (i + 1)));

    const f1Save = await savePageUnits(trans01.api, p0Id, f1Edits);

    assert.assertEquals(f1Save.total_unit_count, 5);
    assert.assertEquals(f1Save.translated_unit_count, 0);
    assert.assertEquals(f1Save.proofread_unit_count, 0);

    const mainAfterF1 = await getChapter(ctx.sadmin, mainChapterId);

    assert.assertEquals(mainAfterF1.total_unit_count, 5);

    const exportAfterF1 = await exportPoprako(ctx.sadmin, mainChapterId);
    const p0ExportAfterF1 = exportAfterF1.pages.find((p) => p.page_id === p0Id);

    assert.assert(p0ExportAfterF1, "export must include p0");
    assert.assertEquals(p0ExportAfterF1!.units.length, 5);

    await assertPageUnitInvariant(ctx.sadmin, p0Id);
    await assertPageExportInvariant(ctx.sadmin, mainChapterId, p0Id);

    // Capture unit IDs in export order for F2.
    const p0UnitIds = f1Save.unit_infos.map((unit) => unit.id);

    // ---------- F2. next_id insert ----------

    const anchoredEdit = {
        edit: "create" as const,
        local_id: "p0_lu_insert_before_02",
        next_id: p0UnitIds[1],
        is_bubble: true,
        coord: {
            x_coord: 0.15,
            y_coord: 0.15,
        },
    };

    const f2Save = await savePageUnits(trans01.api, p0Id, [anchoredEdit]);

    assert.assertEquals(f2Save.total_unit_count, 6, "total 5 -> 6 after next_id insert");

    const exportAfterF2 = await exportPoprako(ctx.sadmin, mainChapterId);
    const p0ExportAfterF2 = exportAfterF2.pages.find((p) => p.page_id === p0Id)!;

    const f2Order = [...p0ExportAfterF2.units]
        .sort((a, b) => a.unit_index - b.unit_index)
        .map((u) => u.unit_id);

    // New unit inserted before the 2nd unit: [u0, inserted, u1, u2, u3, u4]
    assert.assertEquals(f2Order.length, 6);
    assert.assertEquals(f2Order[0], p0UnitIds[0], "u0 stays first");
    assert.assertEquals(f2Order[1], f2Save.unit_infos[1]!.id, "inserted unit before u1");
    assert.assertEquals(f2Order[2], p0UnitIds[1], "u1 shifted to index 2");

    await assertPageExportInvariant(ctx.sadmin, mainChapterId, p0Id);

    // ---------- F3. Chapter Unit search and literal transform ----------

    await savePageUnits(trans01.api, p0Id, [
        {
            edit: "patch",
            id: p0UnitIds[0]!,
            translation: {
                type: "assign",
                value: { translated_text: "alpha beta" },
            },
        },
    ]);

    await savePageUnits(proof02.api, p0Id, [
        {
            edit: "patch",
            id: p0UnitIds[0]!,
            revision: {
                type: "assign",
                value: {
                    is_proofread: false,
                    proofread_text: "alpha reviewed",
                },
            },
        },
    ]);

    const edittedDiffPages = await listEdittedDiffPageIds(
        proof02.api,
        mainChapterId,
    );

    assert.assertEquals(edittedDiffPages.page_ids, [p0Id]);

    const searchMatches = await searchChapterUnits(
        trans01.api,
        mainChapterId,
        "translated_text",
        " alpha ",
    );

    assert.assertEquals(searchMatches.map((unit) => unit.id), [p0UnitIds[0]]);

    await savePageUnits(trans01.api, p0Id, [
        {
            edit: "patch",
            id: p0UnitIds[0]!,
            translation: {
                type: "assign",
                value: { translated_text: "Case %_\\ literal" },
            },
        },
    ]);

    const literalMatches = await searchChapterUnits(
        trans01.api,
        mainChapterId,
        "translated_text",
        "%_\\",
    );

    assert.assertEquals(literalMatches.map((unit) => unit.id), [p0UnitIds[0]]);

    assert.assertEquals(
        await searchChapterUnits(
            trans01.api,
            mainChapterId,
            "translated_text",
            "case",
        ),
        [],
        "search remains case-sensitive",
    );

    assert.assertEquals(
        await searchChapterUnits(
            trans01.api,
            mainChapterId,
            "translated_text",
            "reviewed",
        ),
        [],
        "search only examines the requested text field",
    );

    const proofreadMatches = await searchChapterUnits(
        proof02.api,
        mainChapterId,
        "proofread_text",
        "reviewed",
    );

    assert.assertEquals(proofreadMatches.map((unit) => unit.id), [p0UnitIds[0]]);

    assert.assertEquals(
        await searchChapterUnits(
            trans01.api,
            mainChapterId,
            "translated_text",
            "\0",
        ),
        [],
        "PostgreSQL-incompatible NUL is a literal non-match",
    );

    const insertedUnitId = f2Save.unit_infos[1]!.id;

    await savePageUnits(trans01.api, p0Id, [
        {
            edit: "patch",
            id: p0UnitIds[0]!,
            translation: {
                type: "assign",
                value: { translated_text: "visible-order-marker" },
            },
        },
        {
            edit: "patch",
            id: insertedUnitId,
            translation: {
                type: "assign",
                value: { translated_text: "visible-order-marker" },
            },
        },
        {
            edit: "patch",
            id: p0UnitIds[1]!,
            translation: {
                type: "assign",
                value: { translated_text: "visible-order-marker" },
            },
        },
    ]);

    await savePageUnits(trans01.api, p0Id, [
        { edit: "delete", id: insertedUnitId },
    ]);

    const visibleOrderMatches = await searchChapterUnits(
        trans01.api,
        mainChapterId,
        "translated_text",
        "visible-order-marker",
    );

    assert.assertEquals(
        visibleOrderMatches.map((unit) => unit.id),
        [p0UnitIds[0], p0UnitIds[1]],
        "hidden chain nodes are excluded without changing visible order",
    );

    await savePageUnits(trans01.api, p0Id, [
        {
            edit: "patch",
            id: insertedUnitId,
            translation: {
                type: "assign",
                value: { translated_text: "restored" },
            },
        },
        {
            edit: "patch",
            id: p0UnitIds[0]!,
            translation: {
                type: "assign",
                value: { translated_text: "alpha beta" },
            },
        },
    ]);

    await transformChapterUnits(trans01.api, mainChapterId, "translated_text", [
        {
            unit_id: p0UnitIds[0]!,
            transforms: [
                { origin: "alpha", target: "beta" },
                { origin: "beta", target: "final" },
            ],
        },
        {
            unit_id: "missing-unit-is-skipped",
            transforms: [{ origin: "alpha", target: "unused" }],
        },
    ]);

    const transformed = await searchChapterUnits(
        trans01.api,
        mainChapterId,
        "translated_text",
        "final",
    );

    assert.assertEquals(transformed[0]!.translated_text, "beta final");

    expectError(
        await trans01.api.get<ErrorBody>(
            `/api/v1/chapters/${mainChapterId}/units/search?part=translated_text&phrase=%20%E3%80%80%20`,
        ),
        422,
        2,
    );

    // ---------- F3a. Chapter Unit search match limit ----------

    const searchLimitChapter = await createChapter(
        ctx.sadmin,
        ctx.main.comicId,
        titled("Unit 搜索上限"),
    );

    await grantChapterWorkerRoles(searchLimitChapter.id, ctx.ids.defaultUserId);

    const searchLimitPages = await reserveChapterPages(
        ctx.sadmin,
        searchLimitChapter.id,
        newPageManifest(2, "jpg"),
    );

    const firstPageEdits = Array.from({ length: 100 }, (_, unitIndex) => ({
        ...newBubbleUnit(
            `search-limit-${unitIndex}`,
            unitIndex / 100,
            unitIndex / 100,
        ),
        translation: { translated_text: `日-${unitIndex}` },
    }));

    await savePageUnits(
        ctx.sadmin,
        searchLimitPages.pages[0]!.page_id,
        firstPageEdits,
    );

    const hundredMatches = await searchChapterUnits(
        ctx.sadmin,
        searchLimitChapter.id,
        "translated_text",
        "日",
    );

    assert.assertEquals(hundredMatches.length, 100);

    await savePageUnits(ctx.sadmin, searchLimitPages.pages[1]!.page_id, [
        {
            ...newBubbleUnit("search-limit-101", 0.5, 0.5),
            translation: { translated_text: "第 101 个日文匹配" },
        },
    ]);

    expectError(
        await ctx.sadmin.get<ErrorBody>(
            `/api/v1/chapters/${searchLimitChapter.id}/units/search?part=translated_text&phrase=%E6%97%A5`,
        ),
        422,
        2,
    );

    expectStatus(
        await ctx.sadmin.delete<null>(
            `/api/v1/chapters/${searchLimitChapter.id}`,
        ),
        204,
    );

    // ---------- F10. import/export regression ----------

    const mainExports = await exportTranslations(ctx.sadmin, mainChapterId, [
        "poprako",
        "label_plus",
    ]);

    assert.assert(mainExports.poprako, "combined export must contain PopRaKo");

    assert.assert(mainExports.label_plus, "combined export must contain LabelPlus");

    const mainExport = mainExports.poprako;

    assert.assert(mainExport.chapter_id);
    assert.assert(mainExport.comic_id);
    assert.assert(mainExport.pages.length > 0);

    const mainLabelPlus = mainExports.label_plus;

    assert.assert(mainLabelPlus.length > 0, "label-plus export must be non-empty text");

    const gangtieId = ctx.ids.comicIds["钢铁魔女"]!;
    const f10Chapter = await createChapter(ctx.sadmin, gangtieId, titled("第 7 话 F10导入"));

    await grantChapterWorkerRoles(f10Chapter.id, ctx.ids.defaultUserId);

    await withDatabaseClient(async (client) => {
        await client.queryObject(
            `
              UPDATE "t_assignment"
              SET
                "f_assigned_proofreader_at" = NOW(),
                "f_updated_at" = NOW()
              WHERE "f_chapter_id" = $1
                AND "f_user_id" = $2
            `,
            [f10Chapter.id, ctx.ids.defaultUserId],
        );
    });

    const f10Reserve = await reserveChapterPages(
        ctx.sadmin,
        f10Chapter.id,
        newPageManifest(mainExport.pages.length, "jpg"),
    );

    assert.assertEquals(f10Reserve.pages.length, mainExport.pages.length);

    // JSON body and query enum values use snake_case.
    expectStatus(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${f10Chapter.id}/translations/import`, {
            content: "garbage-content",
            format: "label-plus",
            mode: "overwrite",
        }),
        422,
    );

    // Mode is required even when the source content is otherwise valid.
    expectStatus(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${f10Chapter.id}/translations/import`, {
            content: JSON.stringify(mainExport),
            format: "poprako",
        }),
        422,
    );

    // Valid format with invalid content -> 422/2.
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${f10Chapter.id}/translations/import`, {
            content: "garbage-content",
            format: "label_plus",
            mode: "overwrite",
        }),
        422,
        2,
    );

    const imported = await importTranslations(
        ctx.sadmin,
        f10Chapter.id,
        "poprako",
        "overwrite",
        JSON.stringify(mainExport),
    );

    const populatedPageCount = mainExport.pages.filter((page) => page.units.length > 0).length;

    assert.assertEquals(imported.imported_page_count, populatedPageCount);
    assert.assertEquals(
        imported.imported_unit_count,
        mainExport.pages.reduce((count, page) => count + page.units.length, 0),
    );

    const importedExport = await exportPoprako(ctx.sadmin, f10Chapter.id);

    assert.assertEquals(importedExport.pages.length, mainExport.pages.length);

    mainExport.pages.forEach((sourcePage, pageIndex) => {
        const targetPage = importedExport.pages[pageIndex]!;
        const sourceUnits = [...sourcePage.units].sort((a, b) => a.unit_index - b.unit_index);
        const targetUnits = [...targetPage.units].sort((a, b) => a.unit_index - b.unit_index);

        assert.assertEquals(targetUnits.length, sourceUnits.length);

        sourceUnits.forEach((sourceUnit, unitIndex) => {
            const targetUnit = targetUnits[unitIndex]!;

            assert.assertEquals(targetUnit.unit_index, sourceUnit.unit_index);
            assert.assertEquals(targetUnit.x_coord, sourceUnit.x_coord);
            assert.assertEquals(targetUnit.y_coord, sourceUnit.y_coord);
            assert.assertEquals(targetUnit.is_bubble, sourceUnit.is_bubble);
            assert.assertEquals(targetUnit.translated_text, sourceUnit.translated_text);
            assert.assertEquals(targetUnit.is_proofread, sourceUnit.is_proofread);
            assert.assertEquals(targetUnit.proofread_text, sourceUnit.proofread_text);
            assert.assertNotEquals(targetUnit.unit_id, sourceUnit.unit_id);
        });
    });

    await importTranslations(
        ctx.sadmin,
        f10Chapter.id,
        "poprako",
        "overwrite",
        JSON.stringify(mainExport),
    );

    const repeatedExport = await exportPoprako(ctx.sadmin, f10Chapter.id);
    assert.assertEquals(
        repeatedExport.pages.map((page) => page.units.length),
        importedExport.pages.map((page) => page.units.length),
    );

    const keepSource = JSON.parse(JSON.stringify(mainExport)) as typeof mainExport;
    const keepSourcePage = keepSource.pages.find((page) => page.units.length > 0);

    assert.assert(keepSourcePage, "export fixture must contain a populated page");

    keepSourcePage.units[0]!.translated_text = "keep must not replace this text";

    const beforeKeepExport = await exportPoprako(ctx.sadmin, f10Chapter.id);
    const kept = await importTranslations(
        ctx.sadmin,
        f10Chapter.id,
        "poprako",
        "keep",
        JSON.stringify(keepSource),
    );
    const afterKeepExport = await exportPoprako(ctx.sadmin, f10Chapter.id);

    assert.assertEquals(kept.imported_page_count, 0);
    assert.assertEquals(kept.imported_unit_count, 0);
    assert.assertEquals(afterKeepExport, beforeKeepExport);

    // cleanup F10 aux chapter
    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/chapters/${f10Chapter.id}`), 204);
}
