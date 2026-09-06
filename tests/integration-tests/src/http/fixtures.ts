import * as assert from "@std/assert";

import { testEnv } from "../config/env.ts";
import { expectError, expectNoContent, expectSuccessData, expectSuccessList } from "./assertions.ts";
import { ApiClient, clientFor } from "./apiClient.ts";
import type { ErrorBody, SuccessBody } from "./apiClient.ts";
import type {
    AnnouncementInfoView,
    ArchiveComicVal,
    AssignmentInfoView,
    AssignmentInvitationInfoView,
    ChapterInfoView,
    ChapterTranslationPortView,
    ChapterWorkflowRecordInfoView,
    CodeVal,
    ComicInfoView,
    CommentInfoView,
    CreateComicVal,
    ExportChapterTranslationsVal,
    IdVal,
    ImageExtension,
    ListComicInfosVal,
    ListEdittedDiffPageIdsVal,
    ListPageUnitInfosVal,
    LoginVal,
    MemberInfoView,
    MemberInvitationInfoView,
    PageImageInput,
    PageInfoView,
    ReserveChapterPagesVal,
    ReservedPageVal,
    ReserveImageVal,
    SystemMailInfoView,
    TeamInfoView,
    UnitInfoView,
    UnitTextPart,
    UnitTransformInput,
    UserInfoView,
    WorksetInfoView,
} from "./types.ts";
import { chapterStageInstr } from "../state/stages.ts";
import type { StageName, StageOper } from "../state/stages.ts";

// ---------- timestamp / invariant helpers ----------

// Assert a value is a Unix-millisecond integer (the API's timestamp format).
export function assertTimestampMs(value: unknown): asserts value is number {
    assert.assert(typeof value === "number", "timestamp must be a number");
    assert.assert(Number.isInteger(value), "timestamp must be an integer");
}

// Assert `created_at <= updated_at` for a record carrying both fields.
export function assertCreatedBeforeUpdated(created: number, updated: number): void {
    assert.assert(
        created <= updated,
        `created_at (${created}) must be <= updated_at (${updated})`,
    );
}

// ---------- auth ----------

export async function login(
    api: ApiClient,
    qid: string,
    password: string,
): Promise<LoginVal> {
    const response = await api.post<SuccessBody<LoginVal>>("/api/v1/auth/login", {
        password,
        qid,
    });

    const val = expectSuccessData(response, 200);

    assert.assert(val.token.length > 20, "login token must be > 20 chars");

    api.setToken(val.token);

    return val;
}

// Register an invitee via a fresh client and return the authenticated client
// plus the new user id. The caller's `api` is NOT mutated.
export async function registerInvitee(
    code: string,
    qidValue: string,
    nicknameValue: string,
    passwordValue: string,
): Promise<{ api: ApiClient; userId: string; token: string }> {
    const fresh = new ApiClient(testEnv.apiBaseUrl);

    const response = await fresh.post<SuccessBody<LoginVal>>("/api/v1/auth/register", {
        code,
        nickname: nicknameValue,
        password: passwordValue,
        qid: qidValue,
    });

    const val = expectSuccessData(response, 201);

    assert.assert(val.token.length > 20, "register token must be > 20 chars");

    fresh.setToken(val.token);

    return { api: fresh, userId: val.user_id, token: val.token };
}

// Build an authenticated client for an already-registered user.
export async function userClient(qidValue: string, passwordValue: string): Promise<{
    api: ApiClient;
    userId: string;
    token: string;
}> {
    const fresh = new ApiClient(testEnv.apiBaseUrl);

    const { user_id, token } = await login(fresh, qidValue, passwordValue);

    return { api: fresh, userId: user_id, token };
}

export async function logout(api: ApiClient): Promise<void> {
    expectNoContent(await api.post<null>("/api/v1/auth/logout"));
    api.clearToken();
}

// ---------- user ----------

