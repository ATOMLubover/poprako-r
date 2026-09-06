// Unique per-run prefix. Every qid, nickname, team/workset/comic/chapter
// title produced by the suite is prefixed with this string so repeated runs
// do not collide and cleanup is unambiguous.
//
// Format: `it_YYYYMMDD_HHMMSS_` (UTC). Generated once per process; imported
// everywhere via `runPrefix`.

function generatePrefix(): string {
    const now = new Date();

    const pad = (n: number): string => n.toString().padStart(2, "0");

    const stamp = `${now.getUTCFullYear()}${pad(now.getUTCMonth() + 1)}${pad(now.getUTCDate())}` +
        `_${pad(now.getUTCHours())}${pad(now.getUTCMinutes())}${pad(now.getUTCSeconds())}`;

    return `it_${stamp}_`;
}

export const runPrefix: string = generatePrefix();

// Build a qid for a persona short-name, e.g. `prefix("trans_01")` ->
// `it_20260707_160500_trans_01`. QQ ids are string identifiers in the API.
export function qid(persona: string): string {
    return `${runPrefix}${persona}`;
}

// Build a display nickname for a persona.
export function nickname(persona: string): string {
    return `${runPrefix}${persona}`;
}

// Build a password for a persona. Stable across login re-entries in the same run.
export function password(persona: string): string {
    return `${runPrefix}pw_${persona}`;
}

// Build a titled string (workset/comic/chapter/subtitle) for a persona/label.
export function titled(label: string): string {
    return `${runPrefix}${label}`;
}
