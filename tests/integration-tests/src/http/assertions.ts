import * as assert from "@std/assert";

import type { ApiResponse, ErrorBody, SuccessBody } from "./apiClient.ts";

// Status-only check (no body shape assertion).
export function expectStatus<T>(response: ApiResponse<T>, status: number): void {
    assert.assertEquals(response.status, status, responseText(response));
}

// 204 No Content: status + empty body.
export function expectNoContent<T>(response: ApiResponse<T>): void {
    assert.assertEquals(response.status, 204, responseText(response));
    assert.assert(response.body === null || response.rawText === "", "204 should have empty body");
}

// Success envelope: `{ code: 0, data: T }`. Returns the unwrapped data.
export function expectSuccessData<T>(
    response: ApiResponse<SuccessBody<T>> | ApiResponse<unknown>,
    status: number,
): T {
    expectStatus(response, status);

    const body = response.body as SuccessBody<T> | null;

    assert.assert(body, "response should contain a success body");
    assert.assertEquals(body.code, 0);

    return body.data;
}

// Success envelope that is a list: returns the array and asserts it is one.
export function expectSuccessList<T>(
    response: ApiResponse<SuccessBody<T[]>> | ApiResponse<unknown>,
    status: number,
): T[] {
    const data = expectSuccessData<T[]>(response, status);

    assert.assert(Array.isArray(data), "expected success data to be an array");

    return data;
}

// Error envelope: `{ code: <n>, message?: "..." }`. Asserts status and,
// when `code` is provided, the exact error code.
export function expectError(
    response: ApiResponse<ErrorBody>,
    status: number,
    code?: number,
): ErrorBody {
    expectStatus(response, status);

    assert.assert(response.body, "response should contain an error body");
    assert.assert(isErrorBody(response.body), "response error body should be JSON with code");

    if (code !== undefined) {
        assert.assertEquals(response.body.code, code, `expected error code ${code}`);
    }

    return response.body;
}

// Raw (unenveloped) body: status check only. Returns raw text for export
// endpoints whose content-type is not `application/json` with HttpBody.
export function expectRawBody<T>(response: ApiResponse<T>, status: number): string {
    expectStatus(response, status);

    return response.rawText;
}

// Assert status is one of a set (used for "404 or 403, but not 200" cases).
export function expectStatusIn<T>(response: ApiResponse<T>, statuses: number[]): void {
    assert.assert(
        statuses.includes(response.status),
        `expected status in ${statuses.join(", ")}, got ${response.status}: ${responseText(response)}`,
    );
}

// Reject any 5xx: no client request may produce a server error.
export function expectNoServerError<T>(response: ApiResponse<T>): void {
    assert.assert(
        response.status < 500,
        `unexpected 5xx (${response.status}) — client request must not trigger server error: ${responseText(response)}`,
    );
}

function isErrorBody(body: unknown): body is ErrorBody {
    if (!body || typeof body !== "object") {
        return false;
    }

    return typeof (body as ErrorBody).code === "number";
}

function responseText<T>(response: ApiResponse<T>): string {
    return JSON.stringify(
        {
            body: response.body,
            status: response.status,
        },
        null,
        2,
    );
}