export async function getMyInfo(api: ApiClient): Promise<UserInfoView> {
    return expectSuccessData(await api.get("/api/v1/users/me"), 200);
}

export async function getUserInfo(api: ApiClient, userId: string): Promise<UserInfoView> {
    return expectSuccessData(await api.get(`/api/v1/users/${userId}`), 200);
}

// ---------- team ----------

export async function createTeam(
    api: ApiClient,
    name: string,
    description: string,
): Promise<TeamInfoView> {
    return expectSuccessData(
        await api.post<SuccessBody<TeamInfoView>>("/api/v1/teams", {
            description,
            name,
        }),
        201,
    );
}

export async function getTeam(api: ApiClient, teamId: string): Promise<TeamInfoView> {
    return expectSuccessData(await api.get(`/api/v1/teams/${teamId}`), 200);
}

export async function listTeams(api: ApiClient, userId?: string): Promise<TeamInfoView[]> {
    const query = userId ? `?user_id=${encodeURIComponent(userId)}&offset=0&limit=50` : "?offset=0&limit=50";

    return expectSuccessList(await api.get<SuccessBody<TeamInfoView[]>>(`/api/v1/teams${query}`), 200);
}

export async function updateTeam(
    api: ApiClient,
    teamId: string,
    name: string,
    description: string,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/teams/${teamId}`, {
            description,
            id: teamId,
            name,
        }),
    );
}

export function reserveTeamAvatar(
    api: ApiClient,
    teamId: string,
    ext: ImageExtension,
): Promise<ReserveImageVal> {
    return reserveAndUploadImage(
        api,
        `/api/v1/teams/${teamId}/avatar/alloc`,
        `poprako-team-avatar-${teamId}-${ext}`,
        ext,
    );
}

async function reserveAndUploadImage(
    api: ApiClient,
    path: string,
    content: string,
    ext: ImageExtension,
): Promise<ReserveImageVal> {
    const imageBytes = new TextEncoder().encode(content);

    const imageHash = await sha256Base64(imageBytes);

    const reserved = expectSuccessData(
        await api.post<SuccessBody<ReserveImageVal>>(path, {
            image_hash: imageHash,
            new_byte_len: imageBytes.byteLength,
            ext,
        }),
        200,
    );

    if (reserved.slot) {
        const response = await fetch(reserved.slot.put_url, {
            method: "PUT",
            headers: reserved.slot.headers,
            body: imageBytes,
        });

        assert.assert(response.ok, `image upload failed with status ${response.status}`);
    }

    return reserved;
}

export async function markTeamAvatarUploaded(
    api: ApiClient,
    teamId: string,
    avatarVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/teams/${teamId}/avatar/mark-uploaded`, {
            image_version: avatarVersion,
        }),
    );
}

// ---------- user avatar ----------

export function reserveUserAvatar(
    api: ApiClient,
    userId: string,
    ext: ImageExtension,
): Promise<ReserveImageVal> {
    return reserveAndUploadImage(
        api,
        `/api/v1/users/${userId}/avatar/alloc`,
        `poprako-user-avatar-${userId}-${ext}`,
        ext,
    );
}

export async function markUserAvatarUploaded(
    api: ApiClient,
    userId: string,
    avatarVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/users/${userId}/avatar/mark-uploaded`, {
            image_version: avatarVersion,
        }),
    );
}

// ---------- member ----------

export async function listMyMembers(api: ApiClient, incl = ""): Promise<MemberInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<MemberInfoView[]>>(`/api/v1/members/me?offset=0&limit=50${incl}`),
        200,
    );
}

export async function listTeamMembers(
    api: ApiClient,
    teamId: string,
    extraQuery = "",
): Promise<MemberInfoView[]> {
    const query = `?team_id=${encodeURIComponent(teamId)}&offset=0&limit=50${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<MemberInfoView[]>>(`/api/v1/members${query}`),
        200,
    );
}

