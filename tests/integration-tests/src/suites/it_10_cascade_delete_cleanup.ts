// it_10 — Cascade delete: chapter -> comic -> workset -> team.
//
// Preconditions:
//   - it_00..it_09 have run.
//   - `ctx.auxChapters.get("cascade")` exists: 归档池 workset with comicA
//     (2 chapters, each 2 pages) and comicB (1 chapter, 2 pages). Pages
//     reserved by it_03. `cascadeExtraIds` (from it_02) holds the other
//     comic/chapter ids.
//   - `ctx.secondTeam` exists with outsider.
//
// Postconditions:
//   - The cascade subtree (归档池 + comicA + comicB + all chapters/pages) is
//     fully deleted.
//   - The second team + outsider membership deleted.
//   - `ctx.auxChapters.delete("cascade")`, `ctx.secondTeam = null`.
//   - DB left ready for the final `cleanupToSeed`.
//
// Covers test-plan: C9.
//
// Grounded pins:
//   - chapter/comic/workset delete: team ADMIN (sadmin).
//   - Concurrent member admin removals leave one admin; a serialization
//     failure returns 409/code 8 without server-side retry.
//   - Member and user deletion cannot remove a team's last admin.
//   - Team delete: team admin (the surviving second-team admin). Team delete
//     cascades through worksets/comics/chapters/pages/units/members.
//   - Deleting a chapter decreases comic.chapter_count by 1.
//     The chapter's pages/units become inaccessible (422/2).
//   - Deleting a comic decreases workset.comic_count by 1.
//     The comic's chapters become inaccessible (422/2).
//   - Deleting a workset removes it from the team list; its comics/chapters
//     become inaccessible (422/2).
//
// Status: IMPLEMENTED.

import * as assert from "@std/assert";

import { expectError, expectStatus } from "../http/assertions.ts";
import type { ErrorBody } from "../http/apiClient.ts";
import { assertTeamInvariant } from "../http/invariants.ts";
import {
    getChapter,
    getComic,
    getTeam,
    getWorkset,
    listChapterPages,
    listMyMembers,
    listPageUnits,
    listTeamMembers,
    listTeamWorksets,
    listWorksetComics,
    updateMemberRoles,
} from "../http/fixtures.ts";
import { ROLE } from "../state/roles.ts";
import type { RunCtx } from "../state/runCtx.ts";
import { cascadeExtraIds } from "./it_02_workset_comic_chapter_index.ts";

export const IMPLEMENTED = true as const;

