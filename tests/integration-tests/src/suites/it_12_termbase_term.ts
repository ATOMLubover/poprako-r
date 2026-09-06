// it_12 — Termbase and term lifecycle, lookup, perms, and cascades.
//
// Preconditions:
//   - The default team, main comic, proofreader, and translator exist.
//
// Postconditions:
//   - All terminology fixtures and temporary comic/team roots are deleted.
//
// Covers team/comic scopes, comic inheritance, name/source-only fuzzy search,
// full replacement, target order, counters, native import/export and merge,
// perm isolation, path/body identity, duplicate normalization, and cascades.

import * as assert from "@std/assert";

import { expectError, expectNoContent, expectSuccessData, expectSuccessList } from "../http/assertions.ts";
import type { ErrorBody, SuccessBody } from "../http/apiClient.ts";
import {
    createComic,
    createTeam,
    createWorkset,
    deleteWorkset,
    listMyMembers,
    updateMemberRoles,
} from "../http/fixtures.ts";
import type {
    ExportTermbaseVal,
    IdVal,
    ImportTermbaseInstr,
    ImportTermbaseVal,
    TermbaseInfoView,
    TermInfoView,
} from "../http/types.ts";
import { titled } from "../state/prefix.ts";
import { ROLE } from "../state/roles.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

async function createTermbase(
    api: RunCtx["sadmin"],
    scope: { team_id: string; comic_id?: never } | { team_id?: never; comic_id: string },
    name: string,
    description: string | null,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/termbases", {
            ...scope,
            description,
            name,
        }),
        201,
    );
}

async function createTerm(
    api: RunCtx["sadmin"],
    termbaseId: string,
    source: string,
    targets: string[],
    comment: string | null,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/terms", {
            comment,
            source,
            targets,
            termbase_id: termbaseId,
        }),
        201,
    );
}

async function getTermbase(api: RunCtx["sadmin"], termbaseId: string): Promise<TermbaseInfoView> {
    return expectSuccessData(await api.get(`/api/v1/termbases/${termbaseId}`), 200);
}

async function getTerm(api: RunCtx["sadmin"], termId: string): Promise<TermInfoView> {
    return expectSuccessData(await api.get(`/api/v1/terms/${termId}`), 200);
}