export async function updateMemberRoles(
    api: ApiClient,
    memberId: string,
    roles: number,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/members/${memberId}/roles`, {
            id: memberId,
            roles,
        }),
    );
}

// ---------- member invitation ----------

export async function createMemberInvitation(
    api: ApiClient,
    teamId: string,
    inviteeQid: string,
    roles: number,
): Promise<CodeVal> {
    return expectSuccessData(
        await api.post<SuccessBody<CodeVal>>("/api/v1/member-invitations", {
            invitee_qid: inviteeQid,
            roles,
            team_id: teamId,
        }),
        201,
    );
}

export async function listMemberInvitations(
    api: ApiClient,
    teamId: string,
    isPending: boolean,
): Promise<MemberInvitationInfoView[]> {
    const query = `?is_pending=${isPending}&offset=0&limit=100`;

    return expectSuccessList(
        await api.get<SuccessBody<MemberInvitationInfoView[]>>(
            `/api/v1/teams/${teamId}/member-invitations${query}`,
        ),
        200,
    );
}

export async function updateMemberInvitationRoles(
    api: ApiClient,
    invitationId: string,
    roles: number,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/member-invitations/${invitationId}/roles`, {
            id: invitationId,
            roles,
        }),
    );
}

export async function deleteMemberInvitation(api: ApiClient, invitationId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/member-invitations/${invitationId}`));
}

// ---------- workset ----------

export async function createWorkset(
    api: ApiClient,
    teamId: string,
    name: string,
    description?: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/worksets", {
            description: description ?? null,
            name,
            team_id: teamId,
        }),
        201,
    );
}

export async function getWorkset(api: ApiClient, worksetId: string): Promise<WorksetInfoView> {
    return expectSuccessData(await api.get(`/api/v1/worksets/${worksetId}`), 200);
}

export async function listTeamWorksets(api: ApiClient, teamId: string): Promise<WorksetInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<WorksetInfoView[]>>(`/api/v1/teams/${teamId}/worksets?offset=0&limit=100`),
        200,
    );
}

export async function updateWorkset(
    api: ApiClient,
    worksetId: string,
    name: string,
    description?: string,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/worksets/${worksetId}`, {
            description: description ?? null,
            id: worksetId,
            name,
        }),
    );
}

export async function deleteWorkset(api: ApiClient, worksetId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/worksets/${worksetId}`));
}

// ---------- comic ----------

export async function createComic(
    api: ApiClient,
    worksetId: string,
    title: string,
    author: string,
    firstChapterSubtitle?: string,
): Promise<CreateComicVal> {
    return expectSuccessData(
        await api.post<SuccessBody<CreateComicVal>>("/api/v1/comics", {
            author,
            description: null,
            first_chapter_subtitle: firstChapterSubtitle ?? null,
            title,
            workset_id: worksetId,
        }),
        201,
    );
}

export async function getComic(api: ApiClient, comicId: string): Promise<ComicInfoView> {
    return expectSuccessData(await api.get(`/api/v1/comics/${comicId}`), 200);
}

export async function archiveComic(api: ApiClient, comicId: string): Promise<ArchiveComicVal> {
    return expectSuccessData(
        await api.post<SuccessBody<ArchiveComicVal>>(`/api/v1/comics/${comicId}/archive`),
        201,
    );
}

export async function listWorksetComics(
    api: ApiClient,
    worksetId: string,
    extraQuery = "",
): Promise<ComicInfoView[]> {
    const payload = await listWorksetComicInfos(api, worksetId, extraQuery);

    return payload.comics;
}

export async function listWorksetComicInfos(
    api: ApiClient,
    worksetId: string,
    extraQuery = "",
): Promise<ListComicInfosVal> {
    const query = `?offset=0&limit=100${extraQuery}`;

    return expectSuccessData(
        await api.get<SuccessBody<ListComicInfosVal>>(`/api/v1/worksets/${worksetId}/comics${query}`),
        200,
    );
}

export async function updateComic(
    api: ApiClient,
    comicId: string,
    title: string,
    author: string,
    description?: string,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/comics/${comicId}`, {
            author,
            description: description ?? null,
            id: comicId,
            title,
        }),
    );
}