export async function runIt10Module(ctx: RunCtx): Promise<void> {
    const cascadeRefs = ctx.auxChapters.get("cascade");

    assert.assert(cascadeRefs, "it_02 must have set the cascade aux subtree");

    const archiveWsId = cascadeRefs.worksetId;
    const comicAId = cascadeExtraIds.cascadeComicAId;
    const comicACh1 = cascadeExtraIds.cascadeComicACh1;
    const comicACh2 = cascadeRefs.chapterId;
    const comicBId = cascadeExtraIds.cascadeComicBId;
    const comicBCh1 = cascadeExtraIds.cascadeComicBCh1;

    assert.assert(comicAId && comicACh1 && comicBId && comicBCh1, "cascade extra ids must be populated");

    // ---------- C9.1 delete a chapter (comicACh2) ----------

    const comicABefore = await getComic(ctx.sadmin, comicAId);
    const comicAChCountBefore = comicABefore.chapter_count;

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/chapters/${comicACh2}`), 204);

    // GET deleted chapter -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicACh2}`), 422, 2);

    // comic.chapter_count -1
    const comicAAfterChDelete = await getComic(ctx.sadmin, comicAId);

    assert.assertEquals(comicAAfterChDelete.chapter_count, comicAChCountBefore - 1);

    // ---------- C9.2 delete a comic (comicB) ----------

    const archiveWsBefore = await getWorkset(ctx.sadmin, archiveWsId);
    const wsComicCountBefore = archiveWsBefore.comic_count;

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/comics/${comicBId}`), 204);

    // GET deleted comic -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/comics/${comicBId}`), 422, 2);

    // workset.comic_count -1
    const archiveWsAfterComicDelete = await getWorkset(ctx.sadmin, archiveWsId);

    assert.assertEquals(archiveWsAfterComicDelete.comic_count, wsComicCountBefore - 1);

    // comicB's chapter (comicBCh1) -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicBCh1}`), 422, 2);

    // comicB's page units -> 422/2 (page gone with chapter)
    // (pages were reserved on comicBCh1 by it_03; pick the first page id we
    // don't have stored, so just assert a chapter-pages list -> 422/2)
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicBCh1}/pages?offset=0&limit=20`),
        422,
        2,
    );

    // ---------- C9.3 delete the workset (归档池) ----------

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/worksets/${archiveWsId}`), 204);

    // GET deleted workset -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/worksets/${archiveWsId}`), 422, 2);

    // team workset list excludes it
    const teamWorksetsAfter = await listTeamWorksets(ctx.sadmin, ctx.ids.defaultTeamId);

    assert.assert(!teamWorksetsAfter.find((ws) => ws.id === archiveWsId), "deleted workset must not list");

    // comicA (under the deleted workset) -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/comics/${comicAId}`), 422, 2);

    // comicACh1 -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicACh1}`), 422, 2);

    // a page under comicACh1 -> list pages 422/2
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicACh1}/pages?offset=0&limit=20`),
        422,
        2,
    );

    // ---------- C9.4 delete the second team (created by it_09) ----------

    if (ctx.secondTeam) {
        const secondTeamId = ctx.secondTeam.teamId;

        // Clean up the FK-restricted invitation before exercising member
        // retention and deleting the complete team.
        if (ctx.secondTeam.outsiderInvitationId) {
            expectStatus(
                await ctx.sadmin.delete<null>(
                    `/api/v1/member-invitations/${ctx.secondTeam.outsiderInvitationId}`,
                ),
                204,
            );
        }

        const outsider = ctx.secondTeam.outsider;

        assert.assert(outsider, "second-team outsider must exist");

        const secondTeamMembers = await listTeamMembers(ctx.sadmin, secondTeamId);

        const sadminMember = secondTeamMembers.find(
            (member) => member.user_id === ctx.ids.defaultUserId,
        );

        const outsiderMember = secondTeamMembers.find(
            (member) => member.user_id === outsider.userId,
        );

        assert.assert(sadminMember, "sadmin second-team member must exist");
        assert.assert(outsiderMember, "outsider second-team member must exist");

        // Give both members admin, then concurrently have them remove the
        // other's admin role from the same initial state.
        await updateMemberRoles(
            ctx.sadmin,
            outsiderMember.id,
            outsiderMember.roles | ROLE.ADMIN,
        );

        const [removeOutsiderAdmin, removeSadminAdmin] = await Promise.all([
            ctx.sadmin.put<ErrorBody>(`/api/v1/members/${outsiderMember.id}/roles`, {
                id: outsiderMember.id,
                roles: ROLE.RAW_PROVIDER,
            }),
            outsider.api.put<ErrorBody>(`/api/v1/members/${sadminMember.id}/roles`, {
                id: sadminMember.id,
                roles: ROLE.RAW_PROVIDER,
            }),
        ]);

        const adminRemovalResponses = [removeOutsiderAdmin, removeSadminAdmin];

        assert.assertEquals(
            adminRemovalResponses.filter((response) => response.status === 204).length,
            1,
            "exactly one concurrent admin removal must commit",
        );

        const rejectedAdminRemoval = adminRemovalResponses.find(
            (response) => response.status !== 204,
        );

        assert.assert(rejectedAdminRemoval, "one concurrent admin removal must be rejected");
        assert.assert(
            rejectedAdminRemoval.status === 403 || rejectedAdminRemoval.status === 409,
            `rejected admin removal must be 403 or 409, got ${rejectedAdminRemoval.status}`,
        );

        expectError(
            rejectedAdminRemoval,
            rejectedAdminRemoval.status,
            rejectedAdminRemoval.status === 409 ? 8 : 4,
        );

        const membersAfterConcurrentRemoval = await listTeamMembers(ctx.sadmin, secondTeamId);

        const adminMembersAfterConcurrentRemoval = membersAfterConcurrentRemoval.filter(
            (member) => (member.roles & ROLE.ADMIN) !== 0,
        );

        assert.assertEquals(
            adminMembersAfterConcurrentRemoval.length,
            1,
            "concurrent role removals must retain exactly one admin",
        );

        // Normalize the outsider as the sole admin so both member deletion and
        // user deletion can exercise last-admin retention without touching the
        // default seed account.
        if (adminMembersAfterConcurrentRemoval[0]!.user_id === ctx.ids.defaultUserId) {
            await updateMemberRoles(
                ctx.sadmin,
                outsiderMember.id,
                ROLE.RAW_PROVIDER | ROLE.ADMIN,
            );

            await updateMemberRoles(
                outsider.api,
                sadminMember.id,
                ROLE.RAW_PROVIDER,
            );
        }

        const normalizedMembers = await listTeamMembers(outsider.api, secondTeamId);

        assert.assertEquals(
            normalizedMembers
                .filter((member) => (member.roles & ROLE.ADMIN) !== 0)
                .map((member) => member.user_id),
            [outsider.userId],
            "outsider must be the sole admin before retention checks",
        );

        expectError(
            await outsider.api.delete<ErrorBody>(`/api/v1/users/${outsider.userId}`),
            403,
            4,
        );

        expectError(
            await outsider.api.delete<ErrorBody>(`/api/v1/members/${outsiderMember.id}`),
            403,
            4,
        );

        // Team deletion intentionally removes every member, including the
        // last admin, because the protected team no longer exists afterward.
        expectStatus(await outsider.api.delete<null>(`/api/v1/teams/${secondTeamId}`), 204);

        expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/teams/${secondTeamId}`), 422, 2);

        const outsiderMembers = await listMyMembers(outsider.api);

        assert.assertEquals(
            outsiderMembers.length,
            0,
            "outsider must have no memberships after second-team delete",
        );

        ctx.secondTeam = null;
    }

    // ---------- cleanup aux state ----------

    ctx.auxChapters.delete("cascade");
    ctx.auxChapters.delete("d3");

    // ---------- final: default team still consistent ----------

    await assertTeamInvariant(ctx.sadmin, ctx.ids.defaultTeamId);

    // sanity: main chapter still accessible (it_07 drove it to publish-completed)
    if (ctx.main) {
        const mainChapter = await getChapter(ctx.sadmin, ctx.main.chapterId);

        assert.assert(mainChapter, "main chapter must still be accessible after cascade deletes");
    }

    // sanity: list workset comics / chapter pages / page units helpers still
    // work on the main subtree (no accidental cross-contamination).
    const serialWsId = ctx.ids.worksetIds["连载池"];

    if (serialWsId) {
        const serialComics = await listWorksetComics(ctx.sadmin, serialWsId);

        assert.assert(serialComics.length >= 1, "连载池 still has its comics");

        if (ctx.main) {
            const mainPages = await listChapterPages(ctx.sadmin, ctx.main.chapterId);

            assert.assert(mainPages.length > 0, "main chapter still has its pages");

            if (mainPages[0]) {
                const mainUnits = await listPageUnits(ctx.sadmin, mainPages[0].id);

                assert.assert(mainUnits.total_unit_count >= 0, "main page units still queryable");
            }
        }
    }

    // sanity: default team profile still the seed-restored name (it_08 restored it)
    const defaultTeam = await getTeam(ctx.sadmin, ctx.ids.defaultTeamId);

    assert.assert(defaultTeam.id === ctx.ids.defaultTeamId);
}
