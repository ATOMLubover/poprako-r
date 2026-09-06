// it_04 — Assignment join, assignment invitation, role update/delete.
//
// Preconditions:
//   - it_00 + it_01 + it_02 have run. `ctx.main` exists (chapterId set,
//     no pages required for join).
//
// Postconditions:
//   - `ctx.main.assignmentIds` populated for trans_01, trans_02, proof_01,
//     type_01, review_01, publish_01, trans_03 (via invitation). trans_03
//     later deleted in E3, so final assignmentIds: trans_01, trans_02,
//     proof_01, type_01, review_01, publish_01.
//
// Covers test-plan: E1, E2, E3.
//
// Grounded perm pins:
//   - assignment join: caller's MEMBER roles must contain the requested
//     assignment roles (`member.roles.contains_mask(assignment.roles)`).
//     ADMIN bit in join roles -> 422/2 (`error-chapter-role-not-assignable`).
//     Non-ADMIN bit not in member roles -> 403/4.
//   - duplicate join: upsert (merge_roles), NOT an error. `(chapter_id,
//     user_id)` stays unique.
//   - assignment invitation create/list: chapter ADMIN only. sadmin has it
//     from chapter create. trans_01 -> 403/4.
//   - invitation create for already-assigned invitee -> 422/2
//     (`error-assignment-already-exists`).
//   - invitation join with wrong user (qid mismatch) -> 422/2
//     (`error-no-pending-invitation`).
//   - invitation join with non-existent code -> 422/2 (not found).
//   - assignment roles update: chapter admin OR self-reduce (caller == target
//     and new roles subset of existing). Non-admin updating another -> 403/4.
//   - assignment delete: self OR chapter admin.
//
// Status: IMPLEMENTED.

import * as assert from "@std/assert";

import { expectError, expectStatus } from "../http/assertions.ts";
import type { ErrorBody } from "../http/apiClient.ts";
import { assertChapterInvariant } from "../http/invariants.ts";
import {
    createAssignmentInvitation,
    deleteAssignment,
    deleteAssignmentInvitation,
    getComic,
    joinAssignmentInvitation,
    joinChapterAssignment,
    listChapterAssignmentInvitations,
    listChapterAssignments,
    listOwnerAssignments,
    updateAssignmentRoles,
} from "../http/fixtures.ts";
import { ROLE, ROLE_MASK } from "../state/roles.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