export function reserveComicCover(
    api: ApiClient,
    comicId: string,
    ext: ImageExtension,
): Promise<ReserveImageVal> {
    return reserveAndUploadImage(
        api,
        `/api/v1/comics/${comicId}/cover/alloc`,
        `poprako-comic-cover-${comicId}-${ext}`,
        ext,
    );
}

export async function markComicCoverUploaded(
    api: ApiClient,
    comicId: string,
    coverVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/comics/${comicId}/cover/mark-uploaded`, {
            image_version: coverVersion,
        }),
    );
}

// ---------- chapter ----------

export async function createChapter(
    api: ApiClient,
    comicId: string,
    subtitle?: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/chapters", {
            comic_id: comicId,
            subtitle: subtitle ?? null,
        }),
        201,
    );
}

export async function getChapter(api: ApiClient, chapterId: string): Promise<ChapterInfoView> {
    return expectSuccessData(await api.get(`/api/v1/chapters/${chapterId}`), 200);
}

export async function listChapterWorkflowRecords(
    api: ApiClient,
    chapterId: string,
    offset = 0,
    limit = 100,
): Promise<ChapterWorkflowRecordInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<ChapterWorkflowRecordInfoView[]>>(
            `/api/v1/chapters/${chapterId}/workflow-records?offset=${offset}&limit=${limit}`,
        ),
        200,
    );
}

export async function listComicChapters(
    api: ApiClient,
    comicId: string,
    extraQuery = "",
): Promise<ChapterInfoView[]> {
    const query = `?offset=0&limit=100${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<ChapterInfoView[]>>(`/api/v1/comics/${comicId}/chapters${query}`),
        200,
    );
}

export async function getPinnedChapter(
    api: ApiClient,
    comicId: string,
): Promise<ChapterInfoView | null> {
    const data = expectSuccessData(
        await api.get<SuccessBody<ChapterInfoView | null>>(`/api/v1/comics/${comicId}/chapters/pinned`),
        200,
    );

    return data;
}

export async function patchChapter(
    api: ApiClient,
    chapterId: string,
    patch: { subtitle?: string },
): Promise<void> {
    const body: Record<string, unknown> = { id: chapterId };

    if (patch.subtitle !== undefined) {
        body.subtitle = patch.subtitle;
    }

    expectNoContent(await api.patch<null>(`/api/v1/chapters/${chapterId}`, body));
}

export async function markChapterPinned(api: ApiClient, chapterId: string): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/chapters/${chapterId}/mark-pinned`),
    );
}

export async function advanceStage(
    api: ApiClient,
    chapterId: string,
    stage: StageName,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/chapters/${chapterId}/stage/advance`, {
            id: chapterId,
            oper: "advance" as StageOper,
            stage: chapterStageInstr(stage),
        }),
    );
}