export async function runIt12Module(ctx: RunCtx): Promise<void> {
    const mainComicId = ctx.ids.comicIds["星尘旅人"];
    const proofreader = ctx.users.get("proof_01");
    const translator = ctx.users.get("trans_01");

    assert.assert(mainComicId, "it_02 must have created the main comic");
    assert.assert(proofreader, "proof_01 must exist");
    assert.assert(translator, "trans_01 must exist");

    const teamTermbase = await createTermbase(
        proofreader.api,
        { team_id: ctx.ids.defaultTeamId },
        " Shared Glossary ",
        "team description",
    );
    const comicTermbase = await createTermbase(
        translator.api,
        { comic_id: mainComicId },
        " Main Comic Glossary ",
        "comic description",
    );

    const teamInfo = await getTermbase(translator.api, teamTermbase.id);

    assert.assertEquals(teamInfo.name, "Shared Glossary");
    assert.assertEquals(teamInfo.team_id, ctx.ids.defaultTeamId);
    assert.assertEquals(teamInfo.comic_id, undefined);
    assert.assertEquals(teamInfo.term_count, 0);

    expectNoContent(
        await translator.api.put<null>(`/api/v1/termbases/${comicTermbase.id}`, {
            description: null,
            id: comicTermbase.id,
            name: " Main Comic Glossary Updated ",
        }),
    );

    const updatedComicInfo = await getTermbase(translator.api, comicTermbase.id);

    assert.assertEquals(updatedComicInfo.name, "Main Comic Glossary Updated");
    assert.assertEquals(updatedComicInfo.description, undefined);

    const visibleFromComic = expectSuccessList<TermbaseInfoView>(
        await translator.api.get<SuccessBody<TermbaseInfoView[]>>(
            `/api/v1/comics/${mainComicId}/termbases?offset=0&limit=20`,
        ),
        200,
    );

    assert.assertEquals(
        new Set(visibleFromComic.map((termbase) => termbase.id)),
        new Set([teamTermbase.id, comicTermbase.id]),
    );

    const fuzzyByName = expectSuccessList<TermbaseInfoView>(
        await translator.api.get<SuccessBody<TermbaseInfoView[]>>(
            `/api/v1/comics/${mainComicId}/termbases?fuzzy_name=${encodeURIComponent("main comic")}&offset=0&limit=20`,
        ),
        200,
    );

    assert.assertEquals(fuzzyByName.map((termbase) => termbase.id), [comicTermbase.id]);

    const fuzzyByDescription = expectSuccessList<TermbaseInfoView>(
        await translator.api.get<SuccessBody<TermbaseInfoView[]>>(
            `/api/v1/comics/${mainComicId}/termbases?fuzzy_name=${encodeURIComponent("description")}&offset=0&limit=20`,
        ),
        200,
    );

    assert.assertEquals(fuzzyByDescription.length, 0);

    const term = await createTerm(
        translator.api,
        comicTermbase.id,
        " Hero ",
        [" 勇者 ", "英雄"],
        " main character ",
    );
    const termInfo = await getTerm(translator.api, term.id);

    assert.assertEquals(termInfo.source, "Hero");
    assert.assertEquals(termInfo.targets, ["勇者", "英雄"]);
    assert.assertEquals(termInfo.comment, "main character");
    assert.assertEquals((await getTermbase(translator.api, comicTermbase.id)).term_count, 1);

    expectError(
        await proofreader.api.post<ErrorBody>("/api/v1/terms", {
            comment: null,
            source: " hero ",
            targets: ["重复"],
            termbase_id: comicTermbase.id,
        }),
        422,
        2,
    );

    expectError(
        await proofreader.api.put<ErrorBody>(`/api/v1/terms/${term.id}`, {
            comment: null,
            id: "different-id",
            source: "Heroine",
            targets: ["女主角"],
        }),
        422,
        7,
    );

    expectNoContent(
        await translator.api.put<null>(`/api/v1/terms/${term.id}`, {
            comment: "   ",
            id: term.id,
            source: " Heroine ",
            targets: [" 女主角 ", "主角"],
        }),
    );

    const updatedTerm = await getTerm(translator.api, term.id);

    assert.assertEquals(updatedTerm.source, "Heroine");
    assert.assertEquals(updatedTerm.targets, ["女主角", "主角"]);
    assert.assertEquals(updatedTerm.comment, undefined);

    const termFuzzy = expectSuccessList<TermInfoView>(
        await translator.api.get<SuccessBody<TermInfoView[]>>(
            `/api/v1/termbases/${comicTermbase.id}/terms?fuzzy_source=heroine&offset=0&limit=20`,
        ),
        200,
    );

    assert.assertEquals(termFuzzy.map((listedTerm) => listedTerm.id), [term.id]);

    const targetFuzzy = expectSuccessList<TermInfoView>(
        await translator.api.get<SuccessBody<TermInfoView[]>>(
            `/api/v1/termbases/${comicTermbase.id}/terms?fuzzy_source=${
                encodeURIComponent("女主角")
            }&offset=0&limit=20`,
        ),
        200,
    );

    assert.assertEquals(targetFuzzy.length, 0);

    const nativeDocument: ImportTermbaseInstr = {
        name: "Native Port Glossary",
        description: "Imported description",
        terms: [
            {
                source: "Beta",
                targets: ["乙"],
                comment: null,
            },
            {
                source: "Alpha",
                targets: ["甲"],
                comment: "initial",
            },
        ],
    };

    const importedTermbase = expectSuccessData<ImportTermbaseVal>(
        await translator.api.post<SuccessBody<ImportTermbaseVal>>(
            `/api/v1/teams/${ctx.ids.defaultTeamId}/termbases/import`,
            nativeDocument,
        ),
        201,
    );

    assert.assertEquals(importedTermbase.created, true);
    assert.assertEquals(importedTermbase.created_term_count, 2);
    assert.assertEquals(importedTermbase.merged_term_count, 0);

    const exportedResponse = await translator.api.get<ExportTermbaseVal>(
        `/api/v1/termbases/${importedTermbase.id}/export`,
    );

    assert.assertEquals(exportedResponse.status, 200);
    assert.assertEquals(exportedResponse.headers.get("content-type"), "application/json");

    const exportedDocument = JSON.parse(exportedResponse.rawText) as ExportTermbaseVal;

    assert.assertEquals(exportedDocument.terms.map((entry) => entry.source), ["Alpha", "Beta"]);

    expectError(
        await translator.api.post<ErrorBody>(
            `/api/v1/teams/${ctx.ids.defaultTeamId}/termbases/import`,
            nativeDocument,
        ),
        422,
        2,
    );

    const mergedDocument: ImportTermbaseInstr = {
        ...nativeDocument,
        description: null,
        terms: [
            {
                source: " alpha ",
                targets: ["甲", "第一"],
                comment: "merged",
            },
            {
                source: "Gamma",
                targets: ["丙"],
                comment: null,
            },
        ],
    };

    const mergedTermbase = expectSuccessData<ImportTermbaseVal>(
        await translator.api.post<SuccessBody<ImportTermbaseVal>>(
            `/api/v1/teams/${ctx.ids.defaultTeamId}/termbases/import?force_merge=true`,
            mergedDocument,
        ),
        200,
    );

    assert.assertEquals(mergedTermbase.id, importedTermbase.id);
    assert.assertEquals(mergedTermbase.created, false);
    assert.assertEquals(mergedTermbase.created_term_count, 1);
    assert.assertEquals(mergedTermbase.merged_term_count, 1);

    const mergedExportResponse = await translator.api.get<ExportTermbaseVal>(
        `/api/v1/termbases/${importedTermbase.id}/export/download`,
    );

    assert.assertEquals(mergedExportResponse.status, 200);
    assert.assertEquals(
        mergedExportResponse.headers.get("content-disposition"),
        `attachment; filename="termbase_${importedTermbase.id}.json"`,
    );

    const mergedExport = JSON.parse(mergedExportResponse.rawText) as ExportTermbaseVal;
    const alpha = mergedExport.terms.find((entry) => entry.source === "alpha");

    assert.assert(alpha);
    assert.assertEquals(alpha.targets, ["甲", "第一"]);
    assert.assertEquals(alpha.comment, "merged");
    assert.assertEquals(mergedExport.description, null);

    expectError(
        await translator.api.post<ErrorBody>(
            `/api/v1/teams/${ctx.ids.defaultTeamId}/termbases/import`,
            {
                name: "Oversized Native Glossary",
                description: null,
                terms: Array.from({ length: 201 }, (_, index) => ({
                    source: `Source ${index}`,
                    targets: [`Target ${index}`],
                    comment: null,
                })),
            },
        ),
        422,
        2,
    );

    expectNoContent(
        await translator.api.delete<null>(`/api/v1/termbases/${importedTermbase.id}`),
    );

    expectNoContent(await translator.api.delete<null>(`/api/v1/terms/${term.id}`));
    assert.assertEquals((await getTermbase(translator.api, comicTermbase.id)).term_count, 0);

    const cascadeWorkset = await createWorkset(
        ctx.sadmin,
        ctx.ids.defaultTeamId,
        titled("termbase-cascade-workset"),
    );
    const cascadeComic = await createComic(
        ctx.sadmin,
        cascadeWorkset.id,
        titled("termbase-cascade-comic"),
        "integration",
    );
    const cascadeTermbase = await createTermbase(
        proofreader.api,
        { comic_id: cascadeComic.id },
        "Cascade Comic Glossary",
        null,
    );
    const cascadeTerm = await createTerm(
        proofreader.api,
        cascadeTermbase.id,
        "Cascade",
        ["级联"],
        null,
    );

    expectNoContent(await ctx.sadmin.delete<null>(`/api/v1/comics/${cascadeComic.id}`));
    expectError(await translator.api.get<ErrorBody>(`/api/v1/termbases/${cascadeTermbase.id}`), 422, 2);
    expectError(await translator.api.get<ErrorBody>(`/api/v1/terms/${cascadeTerm.id}`), 422, 2);
    await deleteWorkset(ctx.sadmin, cascadeWorkset.id);

    const cascadeTeam = await createTeam(ctx.sadmin, titled("termbase-cascade-team"), "cascade");

    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/termbases", {
            comic_id: null,
            description: null,
            name: "Sadmin Must Not Bypass",
            team_id: cascadeTeam.id,
        }),
        403,
        4,
    );

    const memberships = await listMyMembers(ctx.sadmin);
    const cascadeMembership = memberships.find((member) => member.team_id === cascadeTeam.id);

    assert.assert(cascadeMembership, "team creator membership must exist");

    await updateMemberRoles(
        ctx.sadmin,
        cascadeMembership.id,
        ROLE.ADMIN | ROLE.PROOFREADER,
    );

    const cascadeTeamTermbase = await createTermbase(
        ctx.sadmin,
        { team_id: cascadeTeam.id },
        "Cascade Team Glossary",
        null,
    );
    const cascadeTeamTerm = await createTerm(
        ctx.sadmin,
        cascadeTeamTermbase.id,
        "Team Cascade",
        ["团队级联"],
        null,
    );

    expectNoContent(await ctx.sadmin.delete<null>(`/api/v1/teams/${cascadeTeam.id}`));
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/termbases/${cascadeTeamTermbase.id}`), 422, 2);
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/terms/${cascadeTeamTerm.id}`), 422, 2);

    expectNoContent(await translator.api.delete<null>(`/api/v1/termbases/${comicTermbase.id}`));
    expectNoContent(await proofreader.api.delete<null>(`/api/v1/termbases/${teamTermbase.id}`));
}