export async function runIt04Module(ctx: RunCtx): Promise<void> {
    assert.assert(ctx.main, "it_02 must have set ctx.main");

    const mainChapterId = ctx.main.chapterId;
    const trans01 = ctx.users.get("trans_01");
    const trans02 = ctx.users.get("trans_02");
    const trans03 = ctx.users.get("trans_03");
    const proof01 = ctx.users.get("proof_01");
    const proof02 = ctx.users.get("proof_02");
    const type01 = ctx.users.get("type_01");
    const review01 = ctx.users.get("review_01");
    const publish01 = ctx.users.get("publish_01");
    const guest01 = ctx.users.get("guest_01");

    assert.assert(
        trans01 && trans02 && trans03 && proof01 && proof02 && type01 && review01 && publish01 && guest01,
        "it_01 must have registered all 14 personas",
    );

    // ---------- E1. direct join ----------

    const joinSpecs: Array<{ user: typeof trans01; roles: number; label: string }> = [
        { user: trans01, roles: ROLE.TRANSLATOR, label: "trans_01" },
        { user: trans02, roles: ROLE.TRANSLATOR, label: "trans_02" },
        { user: proof01, roles: ROLE.PROOFREADER, label: "proof_01" },
        { user: type01, roles: ROLE.TYPESETTER, label: "type_01" },
        { user: review01, roles: ROLE.REVIEWER, label: "review_01" },
        { user: publish01, roles: ROLE.PUBLISHER, label: "publish_01" },
    ];

    for (const spec of joinSpecs) {
        const assignment = await joinChapterAssignment(spec.user.api, mainChapterId, spec.roles);

        assert.assertEquals(assignment.chapter_id, mainChapterId);
        assert.assertEquals(assignment.user_id, spec.user.userId);
        assert.assert((assignment.roles & spec.roles) !== 0, `${spec.label} roles must contain requested bit`);

        ctx.main.assignmentIds[spec.label] = assignment.id;
    }

    // E1.3: duplicate join is an upsert (merge_roles), not an error.
    // Re-list and assert (chapter_id, user_id) uniqueness.
    const dupJoin = await joinChapterAssignment(trans01.api, mainChapterId, ROLE.TRANSLATOR);

    assert.assertEquals(dupJoin.user_id, trans01.userId);

    const assignments = await listChapterAssignments(ctx.sadmin, mainChapterId, "&incl=user");

    const userKeys = new Set<string>();

    for (const a of assignments) {
        const key = `${a.chapter_id}:${a.user_id}`;

        assert.assert(!userKeys.has(key), `duplicate assignment key ${key}`);
        userKeys.add(key);

        assert.assert(a.user, "incl=user embeds user");
        assert.assertEquals(a.user?.id, a.user_id);
    }

    // E1.4: guest_01 (member roles RAW|TRANSLATOR|PROOFREADER) joining with
    // PUBLISHER (not in member roles) -> 403/4.
    expectError(
        await guest01.api.post<ErrorBody>("/api/v1/assignments/join", {
            chapter_id: mainChapterId,
            roles: ROLE.PUBLISHER,
        }),
        403,
        4,
    );

    // E1.4b: joining with ADMIN bit -> 422/2 (not assignable).
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/assignments/join", {
            chapter_id: mainChapterId,
            roles: ROLE.ADMIN,
        }),
        422,
        2,
    );

    // E1.4c: role update preserves the assignment while allowing the caller
    // to leave one of their roles.
    const guestAssignment = await joinChapterAssignment(
        guest01.api,
        mainChapterId,
        ROLE_MASK.TRANS_PROOF,
    );

    await updateAssignmentRoles(
        guest01.api,
        mainChapterId,
        guest01.userId,
        ROLE.PROOFREADER,
    );

    const guestAssignments = await listOwnerAssignments(
        guest01.api,
        guest01.userId,
    );

    const guestAssignmentAfterRoleExit = guestAssignments.find(
        (assignment) => assignment.id === guestAssignment.id,
    );

    assert.assert(guestAssignmentAfterRoleExit, "guest assignment must remain");
    assert.assertEquals(guestAssignmentAfterRoleExit.roles, ROLE.PROOFREADER);

    await deleteAssignment(guest01.api, guestAssignment.id);

    // E1.5: join with non-existent chapter_id -> 422/2.
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/assignments/join", {
            chapter_id: "chapter-does-not-exist",
            roles: ROLE.TRANSLATOR,
        }),
        422,
        2,
    );

    // E1.7: owner-mode list with incl=chapter.comic.workset.team, id chain.
    const ownerAssignments = await listOwnerAssignments(
        trans01.api,
        trans01.userId,
        "&incl=chapter.comic.workset.team",
    );

    const trans01MainAssignment = ownerAssignments.find(
        (a) => a.chapter_id === mainChapterId,
    );

    assert.assert(trans01MainAssignment, "trans_01 owner list must include main chapter assignment");
    assert.assert(trans01MainAssignment?.chapter, "chapter embedded");
    assert.assertEquals(trans01MainAssignment?.chapter?.id, mainChapterId);
    assert.assert(trans01MainAssignment?.chapter?.comic, "comic embedded");
    assert.assertEquals(trans01MainAssignment?.chapter?.comic?.id, ctx.main.comicId);
    assert.assert(trans01MainAssignment?.chapter?.comic?.workset, "workset embedded");
    assert.assertEquals(trans01MainAssignment?.chapter?.comic?.workset?.id, ctx.main.worksetId);

    const directComic = await getComic(ctx.sadmin, ctx.main.comicId);

    assert.assertEquals(
        trans01MainAssignment?.chapter?.comic?.cover_url,
        directComic.cover_url,
        "assignment nested comic must preserve the page-image cover fallback",
    );
    assert.assertEquals(
        trans01MainAssignment?.chapter?.comic?.cover_thumbnail_url,
        directComic.cover_thumbnail_url,
        "assignment nested comic must preserve the fallback thumbnail",
    );

    // E1.8: role=TRANSLATOR filter returns only translator-bit assignments.
    const translatorAssignments = await listChapterAssignments(
        ctx.sadmin,
        mainChapterId,
        `&role=${ROLE.TRANSLATOR}`,
    );

    for (const a of translatorAssignments) {
        assert.assert((a.roles & ROLE.TRANSLATOR) !== 0, "role=TRANSLATOR filter must return only translator-bit");
    }

    // E1.9: composite role filter -> 400 (axum query deserialization rejection)
    expectStatus(
        await ctx.sadmin.get<ErrorBody>(
            `/api/v1/assignments?chapter_id=${mainChapterId}&role=${ROLE_MASK.TRANS_PROOF}&offset=0&limit=20`,
        ),
        400,
    );

    // E1.10: both chapter_id and owner_id -> 422/2.
    expectError(
        await ctx.sadmin.get<ErrorBody>(
            `/api/v1/assignments?chapter_id=${mainChapterId}&owner_id=${trans01.userId}&offset=0&limit=20`,
        ),
        422,
        2,
    );

    // E1.11: neither chapter_id nor owner_id -> 422/2.
    expectError(
        await ctx.sadmin.get<ErrorBody>("/api/v1/assignments?offset=0&limit=20"),
        422,
        2,
    );

    // ---------- E2. admin creates assignment invitation ----------

    // E2.1: sadmin creates translator invitation for trans_03.
    const trans03Inv = await createAssignmentInvitation(
        ctx.sadmin,
        mainChapterId,
        trans03.qid,
        ROLE.TRANSLATOR,
    );

    assert.assert(trans03Inv.id);
    assert.assert(trans03Inv.code);

    // E2.2: pending list includes it.
    const pendingBefore = await listChapterAssignmentInvitations(ctx.sadmin, mainChapterId, true);

    const foundTrans03 = pendingBefore.find((inv) => inv.id === trans03Inv.id);

    assert.assert(foundTrans03, "pending list must include trans_03 invitation");
    assert.assertEquals(foundTrans03?.invitee_qid, trans03.qid);
    assert.assertEquals(foundTrans03?.roles, ROLE.TRANSLATOR);
    assert.assertEquals(foundTrans03?.is_pending, true);

    // E2.3: trans_03 consumes -> 201, assignment roles == TRANSLATOR.
    const trans03Assignment = await joinAssignmentInvitation(trans03.api, trans03Inv.code);

    assert.assertEquals(trans03Assignment.chapter_id, mainChapterId);
    assert.assertEquals(trans03Assignment.user_id, trans03.userId);
    assert.assert((trans03Assignment.roles & ROLE.TRANSLATOR) !== 0);

    ctx.main.assignmentIds["trans_03"] = trans03Assignment.id;

    // E2.4: pending list no longer includes it; consumed list includes it.
    const pendingAfter = await listChapterAssignmentInvitations(ctx.sadmin, mainChapterId, true);

    assert.assert(!pendingAfter.find((inv) => inv.id === trans03Inv.id), "consumed invitation must not be pending");

    const consumedList = await listChapterAssignmentInvitations(ctx.sadmin, mainChapterId, false);

    assert.assert(
        consumedList.find((inv) => inv.id === trans03Inv.id),
        "consumed list must include trans_03 invitation",
    );

    // E2.5: sadmin creates proofread invitation for proof_02; trans_01 tries
    // to consume -> 422/2 (qid mismatch).
    const proof02Inv = await createAssignmentInvitation(
        ctx.sadmin,
        mainChapterId,
        proof02.qid,
        ROLE.PROOFREADER,
    );

    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/assignment-invitations/join", {
            code: proof02Inv.code,
        }),
        422,
        2,
    );

    // E2.6: non-existent code join -> 422/2.
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/assignment-invitations/join", {
            code: "nonexistent-code",
        }),
        422,
        2,
    );

    // E2.7: sadmin creates invitation for already-assigned trans_01 -> 422/2
    // (`error-assignment-already-exists`).
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/assignment-invitations", {
            chapter_id: mainChapterId,
            invitee_qid: trans01.qid,
            roles: ROLE.TRANSLATOR,
        }),
        422,
        2,
    );

    // E2.8: non-admin (trans_01) creates invitation -> 403/4.
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/assignment-invitations", {
            chapter_id: mainChapterId,
            invitee_qid: proof02.qid,
            roles: ROLE.PROOFREADER,
        }),
        403,
        4,
    );

    // E2.9: delete pending invitation (proof_02's) -> 204; its code can no
    // longer be joined (422/2).
    await deleteAssignmentInvitation(ctx.sadmin, proof02Inv.id);

    expectError(
        await proof02.api.post<ErrorBody>("/api/v1/assignment-invitations/join", {
            code: proof02Inv.code,
        }),
        422,
        2,
    );

    // proof_02 is NOT assigned (invitation deleted before consume). Join proof_02
    // directly via /assignments/join so it_05 has a proofreader.
    const proof02Assignment = await joinChapterAssignment(proof02.api, mainChapterId, ROLE.PROOFREADER);

    ctx.main.assignmentIds["proof_02"] = proof02Assignment.id;

    // Also join raw_01 and raw_02 as RAW_PROVIDER so it_05/F-parallel has them
    // for anchored inserts (page reserve needs RAW_PROVIDER; unit save needs
    // TRANSLATOR/PROOFREADER — raw_01/02 will NOT be able to save units, but
    // they can still be assigned). For F8 anchored inserts, the inserter must
    // be a translator/proofreader. So also add TRANSLATOR to raw_01/02 via
    // self-join merge (member roles of raw_01 = RAW only -> cannot take
    // TRANSLATOR). So raw_01/02 CANNOT save units. it_05 will use trans_* for
    // saves. raw_01/02 assignment is still useful for perm negatives.
    const raw01 = ctx.users.get("raw_01");
    const raw02 = ctx.users.get("raw_02");

    assert.assert(raw01 && raw02);

    const raw01Assignment = await joinChapterAssignment(raw01.api, mainChapterId, ROLE.RAW_PROVIDER);

    const raw02Assignment = await joinChapterAssignment(raw02.api, mainChapterId, ROLE.RAW_PROVIDER);

    ctx.main.assignmentIds["raw_01"] = raw01Assignment.id;
    ctx.main.assignmentIds["raw_02"] = raw02Assignment.id;

    // ---------- E3. update + delete assignment ----------

    // E3.1: sadmin updates trans_03's assignment roles (trans_03's member
    // role is TRANSLATOR only, cannot widen to TRANSLATOR|PROOFREADER — keep
    // TRANSLATOR only).
    await updateAssignmentRoles(ctx.sadmin, mainChapterId, trans03.userId, ROLE.TRANSLATOR);

    const afterUpdate = await listChapterAssignments(ctx.sadmin, mainChapterId);

    const trans03After = afterUpdate.find((a) => a.id === trans03Assignment.id);

    assert.assert(trans03After, "trans_03 assignment must exist");
    assert.assert((trans03After!.roles & ROLE.TRANSLATOR) !== 0, "trans_03 roles must contain TRANSLATOR");

    // E3.2: path/body chapter_id mismatch -> 422 code 7.
    expectError(
        await ctx.sadmin.put<ErrorBody>(
            `/api/v1/chapters/${mainChapterId}/assignments/${trans03.userId}/roles`,
            { chapter_id: "not-the-path-id", roles: ROLE.TRANSLATOR, user_id: trans03.userId },
        ),
        422,
        7,
    );

    // E3.3: path/body user_id mismatch -> 422 code 7.
    expectError(
        await ctx.sadmin.put<ErrorBody>(
            `/api/v1/chapters/${mainChapterId}/assignments/${trans03.userId}/roles`,
            { chapter_id: mainChapterId, roles: ROLE.TRANSLATOR, user_id: "not-the-path-id" },
        ),
        422,
        7,
    );

    // E3.4: non-admin (trans_01) updating proof_01's assignment -> 403/4.
    expectError(
        await trans01.api.put<ErrorBody>(
            `/api/v1/chapters/${mainChapterId}/assignments/${proof01.userId}/roles`,
            { chapter_id: mainChapterId, roles: ROLE.PROOFREADER, user_id: proof01.userId },
        ),
        403,
        4,
    );

    // E3.5: sadmin deletes trans_03's assignment -> 204.
    await deleteAssignment(ctx.sadmin, trans03Assignment.id);

    // E3.6: chapter assignment list no longer includes trans_03.
    const afterDelete = await listChapterAssignments(ctx.sadmin, mainChapterId);

    assert.assert(!afterDelete.find((a) => a.id === trans03Assignment.id), "trans_03 assignment must be gone");

    // E3.7: trans_03 owner list no longer includes this chapter.
    const trans03OwnerAfter = await listOwnerAssignments(trans03.api, trans03.userId);

    assert.assert(
        !trans03OwnerAfter.find((a) => a.chapter_id === mainChapterId),
        "trans_03 owner list must not include main chapter after deletion",
    );

    // E3.8: delete same assignment id again -> 422/2.
    expectError(
        await ctx.sadmin.delete<ErrorBody>(`/api/v1/assignments/${trans03Assignment.id}`),
        422,
        2,
    );

    // E3.9: deleted-assignment user (trans_03, no longer assigned) tries to
    // save a unit on a main page -> 403/4. Requires it_03 to have reserved
    // pages (ctx.main.pageIds non-empty).
    if (ctx.main.pageIds.length > 0) {
        const aMainPageId = ctx.main.pageIds[0]!;

        expectError(
            await trans03.api.post<ErrorBody>(
                `/api/v1/pages/${aMainPageId}/units/save`,
                [
                    {
                        edit: "create",
                        local_id: "should-fail",
                        is_bubble: true,
                        coord: {
                            x_coord: 0.1,
                            y_coord: 0.1,
                        },
                    },
                ],
            ),
            403,
            4,
        );
    }

    // Remove trans_03 from assignmentIds (deleted).
    delete ctx.main.assignmentIds["trans_03"];

    await assertChapterInvariant(ctx.sadmin, mainChapterId);
}