export async function revertStage(
    api: ApiClient,
    chapterId: string,
    stage: StageName,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/chapters/${chapterId}/stage/advance`, {
            id: chapterId,
            oper: "revert" as StageOper,
            stage: chapterStageInstr(stage),
        }),
    );
}

// ---------- page ----------

const TEST_PAGE_BYTES = new TextEncoder().encode("poprako-page-integration");

const TEST_PAGE_HASH = await sha256Base64(TEST_PAGE_BYTES);

export function newPageManifest(pageCount: number, ext: ImageExtension): PageImageInput[] {
    return Array.from({ length: pageCount }, () => ({
        page_id: null,
        image_hash: TEST_PAGE_HASH,
        new_byte_len: TEST_PAGE_BYTES.byteLength,
        ext,
    }));
}

async function uploadReservedPages(reservedPages: ReservedPageVal[]): Promise<void> {
    for (const reservedPage of reservedPages) {
        if (!reservedPage.slot) continue;

        const response = await fetch(reservedPage.slot.put_url, {
            method: "PUT",
            headers: reservedPage.slot.headers,
            body: TEST_PAGE_BYTES,
        });

        assert.assert(response.ok, `page upload failed with status ${response.status}`);
    }
}

export async function reserveChapterPages(
    api: ApiClient,
    chapterId: string,
    pages: PageImageInput[],
): Promise<ReserveChapterPagesVal> {
    const reserved = expectSuccessData(
        await api.post<SuccessBody<ReserveChapterPagesVal>>(
            `/api/v1/chapters/${chapterId}/pages/alloc`,
            {
                chapter_id: chapterId,
                pages,
            },
        ),
        200,
    );

    await uploadReservedPages(reserved.pages);

    return reserved;
}

export async function listChapterPages(api: ApiClient, chapterId: string): Promise<PageInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<PageInfoView[]>>(`/api/v1/chapters/${chapterId}/pages?offset=0&limit=100`),
        200,
    );
}

export async function reservePageImage(
    api: ApiClient,
    pageId: string,
    ext: ImageExtension,
): Promise<ReservedPageVal> {
    const imageBytes = new TextEncoder().encode(`poprako-page-replacement-${pageId}-${ext}`);

    const imageHash = await sha256Base64(imageBytes);

    const reserved = expectSuccessData(
        await api.post<SuccessBody<ReservedPageVal>>(`/api/v1/pages/${pageId}/image/alloc`, {
            image_hash: imageHash,
            new_byte_len: imageBytes.byteLength,
            ext,
        }),
        200,
    );

    if (reserved.slot) {
        const response = await fetch(reserved.slot.put_url, {
            method: "PUT",
            headers: reserved.slot.headers,
            body: imageBytes,
        });

        assert.assert(response.ok, `page upload failed with status ${response.status}`);
    }

    return reserved;
}

export async function markPageImageUploaded(
    api: ApiClient,
    pageId: string,
    imageVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/pages/${pageId}/image/mark-uploaded`, {
            image_version: imageVersion,
        }),
    );
}

