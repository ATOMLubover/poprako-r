// it_06 — Unit concurrency: parallel writes, conflicts, inserts, delete+update.
//
// Preconditions:
//   - it_00..it_05 have run. `ctx.main` has 8 pages; p0 has the F1-F4 state;
//     p1 is untouched (0 units). trans_01/02/03, proof_01/02, raw_01/02
//     assigned on main.
//
// Postconditions:
//   - p1 ends with 12 translated units (F6) plus F8 inserts minus F9 delete.
//     Final p1 state is consumed by it_07.
//
// Covers test-plan: F6, F7, F8, F9.
//
// Grounded pins:
//   - unit save is Serializable. A serialization failure returns 409/code 8;
//     the client retries the complete request. Same-unit concurrent saves are
//     last-write-wins after successful commits (no version field in the DTO).
//   - next_id insert is an in-memory order operation; concurrent inserts
//     before the same anchor both eventually succeed after client retries;
//     final order has both new units before the anchor.
//
// Status: IMPLEMENTED.

import * as assert from "@std/assert";

import {
    assertChapterPageCountMetricsConsistent,
    assertPageExportInvariant,
    assertPageUnitInvariant,
} from "../http/invariants.ts";
import {
    deleteUnit,
    exportPoprako,
    getChapter,
    listPageUnits,
    newBubbleUnit,
    savePageUnits,
    updateUnit,
} from "../http/fixtures.ts";
import { PHASE, stagePhase } from "../state/stages.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

