// it_01 — Member invitation, registration, member list, and role perms.
//
// Preconditions:
//   - it_00 has run: `ctx.sadmin` is authenticated.
//   - Database is seed-only plus whatever it_00 created (nothing).
//
// Postconditions:
//   - `ctx.personas` is set to the 14 default-team personas with real qids.
//   - `ctx.users` contains 14 authenticated UserClients keyed by persona
//     (`raw_01`, `trans_01`, ... `guest_01`).
//   - Each user has a default-team membership with roles equal to the
//     invitation roles (guest_01 widened to RAW|TRANSLATOR by B2).
//   - All 14 invitations are consumed (pending=false).
//
// Covers test-plan: B1 (batch invite 14), B2 (modify/delete invitation),
// B3 (14 register + close the loop), B4 (member list filters + bad params),
// B5 (member role update + perm boundary).
//
// Status: IMPLEMENTED.
//
// Grounding notes (verified against the Rust source — the plan's 409/401
// expectations are adjusted to the real server behaviour):
//   - Duplicate pending invitation for the same (team, qid) hits the partial
//     unique index `uidx_member_invitation_team_id_invitee_qid_pending` and
//     is mapped to 422 code 2 (`error-already-exists`), NOT 409.
//   - Register with a deleted / already-consumed / wrong-qid code: the usecase
//     uses `get_info_by_code_excluded` (excludes consumed) and compares qid,
//     returning `Expected::Args` → 422 code 2, NOT 401.
//   - Composite `role` query filter: `role: Option<RoleField>` rejects
//     multi-bit values at the query-extractor level → 422 (raw serde
//     rejection, no `code` field), so we only assert the status there.

import * as assert from "@std/assert";

import { testEnv } from "../config/env.ts";
import { expectError, expectStatus } from "../http/assertions.ts";
import type { ErrorBody } from "../http/apiClient.ts";
import { ApiClient } from "../http/apiClient.ts";
import { assertMemberListWellFormed } from "../http/invariants.ts";
import {
    createMemberInvitation,
    deleteMemberInvitation,
    getMyInfo,
    listMemberInvitations,
    listMyMembers,
    listTeamMembers,
    registerInvitee,
    updateMemberInvitationRoles,
    updateMemberRoles,
} from "../http/fixtures.ts";
import { nickname, password, qid, runPrefix } from "../state/prefix.ts";
import { ROLE, ROLE_MASK } from "../state/roles.ts";
import { DEFAULT_TEAM_PERSONAS, type MemberPersona, type RunCtx, type UserClient } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