export async function deleteChapterPages(api: ApiClient, chapterId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/chapters/${chapterId}/pages`));
}

// ---------- unit ----------

export interface UnitCoordInput {
    x_coord: number;
    y_coord: number;
}

export interface UnitTranslationInput {
    translated_text: string;
}

export interface UnitRevisionInput {
    is_proofread: boolean;
    proofread_text?: string | null;
}

export type PatchInput<T> =
    | { type: "assign"; value: T }
    | { type: "clear" };

export interface UnitCreateEdit {
    edit: "create";
    local_id: string;
    next_id?: string | null;
    is_bubble: boolean;
    coord: UnitCoordInput;
    translation?: UnitTranslationInput | null;
    revision?: UnitRevisionInput | null;
}

export interface UnitPatchEdit {
    edit: "patch";
    id: string;
    next_id?: PatchInput<string>;
    is_bubble?: boolean | null;
    coord?: UnitCoordInput | null;
    translation?: PatchInput<UnitTranslationInput>;
    revision?: PatchInput<UnitRevisionInput>;
}

export interface UnitDeleteEdit {
    edit: "delete";
    id: string;
}

export type UnitEdit = UnitCreateEdit | UnitPatchEdit | UnitDeleteEdit;

// Build a Create edit for a brand-new bubble Unit with no translation yet.
export function newBubbleUnit(
    localId: string,
    xCoord: number,
    yCoord: number,
): UnitCreateEdit {
    return {
        edit: "create",
        local_id: localId,
        next_id: null,
        is_bubble: true,
        coord: {
            x_coord: xCoord,
            y_coord: yCoord,
        },
    };
}

export interface UnitPatchFixture {
    next_id?: string | null;
    is_bubble?: boolean | null;
    x_coord?: number;
    y_coord?: number;
    translated_text?: string | null;
    proofread_text?: string | null;
    is_proofread?: boolean;
}

export function updateUnit(unitId: string, patch: UnitPatchFixture): UnitPatchEdit {
    const edit: UnitPatchEdit = {
        edit: "patch",
        id: unitId,
    };

    if ("next_id" in patch) {
        edit.next_id = patch.next_id == null ? { type: "clear" } : { type: "assign", value: patch.next_id };
    }

    if ("is_bubble" in patch) {
        edit.is_bubble = patch.is_bubble;
    }

    if ("x_coord" in patch || "y_coord" in patch) {
        edit.coord = {
            x_coord: patch.x_coord ?? 0,
            y_coord: patch.y_coord ?? 0,
        };
    }

    if ("translated_text" in patch) {
        edit.translation = patch.translated_text == null ? { type: "clear" } : {
            type: "assign",
            value: { translated_text: patch.translated_text },
        };
    }

    if ("proofread_text" in patch || patch.is_proofread === true) {
        edit.revision = {
            type: "assign",
            value: {
                is_proofread: patch.is_proofread ?? false,
                proofread_text: patch.proofread_text ?? null,
            },
        };
    }

    return edit;
}

export function deleteUnit(unitId: string): UnitDeleteEdit {
    return { edit: "delete", id: unitId };
}

export async function savePageUnits(
    api: ApiClient,
    pageId: string,
    edits: UnitEdit[],
): Promise<ListPageUnitInfosVal> {
    const maxConflictRetries = 5;

    for (let attempt = 0; attempt <= maxConflictRetries; attempt++) {
        const response = await api.post<ErrorBody>(
            `/api/v1/pages/${pageId}/units/save`,
            edits,
        );

        if (response.status === 409) {
            expectError(response, 409, 8);

            continue;
        }

        expectNoContent(response);

        return listPageUnits(api, pageId);
    }

    throw new Error(`unit save still conflicts after ${maxConflictRetries} client retries`);
}

export async function listPageUnits(api: ApiClient, pageId: string): Promise<ListPageUnitInfosVal> {
    return expectSuccessData(
        await api.get<SuccessBody<ListPageUnitInfosVal>>(`/api/v1/pages/${pageId}/units?offset=0&limit=100`),
        200,
    );
}

export async function listEdittedDiffPageIds(
    api: ApiClient,
    chapterId: string,
): Promise<ListEdittedDiffPageIdsVal> {
    return expectSuccessData(
        await api.get<SuccessBody<ListEdittedDiffPageIdsVal>>(
            `/api/v1/chapters/${chapterId}/pages/editted-diffs`,
        ),
        200,
    );
}

export async function searchChapterUnits(
    api: ApiClient,
    chapterId: string,
    part: UnitTextPart,
    phrase: string,
): Promise<UnitInfoView[]> {
    const query = new URLSearchParams({ part, phrase });

    return expectSuccessData(
        await api.get<SuccessBody<UnitInfoView[]>>(
            `/api/v1/chapters/${chapterId}/units/search?${query.toString()}`,
        ),
        200,
    );
}

export async function transformChapterUnits(
    api: ApiClient,
    chapterId: string,
    part: UnitTextPart,
    units: UnitTransformInput[],
): Promise<void> {
    const response = await api.post<ErrorBody>(
        `/api/v1/chapters/${chapterId}/units/transform`,
        { part, units },
    );

    expectNoContent(response);
}

// ---------- assignment ----------

export async function listChapterAssignments(
    api: ApiClient,
    chapterId: string,
    extraQuery = "",
): Promise<AssignmentInfoView[]> {
    const query = `?chapter_id=${encodeURIComponent(chapterId)}&offset=0&limit=100${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<AssignmentInfoView[]>>(`/api/v1/assignments${query}`),
        200,
    );
}

