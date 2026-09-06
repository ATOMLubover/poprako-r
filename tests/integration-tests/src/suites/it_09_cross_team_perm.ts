// it_09 — Cross-team isolation, outsider permissions, and online-user leases.
//
// Preconditions:
//   - it_00 + it_01 have run. it_02 has run (default team has worksets/comics
//     to probe isolation against).
//
// Postconditions:
//   - `ctx.secondTeam = { teamId, outsider: UserClient }`.
//   - `ctx.users.set("outsider_01", outsiderClient)`.
//   - outsider_01 is a member of the SECOND team only (RAW_PROVIDER).
//
// Covers test-plan: I1 and team-scoped online-user presence.
//
// Grounded pins:
//   - team create: sadmin only. sadmin auto-becomes ADMIN of the new team.
//   - team delete: team admin (sadmin for the new team).
//   - workset/comic/chapter list/get on a team the caller is not a member of
//     -> 403/4.
//   - announcement/comment/member-invitation/assignment-invitation create
//     targeting a team the caller is not a member of -> 403/4.
//   - `GET /api/v1/teams?user_id={outsiderId}` returns only teams the outsider
//     joined (the second team), NOT the default team.
//
// Status: IMPLEMENTED.

import * as assert from "@std/assert";

import { testEnv } from "../config/env.ts";
import { expectError, expectNoContent, expectSuccessList } from "../http/assertions.ts";
import type { ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { ApiClient } from "../http/apiClient.ts";
import { createMemberInvitation, createTeam, listMyMembers, listTeams, registerInvitee } from "../http/fixtures.ts";
import { nickname, password, qid, titled } from "../state/prefix.ts";
import { ROLE } from "../state/roles.ts";
import { OUTSIDER_PERSONA, type RunCtx, type UserClient } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

export async function runIt09Module(ctx: RunCtx): Promise<void> {
    const defaultTeamId = ctx.ids.defaultTeamId;
    const trans01 = ctx.users.get("trans_01")!;

    // ---------- I1.1 sadmin creates the second team ----------

    const secondTeam = await createTeam(ctx.sadmin, titled("外包协作组"), "cross-team isolation");

    ctx.secondTeam = { teamId: secondTeam.id, outsider: null, outsiderInvitationId: "", outsiderMemberId: "" };

    // ---------- I1.2 invite outsider_01 to the second team ----------

    const outsiderPersona = { ...OUTSIDER_PERSONA, qid: qid(OUTSIDER_PERSONA.persona) };
    const outsiderInv = await createMemberInvitation(
        ctx.sadmin,
        secondTeam.id,
        outsiderPersona.qid,
        ROLE.RAW_PROVIDER,
    );

    // ---------- I1.3 outsider registers + logs in ----------

    const { api: outsiderApi, userId: outsiderUserId } = await registerInvitee(
        outsiderInv.code,
        outsiderPersona.qid,
        nickname(outsiderPersona.persona),
        password(outsiderPersona.persona),
    );

    const outsiderMembers = await listMyMembers(outsiderApi);

    assert.assertEquals(outsiderMembers.length, 1, "outsider must have exactly one membership (second team)");
    assert.assertEquals(outsiderMembers[0]!.team_id, secondTeam.id, "outsider must be in the second team");
    assert.assertEquals(outsiderMembers[0]!.roles, ROLE.RAW_PROVIDER);

    const outsiderClient: UserClient = {
        persona: outsiderPersona.persona,
        api: outsiderApi,
        userId: outsiderUserId,
        qid: outsiderPersona.qid,
        memberIds: { [secondTeam.id]: outsiderMembers[0]!.id },
        roles: ROLE.RAW_PROVIDER,
    };

    ctx.secondTeam.outsider = outsiderClient;
    ctx.secondTeam.outsiderInvitationId = outsiderInv.id;
    ctx.secondTeam.outsiderMemberId = outsiderMembers[0]!.id;
    ctx.users.set("outsider_01", outsiderClient);

    // ---------- I1.4 outsider cannot see/touch default-team descendants ----------

    // user-scoped team list returns only the second team
    const outsiderTeams = await listTeams(outsiderApi, outsiderUserId);

    assert.assert(
        !outsiderTeams.find((t) => t.id === defaultTeamId),
        "outsider must not see default team in user-scoped list",
    );
    assert.assert(outsiderTeams.find((t) => t.id === secondTeam.id), "outsider must see the second team");

    // outsider list default-team worksets -> 403/4
    expectError(
        await outsiderApi.get<ErrorBody>(`/api/v1/teams/${defaultTeamId}/worksets?offset=0&limit=20`),
        403,
        4,
    );

    // outsider get a default-team workset -> 403/4 (if it_02 created any)
    const defaultWorksetId = ctx.ids.worksetIds["连载池"];

    if (defaultWorksetId) {
        expectError(
            await outsiderApi.get<ErrorBody>(`/api/v1/worksets/${defaultWorksetId}`),
            403,
            4,
        );

        // outsider list comics / chapters / pages / units on default-team descendants
        expectError(
            await outsiderApi.get<ErrorBody>(`/api/v1/worksets/${defaultWorksetId}/comics?offset=0&limit=20`),
            403,
            4,
        );

        const defaultComicId = ctx.ids.comicIds["星尘旅人"];

        if (defaultComicId) {
            expectError(
                await outsiderApi.get<ErrorBody>(`/api/v1/comics/${defaultComicId}`),
                403,
                4,
            );

            expectError(
                await outsiderApi.get<ErrorBody>(`/api/v1/comics/${defaultComicId}/chapters?offset=0&limit=20`),
                403,
                4,
            );

            if (ctx.main) {
                expectError(
                    await outsiderApi.get<ErrorBody>(`/api/v1/chapters/${ctx.main.chapterId}`),
                    403,
                    4,
                );

                expectError(
                    await outsiderApi.get<ErrorBody>(`/api/v1/chapters/${ctx.main.chapterId}/pages?offset=0&limit=20`),
                    403,
                    4,
                );

                expectError(
                    await outsiderApi.get<ErrorBody>(
                        `/api/v1/chapters/${ctx.main.chapterId}/workflow-records?offset=0&limit=20`,
                    ),
                    403,
                    4,
                );
            }
        }
    }

    // outsider create comment / announcement / member invitation / assignment
    // invitation targeting the default team -> 403/4
    expectError(
        await outsiderApi.post<ErrorBody>("/api/v1/comments", {
            content: "x",
            team_id: defaultTeamId,
        }),
        403,
        4,
    );

    expectError(
        await outsiderApi.post<ErrorBody>("/api/v1/announcements", {
            content: "x",
            team_id: defaultTeamId,
            title: "x",
        }),
        403,
        4,
    );

    expectError(
        await outsiderApi.post<ErrorBody>("/api/v1/member-invitations", {
            invitee_qid: qid("outsider_99"),
            roles: ROLE.RAW_PROVIDER,
            team_id: defaultTeamId,
        }),
        403,
        4,
    );

    // outsider receives no default-team workflow mails (no earlier module
    // addressed mails to outsider; the mail list must be empty).
    const outsiderMails = await (await import("../http/fixtures.ts")).listSystemMails(outsiderApi);

    assert.assertEquals(outsiderMails.length, 0, "outsider must have no system mails");

    // ---------- I1.9 default-team members have no second-team powers ----------

    // trans_01 list second-team worksets -> 403/4
    expectError(
        await trans01.api.get<ErrorBody>(`/api/v1/teams/${secondTeam.id}/worksets?offset=0&limit=20`),
        403,
        4,
    );

    // trans_01 create workset in the second team -> 403/4
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/worksets", {
            description: "x",
            name: titled("trans-second-ws"),
            team_id: secondTeam.id,
        }),
        403,
        4,
    );

    // ---------- I1.10 online-user leases stay team-scoped ----------

    expectNoContent(
        await trans01.api.put<null>(`/api/v1/teams/${defaultTeamId}/mark-self-online`),
    );

    // Repeated marks renew the same lease and remain idempotent.
    expectNoContent(
        await trans01.api.put<null>(`/api/v1/teams/${defaultTeamId}/mark-self-online`),
    );

    expectNoContent(
        await ctx.sadmin.put<null>(`/api/v1/teams/${defaultTeamId}/mark-self-online`),
    );

    expectNoContent(
        await outsiderApi.put<null>(`/api/v1/teams/${secondTeam.id}/mark-self-online`),
    );

    expectNoContent(
        await ctx.sadmin.put<null>(`/api/v1/teams/${secondTeam.id}/mark-self-online`),
    );

    const defaultOnlineUserIds = expectSuccessList<string>(
        await ctx.sadmin.get<SuccessBody<string[]>>(
            `/api/v1/teams/${defaultTeamId}/online-users`,
        ),
        200,
    );

    assert.assertEquals(
        defaultOnlineUserIds,
        [ctx.ids.defaultUserId, trans01.userId].sort(),
        "default team online users must be sorted and team-scoped",
    );

    const secondOnlineUserIds = expectSuccessList<string>(
        await outsiderApi.get<SuccessBody<string[]>>(
            `/api/v1/teams/${secondTeam.id}/online-users`,
        ),
        200,
    );

    assert.assertEquals(
        secondOnlineUserIds,
        [ctx.ids.defaultUserId, outsiderUserId].sort(),
        "second team online users must not include default-team-only users",
    );

    expectError(
        await outsiderApi.put<ErrorBody>(
            `/api/v1/teams/${defaultTeamId}/mark-self-online`,
        ),
        403,
        4,
    );

    expectError(
        await outsiderApi.get<ErrorBody>(
            `/api/v1/teams/${defaultTeamId}/online-users`,
        ),
        403,
        4,
    );

    expectError(
        await trans01.api.put<ErrorBody>(
            `/api/v1/teams/${secondTeam.id}/mark-self-online`,
        ),
        403,
        4,
    );

    expectError(
        await trans01.api.get<ErrorBody>(
            `/api/v1/teams/${secondTeam.id}/online-users`,
        ),
        403,
        4,
    );

    const anon = new ApiClient(testEnv.apiBaseUrl);

    expectError(
        await anon.put<ErrorBody>(
            `/api/v1/teams/${defaultTeamId}/mark-self-online`,
        ),
        401,
        3,
    );

    expectError(
        await anon.get<ErrorBody>(
            `/api/v1/teams/${defaultTeamId}/online-users`,
        ),
        401,
        3,
    );

    // ---------- I1.11 sadmin can list all teams ----------

    const allTeams = await listTeams(ctx.sadmin);

    assert.assert(allTeams.find((t) => t.id === defaultTeamId), "sadmin sees default team");
    assert.assert(allTeams.find((t) => t.id === secondTeam.id), "sadmin sees second team");
}
