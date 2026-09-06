// Role bitmask constants. Values MUST match `crate::value::role::RoleField`
// in the Rust source (`src/value/role.rs`):
//
//   RAW_PROVIDER = 1 << 0  = 1
//   TRANSLATOR   = 1 << 1  = 2
//   PROOFREADER  = 1 << 2  = 4
//   TYPESETTER   = 1 << 3  = 8
//   REDRAWER     = 1 << 4  = 16
//   REVIEWER     = 1 << 5  = 32
//   PUBLISHER    = 1 << 6  = 64
//   ADMIN        = 1 << 7  = 128
//   BOT          = 1 << 8  = 256
//
// `RoleField` (single-bit) is used by list filters: composite values are
// rejected with 422 code 2. `RoleMask` (composite) is used by create/update
// payloads: any combination of valid bits is accepted.

export const ROLE = {
    RAW_PROVIDER: 1,
    TRANSLATOR: 2,
    PROOFREADER: 4,
    TYPESETTER: 8,
    REDRAWER: 16,
    REVIEWER: 32,
    PUBLISHER: 64,
    ADMIN: 128,
    BOT: 256,
} as const;

export type RoleBit = (typeof ROLE)[keyof typeof ROLE];

// All valid single-bit role values, used to assert composite-role rejection.
export const ALL_ROLE_BITS: readonly number[] = Object.values(ROLE);

// Convenience composite masks used by the suite.
export const ROLE_MASK = {
    RAW: ROLE.RAW_PROVIDER,
    TRANSLATOR: ROLE.TRANSLATOR,
    PROOFREADER: ROLE.PROOFREADER,
    TYPESETTER: ROLE.TYPESETTER,
    REDRAWER: ROLE.REDRAWER,
    REVIEWER: ROLE.REVIEWER,
    PUBLISHER: ROLE.PUBLISHER,
    ADMIN: ROLE.ADMIN,
    BOT: ROLE.BOT,
    // Common composites used in the plan.
    RAW_OR_TRANSLATOR: ROLE.RAW_PROVIDER | ROLE.TRANSLATOR,
    RAW_TRANS_PROOF: ROLE.RAW_PROVIDER | ROLE.TRANSLATOR | ROLE.PROOFREADER,
    TRANS_PROOF: ROLE.TRANSLATOR | ROLE.PROOFREADER,
    ALL_WORKERS: ROLE.RAW_PROVIDER |
        ROLE.TRANSLATOR |
        ROLE.PROOFREADER |
        ROLE.TYPESETTER |
        ROLE.REDRAWER |
        ROLE.REVIEWER |
        ROLE.PUBLISHER,
} as const;

// Worker bit count for membership seed. The seed sadmin member is granted
// every worker role timestamp; tests that re-create members use ALL_WORKERS.
export const SEED_MEMBER_ROLES = ROLE_MASK.ALL_WORKERS | ROLE.ADMIN;
