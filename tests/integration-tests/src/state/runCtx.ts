import type { ApiClient } from "../http/apiClient.ts";

// RunCtx is the single shared state object carried across all 14 progressive
// integration modules (`suites/it_00_*` through `suites/it_13_*`).
//
// Each module:
//   - reads its preconditions from RunCtx (ids/clients set by earlier modules),
//   - mutates RunCtx to publish ids it creates,
//   - leaves RunCtx in a state satisfying the next module's preconditions.
//
// This progressive handoff lets an agent implement one module at a time
// without understanding the whole suite: the module's header doc states its
// preconditions and postconditions explicitly.
//
// Field naming follows the AGENTS.md "typed local names must be specific"
// rule: domain-qualified ids, never bare `id`.

export interface UserClient {
    // Stable persona key, e.g. "trans_01". Matches keys in `personas` map.
    persona: string;

    // Authenticated API client for this user.
    api: ApiClient;

    // Server-side user id assigned at registration/login.
    userId: string;

    // qid used to log in / register.
    qid: string;

    // Default-team member id, set after registration (sadmin's is from seed).
    memberIds: Record<string, string>;

    // Roles mask granted on the default-team membership.
    roles: number;
}

export interface ChapterRefs {
    chapterId: string;
    comicId: string;
    worksetId: string;
    // Page ids in index order (0..n-1).
    pageIds: string[];
    // Assignment id keyed by persona, for this chapter.
    assignmentIds: Record<string, string>;
}

export interface InvitationRef {
    id: string;
    code: string;
    inviteeQid: string;
    roles: number;
}

export interface AssignmentInvitationRef {
    id: string;
    code: string;
    inviteeQid: string;
    roles: number;
}

export interface RunCtx {
    // Single sadmin client (seed user). Always set by it_00.
    sadmin: ApiClient;

    // All per-user clients keyed by persona. `sadmin` is NOT in this map.
    users: Map<string, UserClient>;

    // Server ids discovered/created during the run.
    ids: RunIds;

    // The 15-persona member matrix from the test plan (section 0 / B1).
    // Set up by it_01; consumed by it_04+.
    personas: MemberPersona[];

    // Chapter working set shared across modules. `main` is the high-traffic
    // chapter under `连载池 / 星尘旅人`. Auxiliary chapters for destructive
    // tests live in `auxChapters` keyed by label.
    main: ChapterRefs | null;

    auxChapters: Map<string, ChapterRefs>;

    // Second team + outsider for cross-team isolation (it_09). The outsider's
    // member and invitation ids are stored so it_10 can delete them before
    // the team (FK RESTRICT chain).
    secondTeam: {
        teamId: string;
        outsider: UserClient | null;
        outsiderInvitationId: string;
        outsiderMemberId: string;
    } | null;

    // Announcement/comment ids still requiring direct cleanup after each suite.
    leftoverCommentIds: string[];
    leftoverAnnouncementIds: string[];

    // Whether each module has completed. Updated by main.ts; used by cleanup
    // and by later modules to decide whether to skip dependent assertions.
    moduleStatus: Record<string, ModuleStatus>;
}

export interface RunIds {
    // Seed default team, user, member.
    defaultTeamId: string;
    defaultUserId: string;
    defaultMemberId: string;

    // Worksets created by it_02, keyed by label (`连载池`, `短篇池`, ...).
    worksetIds: Record<string, string>;

    // Comics created by it_02, keyed by title label.
    comicIds: Record<string, string>;

    // First-chapter id for each comic, keyed by comic label.
    firstChapterIds: Record<string, string>;
}

export interface MemberPersona {
    persona: string;

    qid: string;

    // Initial roles to invite with (single bit, except guest_01 which is
    // later widened).
    roles: number;

    // Human-readable role label for assertion messages.
    roleLabel: string;
}

export type ModuleStatus = "pending" | "running" | "done" | "skipped";

// The 14 ordinary members + 1 outsider-defining persona from the plan.
// `outsider_01` is invited to the SECOND team, not the default team.
export const DEFAULT_TEAM_PERSONAS: readonly MemberPersona[] = [
    { persona: "raw_01", qid: "", roles: 1, roleLabel: "raw-provider" },
    { persona: "raw_02", qid: "", roles: 1, roleLabel: "raw-provider" },
    { persona: "trans_01", qid: "", roles: 2, roleLabel: "translator" },
    { persona: "trans_02", qid: "", roles: 2, roleLabel: "translator" },
    { persona: "trans_03", qid: "", roles: 2, roleLabel: "translator" },
    { persona: "proof_01", qid: "", roles: 4, roleLabel: "proofreader" },
    { persona: "proof_02", qid: "", roles: 4, roleLabel: "proofreader" },
    { persona: "type_01", qid: "", roles: 8, roleLabel: "typesetter" },
    { persona: "type_02", qid: "", roles: 8, roleLabel: "typesetter" },
    { persona: "redraw_01", qid: "", roles: 16, roleLabel: "redrawer" },
    { persona: "review_01", qid: "", roles: 32, roleLabel: "reviewer" },
    { persona: "review_02", qid: "", roles: 32, roleLabel: "reviewer" },
    { persona: "publish_01", qid: "", roles: 64, roleLabel: "publisher" },
    { persona: "guest_01", qid: "", roles: 1, roleLabel: "raw-provider" },
];

export const OUTSIDER_PERSONA: MemberPersona = {
    persona: "outsider_01",
    qid: "",
    roles: 1,
    roleLabel: "raw-provider",
};
