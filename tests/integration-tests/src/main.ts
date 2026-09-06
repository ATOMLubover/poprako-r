import { testEnv } from "./config/env.ts";
import { assertDatabaseIsSeedOnly, cleanupToSeed, resetDatabase, seedIds } from "./db/seed.ts";
import { ApiClient } from "./http/apiClient.ts";
import { IMPLEMENTED as IT00_IMPLEMENTED, runIt00Module } from "./suites/it_00_bootstrap_auth_default_seed.ts";
import { IMPLEMENTED as IT01_IMPLEMENTED, runIt01Module } from "./suites/it_01_member_invitation_register_roles.ts";
import { IMPLEMENTED as IT02_IMPLEMENTED, runIt02Module } from "./suites/it_02_workset_comic_chapter_index.ts";
import { IMPLEMENTED as IT03_IMPLEMENTED, runIt03Module } from "./suites/it_03_page_reserve_image.ts";
import { IMPLEMENTED as IT04_IMPLEMENTED, runIt04Module } from "./suites/it_04_assignment_invitation.ts";
import { IMPLEMENTED as IT05_IMPLEMENTED, runIt05Module } from "./suites/it_05_unit_save_order_count.ts";
import { IMPLEMENTED as IT06_IMPLEMENTED, runIt06Module } from "./suites/it_06_unit_concurrency.ts";
import { IMPLEMENTED as IT07_IMPLEMENTED, runIt07Module } from "./suites/it_07_workflow_sysmail.ts";
import { IMPLEMENTED as IT08_IMPLEMENTED, runIt08Module } from "./suites/it_08_info_update_upload_mark.ts";
import { IMPLEMENTED as IT09_IMPLEMENTED, runIt09Module } from "./suites/it_09_cross_team_perm.ts";
import { IMPLEMENTED as IT10_IMPLEMENTED, runIt10Module } from "./suites/it_10_cascade_delete_cleanup.ts";
import { IMPLEMENTED as IT11_IMPLEMENTED, runIt11Module } from "./suites/it_11_comic_archive.ts";
import { IMPLEMENTED as IT12_IMPLEMENTED, runIt12Module } from "./suites/it_12_termbase_term.ts";
import type { RunCtx } from "./state/runCtx.ts";

// Progressive integration test orchestration.
//
// The suite runs 13 modules in dependency order. Each module reads its
// preconditions from `RunCtx` and publishes what it creates back into
// `RunCtx` for the next module. Modules whose `IMPLEMENTED` flag is `false`
// are skipped (visible in the test output as skipped subtests), so the suite
// is green during the progressive handoff while a module is still a stub.
//
// The `IMPLEMENTED` flag is the single source of truth — it is imported from
// each module file, so a handoff agent only flips it in the module and main.ts
// picks it up automatically (no drift).
//
// Order:
//   it_00 bootstrap + auth + default-seed discovery
//   it_01 member invitation / register / roles / member list
//   it_02 workset / comic / chapter index + pinning + info update
//   it_03 page reserve / image mark / page delete+rebuild
//   it_04 assignment join / assignment invitation / role update+delete
//   it_05 unit save order / next_id / counts / export / import
//   it_06 unit concurrency (parallel writes / conflicts / inserts)
//   it_07 workflow advance/revert + system mail
//   it_08 info update / avatar / cover / announcements / comments / profile
//   it_09 cross-team isolation (second team + outsider)
//   it_10 cascade delete (chapter -> comic -> workset -> team)
//   it_11 immutable comic archive and image cleanup records
//   it_12 termbase / term lifecycle, perms, search, and cascades
//
// Cleanup: `cleanupToSeed()` runs in the `finally` block BEFORE
// `assertDatabaseIsSeedOnly()` so the assert verifies the suite self-cleans
// rather than verifying that `resetDatabase` works. `cleanupToSeed` is robust
// to partial runs (it deletes everything non-seed in FK-safe order), so a
// run where only it_00 + it_01 executed still passes the seed-only assert.

interface ModuleEntry {
    name: string;

    implemented: boolean;

    run: (ctx: RunCtx) => Promise<void>;
}

const modules: ModuleEntry[] = [
    { name: "it_00 bootstrap auth default seed", implemented: IT00_IMPLEMENTED, run: runIt00Module },
    { name: "it_01 member invitation register roles", implemented: IT01_IMPLEMENTED, run: runIt01Module },
    { name: "it_02 workset comic chapter index", implemented: IT02_IMPLEMENTED, run: runIt02Module },
    { name: "it_03 page reserve image", implemented: IT03_IMPLEMENTED, run: runIt03Module },
    { name: "it_04 assignment invitation", implemented: IT04_IMPLEMENTED, run: runIt04Module },
    { name: "it_05 unit save order count", implemented: IT05_IMPLEMENTED, run: runIt05Module },
    { name: "it_06 unit concurrency", implemented: IT06_IMPLEMENTED, run: runIt06Module },
    { name: "it_07 workflow sysmail", implemented: IT07_IMPLEMENTED, run: runIt07Module },
    { name: "it_08 info update upload mark", implemented: IT08_IMPLEMENTED, run: runIt08Module },
    { name: "it_09 cross team perm", implemented: IT09_IMPLEMENTED, run: runIt09Module },
    { name: "it_10 cascade delete cleanup", implemented: IT10_IMPLEMENTED, run: runIt10Module },
    { name: "it_11 comic archive", implemented: IT11_IMPLEMENTED, run: runIt11Module },
    { name: "it_12 termbase term", implemented: IT12_IMPLEMENTED, run: runIt12Module },
];

Deno.test("poprako HTTP API integration (progressive)", async (outerT) => {
    await outerT.step("reset database to seed", async () => {
        await resetDatabase();
    });

    const ctx: RunCtx = {
        sadmin: new ApiClient(testEnv.apiBaseUrl),
        users: new Map(),
        ids: {
            defaultTeamId: seedIds.defaultTeamId,
            defaultUserId: seedIds.defaultUserId,
            defaultMemberId: seedIds.defaultMemberId,
            worksetIds: {},
            comicIds: {},
            firstChapterIds: {},
        },
        personas: [],
        main: null,
        auxChapters: new Map(),
        secondTeam: null,
        leftoverCommentIds: [],
        leftoverAnnouncementIds: [],
        moduleStatus: {},
    };

    try {
        for (const module of modules) {
            ctx.moduleStatus[module.name] = module.implemented ? "running" : "skipped";

            // `ignore` makes the stub appear as a skipped subtest in the output
            // rather than a failure, so the progressive handoff stays green.
            await outerT.step({
                name: module.name,
                ignore: !module.implemented,
                fn: async () => {
                    await module.run(ctx);
                },
            });

            ctx.moduleStatus[module.name] = module.implemented ? "done" : "skipped";
        }
    } finally {
        await outerT.step("cleanup to seed state", async () => {
            await cleanupToSeed();
        });

        await outerT.step("assert database is seed-only", async () => {
            await assertDatabaseIsSeedOnly();
        });
    }
});