export async function listOwnerAssignments(
    api: ApiClient,
    ownerId: string,
    extraQuery = "",
): Promise<AssignmentInfoView[]> {
    const query = `?owner_id=${encodeURIComponent(ownerId)}&offset=0&limit=100${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<AssignmentInfoView[]>>(`/api/v1/assignments${query}`),
        200,
    );
}

export async function joinChapterAssignment(
    api: ApiClient,
    chapterId: string,
    roles: number,
): Promise<AssignmentInfoView> {
    return expectSuccessData(
        await api.post<SuccessBody<AssignmentInfoView>>("/api/v1/assignments/join", {
            chapter_id: chapterId,
            roles,
        }),
        201,
    );
}

export async function updateAssignmentRoles(
    api: ApiClient,
    chapterId: string,
    userId: string,
    roles: number,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/chapters/${chapterId}/assignments/${userId}/roles`, {
            chapter_id: chapterId,
            roles,
            user_id: userId,
        }),
    );
}

export async function deleteAssignment(api: ApiClient, assignmentId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/assignments/${assignmentId}`));
}

// ---------- assignment invitation ----------

export async function createAssignmentInvitation(
    api: ApiClient,
    chapterId: string,
    inviteeQid: string,
    roles: number,
): Promise<CodeVal> {
    return expectSuccessData(
        await api.post<SuccessBody<CodeVal>>("/api/v1/assignment-invitations", {
            chapter_id: chapterId,
            invitee_qid: inviteeQid,
            roles,
        }),
        201,
    );
}

export async function listChapterAssignmentInvitations(
    api: ApiClient,
    chapterId: string,
    isPending: boolean,
): Promise<AssignmentInvitationInfoView[]> {
    const query = `?is_pending=${isPending}&offset=0&limit=100`;

    return expectSuccessList(
        await api.get<SuccessBody<AssignmentInvitationInfoView[]>>(
            `/api/v1/chapters/${chapterId}/assignment-invitations${query}`,
        ),
        200,
    );
}

export async function joinAssignmentInvitation(
    api: ApiClient,
    code: string,
): Promise<AssignmentInfoView> {
    return expectSuccessData(
        await api.post<SuccessBody<AssignmentInfoView>>("/api/v1/assignment-invitations/join", {
            code,
        }),
        201,
    );
}

export async function deleteAssignmentInvitation(
    api: ApiClient,
    invitationId: string,
): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/assignment-invitations/${invitationId}`));
}

// ---------- system mail ----------

export async function listSystemMails(
    api: ApiClient,
    extraQuery = "",
): Promise<SystemMailInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<SystemMailInfoView[]>>(`/api/v1/system-mails?offset=0&limit=100${extraQuery}`),
        200,
    );
}

export async function markSystemMailsRead(api: ApiClient, ids: string[]): Promise<void> {
    expectNoContent(
        await api.post<null>("/api/v1/system-mails/mark-read", { ids }),
    );
}

// Poll `listSystemMails` until `predicate(mailCount)` is true, or `timeoutMs`
// elapses. The mail effect runs in a background tokio task, so mails are NOT
// guaranteed to be visible immediately after the workflow op returns 204.
// Throws if the predicate never holds within the timeout.
export async function waitForMails(
    api: ApiClient,
    predicate: (count: number) => boolean,
    timeoutMs = 2000,
): Promise<number> {
    const start = Date.now();

    while (true) {
        const mails = await listSystemMails(api);

        if (predicate(mails.length)) {
            return mails.length;
        }

        if (Date.now() - start >= timeoutMs) {
            throw new Error(
                `waitForMails timed out after ${timeoutMs}ms; last count=${mails.length}`,
            );
        }

        await sleep(50);
    }
}

function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