export async function runIt06Module(ctx: RunCtx): Promise<void> {
    assert.assert(ctx.main, "it_02 must have set ctx.main");
    assert.assert(ctx.main.pageIds.length >= 2, "it_03 must have reserved at least 2 pages on main");

    const mainChapterId = ctx.main.chapterId;
    const p1Id = ctx.main.pageIds[1]!;

    const trans01 = ctx.users.get("trans_01")!;
    const trans02 = ctx.users.get("trans_02")!;
    const trans03 = ctx.users.get("trans_03")!;
    const raw01 = ctx.users.get("raw_01")!;
    const raw02 = ctx.users.get("raw_02")!;

    // trans_03 was deleted from main assignment in it_04 E3. Re-join so F6 has
    // a third translator. trans_03 member roles = TRANSLATOR, so join is allowed.
    const trans03Assignment = await (
        await import("../http/fixtures.ts")
    ).joinChapterAssignment(trans03.api, mainChapterId, 2);

    ctx.main.assignmentIds["trans_03"] = trans03Assignment.id;

    // ---------- setup p1: clear to 0 units, then create 12 bubbles ----------

    const p1Before = await listPageUnits(ctx.sadmin, p1Id);

    if (p1Before.total_unit_count > 0) {
        await savePageUnits(
            ctx.sadmin,
            p1Id,
            p1Before.unit_infos.map((u) => deleteUnit(u.id)),
        );
    }

    const p1Cleared = await listPageUnits(ctx.sadmin, p1Id);

    assert.assertEquals(p1Cleared.total_unit_count, 0, "p1 must start at 0 units");

    // raw_01/02 cannot save units (RAW_PROVIDER only). Use trans_01 to create
    // the 12 bubbles (trans_01 is a translator).
    const create12 = await savePageUnits(
        trans01.api,
        p1Id,
        Array.from({ length: 12 }, (_, i) => newBubbleUnit(`p1_u${i}`, 0.05 * i, 0.05 * i)),
    );

    assert.assertEquals(create12.total_unit_count, 12);

    const p1List = await listPageUnits(ctx.sadmin, p1Id);
    const p1UnitIds = p1List.unit_infos.map((unit) => unit.id);

    assert.assertEquals(p1UnitIds.length, 12);

    // ---------- F6. 3 translators update 4 units each in parallel ----------

    const f6Trans01 = p1UnitIds.slice(0, 4);
    const f6Trans02 = p1UnitIds.slice(4, 8);
    const f6Trans03 = p1UnitIds.slice(8, 12);

    const buildF6Edits = (targets: string[], prefix: string) =>
        targets.map((unitId, i) =>
            updateUnit(unitId, {
                is_bubble: true,
                is_proofread: false,
                translated_text: `${prefix}${i}`,
            })
        );

    const [f6A, f6B, f6C] = await Promise.all([
        savePageUnits(trans01.api, p1Id, buildF6Edits(f6Trans01, "A")),
        savePageUnits(trans02.api, p1Id, buildF6Edits(f6Trans02, "B")),
        savePageUnits(trans03.api, p1Id, buildF6Edits(f6Trans03, "C")),
    ]);

    // All three saves eventually returned a fresh list; the fixture handles
    // retryable 409/code 8 responses at the client boundary.
    assert.assertEquals(f6A.translated_unit_count <= 12, true, "F6 A count <= 12");
    assert.assertEquals(f6B.translated_unit_count <= 12, true, "F6 B count <= 12");
    assert.assertEquals(f6C.translated_unit_count <= 12, true, "F6 C count <= 12");

    void f6A;
    void f6B;
    void f6C;

    const p1AfterF6 = await listPageUnits(ctx.sadmin, p1Id);

    assert.assertEquals(p1AfterF6.total_unit_count, 12, "no units lost");
    assert.assertEquals(p1AfterF6.translated_unit_count, 12, "all 12 translated");

    const unitByIdF6 = new Map(p1AfterF6.unit_infos.map((u) => [u.id, u]));

    for (let i = 0; i < 4; i++) {
        assert.assertEquals(unitByIdF6.get(f6Trans01[i]!)?.translated_text, `A${i}`);
        assert.assertEquals(unitByIdF6.get(f6Trans01[i]!)?.last_translator_id, trans01.userId);
    }

    for (let i = 0; i < 4; i++) {
        assert.assertEquals(unitByIdF6.get(f6Trans02[i]!)?.translated_text, `B${i}`);
        assert.assertEquals(unitByIdF6.get(f6Trans02[i]!)?.last_translator_id, trans02.userId);
    }

    for (let i = 0; i < 4; i++) {
        assert.assertEquals(unitByIdF6.get(f6Trans03[i]!)?.translated_text, `C${i}`);
        assert.assertEquals(unitByIdF6.get(f6Trans03[i]!)?.last_translator_id, trans03.userId);
    }

    // export unit_index 0..11
    const exportF6 = await exportPoprako(ctx.sadmin, mainChapterId);
    const p1ExportF6 = exportF6.pages.find((p) => p.page_id === p1Id)!;

    assert.assertEquals(p1ExportF6.units.length, 12);

    [...p1ExportF6.units]
        .sort((a, b) => a.unit_index - b.unit_index)
        .forEach((u, i) => assert.assertEquals(u.unit_index, i, `F6 export unit_index ${i}`));

    await assertPageUnitInvariant(ctx.sadmin, p1Id);
    await assertPageExportInvariant(ctx.sadmin, mainChapterId, p1Id);

    // ---------- F7. same-unit concurrent write (last-write-wins) ----------

    const f7Target = p1UnitIds[0]!;
    const f7Before = unitByIdF6.get(f7Target)!;
    const f7OldUpdatedAt = f7Before.updated_at;

    const f7Results = await Promise.allSettled([
        savePageUnits(trans01.api, p1Id, [
            updateUnit(f7Target, {
                is_bubble: true,
                is_proofread: false,
                translated_text: "A version",
            }),
        ]),
        savePageUnits(trans02.api, p1Id, [
            updateUnit(f7Target, {
                is_bubble: true,
                is_proofread: false,
                translated_text: "B version",
            }),
        ]),
    ]);

    // Both eventually succeed; the later committed Patch wins.
    for (const r of f7Results) {
        assert.assertEquals(r.status, "fulfilled", "F7 Patch must commit");

        if (r.status === "fulfilled") {
            assert.assertEquals(r.value.total_unit_count, 12, "F7 success must keep count");
        }
    }

    const p1AfterF7 = await listPageUnits(ctx.sadmin, p1Id);

    const f7Final = p1AfterF7.unit_infos.find((u) => u.id === f7Target)!;

    assert.assert(f7Final, "F7 target still exists");
    assert.assert(
        f7Final.translated_text === "A version" || f7Final.translated_text === "B version",
        "final text must be one of the two submitted values",
    );
    assert.assertEquals(p1AfterF7.total_unit_count, 12, "count unchanged");
    assert.assert(f7Final.updated_at >= f7OldUpdatedAt, "updated_at must not decrease");

    // ---------- F8. parallel next_id inserts before the same anchor ----------

    const f8Anchor = p1UnitIds[3]!;
    const f8AnchorOldIndex = p1AfterF7.unit_infos.findIndex((u) => u.id === f8Anchor);

    const f8Results = await Promise.allSettled([
        savePageUnits(trans01.api, p1Id, [
            {
                edit: "create",
                local_id: "A_before_anchor",
                next_id: f8Anchor,
                is_bubble: true,
                coord: {
                    x_coord: 0.5,
                    y_coord: 0.5,
                },
            },
        ]),
        savePageUnits(trans02.api, p1Id, [
            {
                edit: "create",
                local_id: "B_before_anchor",
                next_id: f8Anchor,
                is_bubble: true,
                coord: {
                    x_coord: 0.6,
                    y_coord: 0.6,
                },
            },
        ]),
    ]);

    for (const r of f8Results) {
        assert.assertEquals(
            r.status,
            "fulfilled",
            `both F8 inserts must eventually succeed: ${r.status === "rejected" ? String(r.reason) : ""}`,
        );
    }

    const f8SuccessCount = f8Results.length;

    const p1AfterF8 = await listPageUnits(ctx.sadmin, p1Id);
    const priorIds = new Set(p1AfterF7.unit_infos.map((unit) => unit.id));
    const f8NewIds = p1AfterF8.unit_infos
        .filter((unit) => !priorIds.has(unit.id))
        .map((unit) => unit.id);

    assert.assertEquals(p1AfterF8.total_unit_count, 12 + f8SuccessCount, "total increased by success count");

    // new unit ids unique
    assert.assertEquals(new Set(f8NewIds).size, f8NewIds.length, "F8 new ids unique");

    // export: all new units appear before the anchor; anchor's later relative
    // order preserved; unit_index contiguous.
    const exportF8 = await exportPoprako(ctx.sadmin, mainChapterId);
    const p1ExportF8 = exportF8.pages.find((p) => p.page_id === p1Id)!;

    const f8Order = [...p1ExportF8.units].sort((a, b) => a.unit_index - b.unit_index);

    // unit_index contiguous 0..n-1
    f8Order.forEach((u, i) => assert.assertEquals(u.unit_index, i, `F8 export unit_index ${i}`));

    const f8AnchorExport = f8Order.find((u) => u.unit_id === f8Anchor)!;

    // every new unit is before the anchor
    for (const newId of f8NewIds) {
        const newExport = f8Order.find((u) => u.unit_id === newId)!;

        assert.assert(
            newExport.unit_index < f8AnchorExport.unit_index,
            "F8 new unit must be before the anchor",
        );
    }

    // anchor's index increased by the number of new units before it
    assert.assertEquals(
        f8AnchorExport.unit_index,
        f8AnchorOldIndex + f8SuccessCount,
        "anchor index shifted by inserted count",
    );

    // raw_01/raw_02 were referenced in the plan for F8 but they CANNOT save
    // units (RAW_PROVIDER only). The plan's F8 used raw_01/raw_02 as inserters;
    // adjusted to trans_01/trans_02 here because unit save needs their roles.
    void raw01;
    void raw02;

    await assertPageExportInvariant(ctx.sadmin, mainChapterId, p1Id);

    // ---------- F9. delete + update same unit in parallel ----------

    const f9Target = p1UnitIds[1]!;
    const f9Before = p1AfterF8.unit_infos.find((u) => u.id === f9Target);

    assert.assert(f9Before, "F9 target must exist before parallel op");

    const f9CountBefore = p1AfterF8.total_unit_count;

    const f9Results = await Promise.allSettled([
        savePageUnits(trans01.api, p1Id, [
            updateUnit(f9Target, {
                is_bubble: true,
                is_proofread: false,
                translated_text: "X-updated",
            }),
        ]),
        savePageUnits(trans01.api, p1Id, [deleteUnit(f9Target)]),
    ]);

    for (const r of f9Results) {
        if (r.status === "fulfilled") {
            // Distinguish delete vs update by the response: delete returns no
            // mappers; update of an existing id also returns no mappers. We
            // cannot reliably tell which succeeded from the response alone, so
            // rely on the final state below.
            void r.value;
        } else {
            assert.assert(
                /422|status/i.test(String(r.reason)),
                `F9 rejection must be 422, got: ${String(r.reason)}`,
            );
        }
    }

    const p1AfterF9 = await listPageUnits(ctx.sadmin, p1Id);

    const f9Final = p1AfterF9.unit_infos.find((u) => u.id === f9Target);

    // Acceptable: target is gone (delete won) OR target still exists with
    // updated text (update won and delete rejected). Count must be consistent.
    if (f9Final) {
        // update won; delete rejected -> count unchanged
        assert.assertEquals(
            p1AfterF9.total_unit_count,
            f9CountBefore,
            "if target survives, count must not change",
        );
        assert.assertEquals(f9Final.translated_text, "X-updated");
    } else {
        // delete won -> count -1
        assert.assertEquals(
            p1AfterF9.total_unit_count,
            f9CountBefore - 1,
            "if target deleted, count must drop by exactly 1",
        );
    }

    // forbidden: count -2
    assert.assert(
        p1AfterF9.total_unit_count >= f9CountBefore - 1,
        "F9 count must not drop by more than 1",
    );

    // translated count consistent with remaining non-empty translated_text
    await assertPageUnitInvariant(ctx.sadmin, p1Id);

    // ---------- final ----------

    await assertChapterPageCountMetricsConsistent(ctx.sadmin, mainChapterId);

    const mainFinal = await getChapter(ctx.sadmin, mainChapterId);

    // document the final p1 state for it_07 via the export
    const finalExport = await exportPoprako(ctx.sadmin, mainChapterId);
    const p1FinalExport = finalExport.pages.find((p) => p.page_id === p1Id)!;

    assert.assert(p1FinalExport.units.length >= 12, "p1 has at least 12 units after F6-F9");

    // Translation edits and the earlier export auto-start their corresponding
    // workflow stages. it_07 resets these through the public revert API.
    assert.assertEquals(stagePhase(mainFinal.stages, "translate"), PHASE.ACTIVE);
    assert.assertEquals(stagePhase(mainFinal.stages, "typeset-redraw"), PHASE.ACTIVE);
    assert.assertEquals(stagePhase(mainFinal.stages, "raw-provide"), PHASE.PENDING);
    assert.assertEquals(stagePhase(mainFinal.stages, "proofread"), PHASE.PENDING);
    assert.assertEquals(stagePhase(mainFinal.stages, "review"), PHASE.PENDING);
    assert.assertEquals(stagePhase(mainFinal.stages, "publish"), PHASE.PENDING);
}