export async function runIt01Module(ctx: RunCtx): Promise<void> {
    const teamId = ctx.ids.defaultTeamId;

    // Build the persona matrix with concrete qids for this run.
    const personas: MemberPersona[] = DEFAULT_TEAM_PERSONAS.map((persona) => ({
        ...persona,
        qid: qid(persona.persona),
    }));

    ctx.personas = personas;

    // ---------- B1. sadmin batch-invites 14 members ----------

    const invitations = new Map<string, { id: string; code: string }>();
    const seenCodes = new Set<string>();

    for (const persona of personas) {
        const invitation = await createMemberInvitation(
            ctx.sadmin,
            teamId,
            persona.qid,
            persona.roles,
        );

        assert.assert(invitation.id);
        assert.assert(invitation.code);
        assert.assert(!seenCodes.has(invitation.code), "invitation codes must be unique");
        seenCodes.add(invitation.code);

        invitations.set(persona.persona, { id: invitation.id, code: invitation.code });
    }

    // B1.3: pending list includes the 14 invitations with correct fields.
    const pendingList = await listMemberInvitations(ctx.sadmin, teamId, true);

    for (const persona of personas) {
        const inv = invitations.get(persona.persona);

        assert.assert(inv, `invitation for ${persona.persona} must exist`);

        const found = pendingList.find((item) => item.id === inv.id);

        assert.assert(found, `pending list must include invitation for ${persona.persona}`);
        assert.assertEquals(found?.is_pending, true);
        assert.assertEquals(found?.team_id, teamId);
        assert.assertEquals(found?.invitee_qid, persona.qid);
        assert.assertEquals(found?.roles, persona.roles);
        assert.assertEquals(found?.invitor_id, ctx.ids.defaultUserId);
    }

    // B1.6: duplicate invitee_qid for a still-pending invitation -> 422 code 2
    // (partial unique index on (team_id, invitee_qid) WHERE pending).
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/member-invitations", {
            invitee_qid: personas[0]!.qid,
            roles: ROLE.TRANSLATOR,
            team_id: teamId,
        }),
        422,
        2,
    );

    // ---------- B2. modify and delete invitation ----------

    // B2.1: widen guest_01's roles to RAW | TRANSLATOR.
    const guestInv = invitations.get("guest_01");

    assert.assert(guestInv, "guest_01 invitation must exist");

    const widenedRoles = ROLE_MASK.RAW_OR_TRANSLATOR;

    await updateMemberInvitationRoles(ctx.sadmin, guestInv.id, widenedRoles);

    // B2.2: re-list and verify guest_01 roles changed.
    const pendingAfterUpdate = await listMemberInvitations(ctx.sadmin, teamId, true);

    const guestInvAfter = pendingAfterUpdate.find((item) => item.id === guestInv.id);

    assert.assertEquals(guestInvAfter?.roles, widenedRoles);

    // Update the persona matrix so registration uses the widened roles.
    const guestPersona = personas.find((persona) => persona.persona === "guest_01");

    assert.assert(guestPersona);
    guestPersona.roles = widenedRoles;

    // B2.3: path id / body id mismatch -> 422 code 7.
    expectError(
        await ctx.sadmin.put<ErrorBody>(
            `/api/v1/member-invitations/${guestInv.id}/roles`,
            { id: "not-the-path-id", roles: ROLE.RAW_PROVIDER },
        ),
        422,
        7,
    );

    // B2.4: create a throwaway invitation then delete it.
    const cancelledQid = qid("cancelled_01");

    const cancelledInv = await createMemberInvitation(
        ctx.sadmin,
        teamId,
        cancelledQid,
        ROLE.RAW_PROVIDER,
    );

    await deleteMemberInvitation(ctx.sadmin, cancelledInv.id);

    // B2.5: registering with the deleted invitation's code -> 422 code 2.
    {
        const client = new ApiClient(testEnv.apiBaseUrl);

        expectError(
            await client.post<ErrorBody>("/api/v1/auth/register", {
                code: cancelledInv.code,
                nickname: nickname("cancelled_01"),
                password: password("cancelled_01"),
                qid: cancelledQid,
            }),
            422,
            2,
        );
    }

    // B2.6: deleting the same invitation a second time -> 404 (not found).
    // The repo maps a missing row to Expected::Args (422 code 2). Assert 422.
    expectError(
        await ctx.sadmin.delete<ErrorBody>(`/api/v1/member-invitations/${cancelledInv.id}`),
        422,
        2,
    );

    // ---------- B3. 14 people register and join ----------

    for (const persona of personas) {
        const inv = invitations.get(persona.persona);

        assert.assert(inv, `invitation for ${persona.persona} must exist`);

        const { api, userId } = await registerInvitee(
            inv.code,
            persona.qid,
            nickname(persona.persona),
            password(persona.persona),
        );

        // B3.2: /users/me returns the new user with is_sadmin=false.
        const me = await getMyInfo(api);

        assert.assertEquals(me.id, userId);
        assert.assertEquals(me.qid, persona.qid);
        assert.assertEquals(me.nickname, nickname(persona.persona));
        assert.assertEquals(me.is_sadmin, false);

        // B3.3 + B3.4: /members/me returns exactly one default-team member
        // with roles equal to the (possibly widened) invitation roles.
        const myMembers = await listMyMembers(api);

        assert.assertEquals(myMembers.length, 1);

        const member = myMembers[0];

        assert.assert(member);
        assert.assertEquals(member?.team_id, teamId);
        assert.assertEquals(member?.roles, persona.roles);

        const userClient: UserClient = {
            persona: persona.persona,
            api,
            userId,
            qid: persona.qid,
            memberIds: { [teamId]: member.id },
            roles: persona.roles,
        };

        ctx.users.set(persona.persona, userClient);
    }

    // B3.5: re-registering with an already-consumed code -> 422 code 2.
    {
        const firstPersona = personas[0]!;
        const inv = invitations.get(firstPersona.persona)!;
        const client = new ApiClient(testEnv.apiBaseUrl);

        expectError(
            await client.post<ErrorBody>("/api/v1/auth/register", {
                code: inv.code,
                nickname: nickname(firstPersona.persona) + "_dup",
                password: password(firstPersona.persona),
                qid: firstPersona.qid,
            }),
            422,
            2,
        );
    }

    // B3.6: trans_01's code paired with trans_02's qid -> 422 code 2.
    {
        const trans01Inv = invitations.get("trans_01")!;
        const trans02 = personas.find((persona) => persona.persona === "trans_02")!;
        const client = new ApiClient(testEnv.apiBaseUrl);

        expectError(
            await client.post<ErrorBody>("/api/v1/auth/register", {
                code: trans01Inv.code,
                nickname: nickname("trans_02_wrong"),
                password: password("trans_02"),
                qid: trans02.qid,
            }),
            422,
            2,
        );
    }

    // B3.7: pending list no longer contains the 14 consumed invitations.
    const pendingAfterRegister = await listMemberInvitations(ctx.sadmin, teamId, true);

    for (const persona of personas) {
        const inv = invitations.get(persona.persona)!;
        const stillPending = pendingAfterRegister.find((item) => item.id === inv.id);

        assert.assert(!stillPending, `consumed invitation for ${persona.persona} must not be pending`);
    }

    // B3.8: consumed list (pending=false) contains all 14.
    const consumedList = await listMemberInvitations(ctx.sadmin, teamId, false);

    for (const persona of personas) {
        const inv = invitations.get(persona.persona)!;
        const consumed = consumedList.find((item) => item.id === inv.id);

        assert.assert(consumed, `consumed list must include invitation for ${persona.persona}`);
        assert.assertEquals(consumed?.is_pending, false);
    }

    // ---------- B4. member list filters and bad params ----------

    // B4.1: team mode returns 15 members (sadmin + 14).
    const teamMembers = await listTeamMembers(ctx.sadmin, teamId, "&incl=user");

    assert.assertEquals(teamMembers.length, 15);
    assertMemberListWellFormed(teamMembers);

    // B4.3: incl=user embeds user with matching id.
    for (const member of teamMembers) {
        assert.assert(member.user, "incl=user must embed the user");
        assert.assertEquals(member.user?.id, member.user_id);
    }

    // B4.2: translator role filter returns only members whose roles contain
    // the translator bit. After B2, guest_01 also has translator.
    const translatorMembers = await listTeamMembers(
        ctx.sadmin,
        teamId,
        `&role=${ROLE.TRANSLATOR}`,
    );

    for (const member of translatorMembers) {
        assert.assert(
            (member.roles & ROLE.TRANSLATOR) !== 0,
            `role=TRANSLATOR filter returned member without translator bit: ${member.user_id}`,
        );
    }

    // Every translator persona is present in the filter result.
    for (const persona of personas) {
        if ((persona.roles & ROLE.TRANSLATOR) !== 0) {
            const userClient = ctx.users.get(persona.persona);

            assert.assert(userClient, `${persona.persona} must be registered`);

            const found = translatorMembers.find((member) => member.user_id === userClient.userId);

            assert.assert(found, `translator filter must include ${persona.persona}`);
        }
    }

    // B4.3: fuzzy_nickname=trans returns only members whose nickname contains
    // "trans" (the prefix is unique per run, so match on the persona segment).
    const fuzzyTrans = await listTeamMembers(
        ctx.sadmin,
        teamId,
        `&fuzzy_nickname=${encodeURIComponent(runPrefix + "trans")}`,
    );

    for (const member of fuzzyTrans) {
        assert.assert(
            member.nickname.includes(runPrefix + "trans"),
            `fuzzy_nickname=trans returned non-matching nickname: ${member.nickname}`,
        );
    }

    // B4.4: owner mode returns trans_01's default-team membership.
    const trans01 = ctx.users.get("trans_01");

    assert.assert(trans01);

    const ownerMembers = await listMyMembers(trans01.api);

    const trans01Default = ownerMembers.find((member) => member.team_id === teamId);

    assert.assert(trans01Default, "owner mode must return trans_01's default-team membership");
    assert.assertEquals(trans01Default?.user_id, trans01.userId);

    // B4.5: both team_id and owner_id -> 422 code 2.
    expectError(
        await ctx.sadmin.get<ErrorBody>(
            `/api/v1/members?team_id=${teamId}&owner_id=${trans01.userId}&offset=0&limit=20`,
        ),
        422,
        2,
    );

    // B4.6: owner mode + role -> 422 code 2.
    expectError(
        await ctx.sadmin.get<ErrorBody>(
            `/api/v1/members?owner_id=${trans01.userId}&role=${ROLE.TRANSLATOR}&offset=0&limit=20`,
        ),
        422,
        2,
    );

    // B4.7: composite role filter (TRANSLATOR | PROOFREADER = 6) -> 400.
    // `role` is `Option<RoleField>`; a multi-bit value is rejected by the
    // axum query extractor with a 400 (Failed to deserialize query string),
    // NOT a 422. Assert status 400.
    expectStatus(
        await ctx.sadmin.get<ErrorBody>(
            `/api/v1/members?team_id=${teamId}&role=${ROLE_MASK.TRANS_PROOF}&offset=0&limit=20`,
        ),
        400,
    );

    // ---------- B5. member role update + perm boundary ----------

    // B5.1: sadmin widens guest_01's member roles to RAW | TRANSLATOR | PROOFREADER.
    const guest01 = ctx.users.get("guest_01");

    assert.assert(guest01);

    const guest01MemberId = guest01.memberIds[teamId];

    assert.assert(guest01MemberId, "guest_01 must have a default-team member id");

    const guestWideRoles = ROLE_MASK.RAW_TRANS_PROOF;

    await updateMemberRoles(ctx.sadmin, guest01MemberId, guestWideRoles);

    // Re-list and verify.
    const afterGuestUpdate = await listTeamMembers(ctx.sadmin, teamId);

    const guest01After = afterGuestUpdate.find((member) => member.id === guest01MemberId);

    assert.assertEquals(guest01After?.roles, guestWideRoles);

    // Update the UserClient so later modules see the widened roles.
    guest01.roles = guestWideRoles;

    // B5.2: non-admin (trans_01) modifying proof_01's roles -> 403 code 4.
    const proof01 = ctx.users.get("proof_01");

    assert.assert(proof01);

    const proof01MemberId = proof01.memberIds[teamId];

    assert.assert(proof01MemberId);

    expectError(
        await trans01.api.put<ErrorBody>(
            `/api/v1/members/${proof01MemberId}/roles`,
            { id: proof01MemberId, roles: ROLE.PROOFREADER },
        ),
        403,
        4,
    );

    // B5.3: path/body id mismatch -> 422 code 7.
    expectError(
        await ctx.sadmin.put<ErrorBody>(`/api/v1/members/${proof01MemberId}/roles`, {
            id: "not-the-path-id",
            roles: ROLE.PROOFREADER,
        }),
        422,
        7,
    );

    // B5.4: non-admin (guest_01) deleting trans_01 member -> 403 code 4.
    const trans01MemberId = trans01.memberIds[teamId];

    assert.assert(trans01MemberId);

    expectError(
        await guest01.api.delete<ErrorBody>(`/api/v1/members/${trans01MemberId}`),
        403,
        4,
    );

    // B5.5: sadmin does NOT delete any of the core 14 members here — they are
    // needed by every downstream module. The delete-perm + cascade
    // behaviour is covered by it_10 against a throwaway member.
    //
    // Verify the team still has 15 members after the perm-negative cases.
    const finalMembers = await listTeamMembers(ctx.sadmin, teamId);

    assert.assertEquals(finalMembers.length, 15);
}