async function sha256Base64(bytes: Uint8Array): Promise<string> {
    const digest = await crypto.subtle.digest("SHA-256", Uint8Array.from(bytes).buffer);

    return new Uint8Array(digest).toBase64();
}

// ---------- announcement / comment ----------

export async function createAnnouncement(
    api: ApiClient,
    teamId: string,
    title: string,
    content: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/announcements", {
            content,
            team_id: teamId,
            title,
        }),
        201,
    );
}

export async function updateAnnouncement(
    api: ApiClient,
    announcementId: string,
    title: string,
    content: string,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/announcements/${announcementId}`, {
            content,
            id: announcementId,
            title,
        }),
    );
}

export async function deleteAnnouncement(
    api: ApiClient,
    announcementId: string,
): Promise<void> {
    expectNoContent(
        await api.delete<null>(`/api/v1/announcements/${announcementId}`),
    );
}

export async function listTeamAnnouncements(
    api: ApiClient,
    teamId: string,
): Promise<AnnouncementInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<AnnouncementInfoView[]>>(`/api/v1/teams/${teamId}/announcements?offset=0&limit=100`),
        200,
    );
}

// Paged variant for pagination assertions.
export async function listTeamAnnouncementsPaged(
    api: ApiClient,
    teamId: string,
    offset: number,
    limit: number,
): Promise<AnnouncementInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<AnnouncementInfoView[]>>(
            `/api/v1/teams/${teamId}/announcements?offset=${offset}&limit=${limit}`,
        ),
        200,
    );
}

export async function createComment(
    api: ApiClient,
    teamId: string,
    content: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/comments", {
            content,
            team_id: teamId,
        }),
        201,
    );
}

export async function listTeamComments(api: ApiClient, teamId: string): Promise<CommentInfoView[]> {
    return expectSuccessList(
        await api.get<SuccessBody<CommentInfoView[]>>(`/api/v1/teams/${teamId}/comments?offset=0&limit=100`),
        200,
    );
}

// ---------- translation port (import / export) ----------

// Aggregate translation export (unenveloped). Generates every selected format once.
export async function exportTranslations(
    api: ApiClient,
    chapterId: string,
    formats: ("poprako" | "label_plus")[],
): Promise<ExportChapterTranslationsVal> {
    const response = await api.get<ExportChapterTranslationsVal>(
        `/api/v1/chapters/${chapterId}/translations/export?format=${formats.join(",")}`,
    );

    if (response.status !== 200) {
        throw new Error(`translation export failed: ${response.status} ${response.rawText}`);
    }

    return JSON.parse(response.rawText) as ExportChapterTranslationsVal;
}

// Poprako JSON export. Returns the selected native document.
export async function exportPoprako(api: ApiClient, chapterId: string): Promise<ChapterTranslationPortView> {
    const exported = await exportTranslations(api, chapterId, ["poprako"]);

    assert.assert(exported.poprako, "poprako export must be present");

    return exported.poprako;
}

// Label-plus text export. Returns the selected text document.
export async function exportLabelPlus(api: ApiClient, chapterId: string): Promise<string> {
    const exported = await exportTranslations(api, chapterId, ["label_plus"]);

    assert.assert(exported.label_plus, "label-plus export must be present");

    return exported.label_plus;
}

// Import translations into a chapter with an explicit page-content strategy.
// Returns `{ imported_page_count, imported_unit_count }`.
export async function importTranslations(
    api: ApiClient,
    chapterId: string,
    format: "poprako" | "label_plus",
    mode: "keep" | "overwrite",
    content: string,
): Promise<{ imported_page_count: number; imported_unit_count: number }> {
    return expectSuccessData(
        await api.post<SuccessBody<{ imported_page_count: number; imported_unit_count: number }>>(
            `/api/v1/chapters/${chapterId}/translations/import`,
            { content, format, mode },
        ),
        200,
    );
}

// Re-export the per-user client factory for convenience.
export { clientFor };
