// it_00 — Bootstrap, auth, and default seed discovery.
//
// Preconditions:
//   - Database has been reset to seed state by `main.ts` (sadmin user,
//     default team, default member).
//   - `ctx.sadmin` is a fresh unauthenticated ApiClient.
//   - `ctx.ids.defaultTeamId` / `defaultUserId` / `defaultMemberId` are set
//     from `seedIds`.
//
// Postconditions:
//   - `ctx.sadmin` is authenticated (bearer token set).
//   - `ctx.ids.defaultTeamId` confirmed via API.
//   - No extra rows created; safe to run before any other module.
//
// Covers test-plan: A1 (sadmin login + default-data discovery),
// A2 (unauthenticated-access protection).
//
// Status: IMPLEMENTED.

import * as assert from "@std/assert";

import { seedIds } from "../db/seed.ts";
import { expectError, expectSuccessData } from "../http/assertions.ts";
import type { ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { ApiClient } from "../http/apiClient.ts";
import {
    assertCreatedBeforeUpdated,
    assertTimestampMs,
    getMyInfo,
    getTeam,
    listMyMembers,
    listTeams,
    login,
    logout,
} from "../http/fixtures.ts";
import { testEnv } from "../config/env.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

export async function runIt00Module(ctx: RunCtx): Promise<void> {
    // ---- A1. sadmin login and default-data discovery ----

    const loginVal = await login(ctx.sadmin, "123456", "123456");

    assert.assertEquals(loginVal.user_id, seedIds.defaultUserId);

    const me = await getMyInfo(ctx.sadmin);

    assert.assertEquals(me.id, seedIds.defaultUserId);
    assert.assertEquals(me.is_sadmin, true);

    // /members/me?incl=team&offset=0&limit=20 returns at least the seed member.
    const myMembers = await listMyMembers(ctx.sadmin, "&incl=team");

    assert.assert(myMembers.length >= 1, "sadmin must have at least one membership");

    const defaultMember = myMembers.find((member) => member.team_id === ctx.ids.defaultTeamId);

    assert.assert(defaultMember, "sadmin must be a member of the default team");
    assert.assertEquals(defaultMember?.user_id, seedIds.defaultUserId);
    assert.assert(defaultMember?.team, "incl=team must embed the team on /members/me");

    // /teams (no user_id) for sadmin returns 200 and includes the default team.
    const teams = await listTeams(ctx.sadmin);

    const defaultTeam = teams.find((team) => team.id === ctx.ids.defaultTeamId);

    assert.assert(defaultTeam, "default team must be present in /teams");

    // default team exists and has valid timestamps.
    const teamInfo = await getTeam(ctx.sadmin, ctx.ids.defaultTeamId);

    assert.assert(teamInfo.id, "team id must be present");

    // Timestamp fields are Unix-ms integers and created_at <= updated_at.
    assertTimestampMs(teamInfo.created_at);
    assertTimestampMs(teamInfo.updated_at);
    assertCreatedBeforeUpdated(teamInfo.created_at, teamInfo.updated_at);

    for (const member of myMembers) {
        assertTimestampMs(member.last_active_at);
    }

    // ---- A2. unauthenticated-access protection ----

    const anon = new ApiClient(testEnv.apiBaseUrl);

    expectError(await anon.get<ErrorBody>("/api/v1/users/me"), 401, 3);
    expectError(await anon.get<ErrorBody>("/api/v1/members/me?offset=0&limit=20"), 401, 3);
    expectError(await anon.get<ErrorBody>("/api/v1/teams?offset=0&limit=20"), 401, 3);
    expectError(
        await anon.post<ErrorBody>("/api/v1/worksets", {
            description: "x",
            name: "x",
            team_id: ctx.ids.defaultTeamId,
        }),
        401,
        3,
    );

    // logout clears the token; subsequent /users/me is 401.
    // Use a throwaway client so we don't destroy ctx.sadmin's session.
    const throwaway = new ApiClient(testEnv.apiBaseUrl);

    await login(throwaway, "123456", "123456");
    expectSuccessData(await throwaway.get<SuccessBody<{ id: string }>>("/api/v1/users/me"), 200);

    await logout(throwaway);
    expectError(await throwaway.get<ErrorBody>("/api/v1/users/me"), 401, 3);

    // Sanity: ctx.sadmin is still authenticated after A2 (we used `anon` and
    // `throwaway`, not ctx.sadmin, for the negative cases).
    expectSuccessData(
        await ctx.sadmin.get<SuccessBody<{ id: string }>>("/api/v1/users/me"),
        200,
    );
}
