type JsonRecord = Record<string, unknown>;
type JsonBody = JsonRecord | unknown[];

export interface ApiResponse<T> {
    status: number;
    headers: Headers;
    body: ApiBody<T>;
    // Raw response text before JSON parsing. Useful for export endpoints that
    // return unenveloped JSON or plain text (e.g. label-plus download).
    rawText: string;
}

export interface SuccessBody<T> {
    code: 0;
    data: T;
}

export interface ErrorBody {
    code: number;
    message?: string;
}

// Body is either the typed success payload, an error body, a raw string
// (non-JSON responses), or null (empty 204 responses).
export type ApiBody<T> = T | string | null;

export class ApiClient {
    private token: string | null = null;

    private readonly maxRetries = 3;

    constructor(private readonly baseUrl: string) {}

    setToken(token: string): void {
        this.token = token;
    }

    clearToken(): void {
        this.token = null;
    }

    tokenSet(): boolean {
        return this.token !== null;
    }

    get<T>(path: string): Promise<ApiResponse<T>> {
        return this.retryRequest<T>("GET", path);
    }

    post<T>(path: string, body?: JsonBody): Promise<ApiResponse<T>> {
        return this.retryRequest<T>("POST", path, body);
    }

    put<T>(path: string, body?: JsonBody): Promise<ApiResponse<T>> {
        return this.retryRequest<T>("PUT", path, body);
    }

    patch<T>(path: string, body?: JsonBody): Promise<ApiResponse<T>> {
        return this.retryRequest<T>("PATCH", path, body);
    }

    delete<T>(path: string): Promise<ApiResponse<T>> {
        return this.retryRequest<T>("DELETE", path);
    }

    private async retryRequest<T>(
        method: string,
        path: string,
        body?: JsonBody,
    ): Promise<ApiResponse<T>> {
        let lastResponse: ApiResponse<T> | null = null;

        for (let attempt = 0; attempt <= this.maxRetries; attempt++) {
            const response = await this.request<T>(method, path, body);

            if (response.status !== 429) {
                return response;
            }

            lastResponse = response;

            const waitMs = 1000 * (attempt + 1);

            await sleep(waitMs);
        }

        return lastResponse!;
    }

    private async request<T>(
        method: string,
        path: string,
        body?: JsonBody,
    ): Promise<ApiResponse<T>> {
        const headers = new Headers();

        if (body !== undefined) {
            headers.set("content-type", "application/json");
        }

        if (this.token) {
            headers.set("authorization", `Bearer ${this.token}`);
        }

        const response = await fetch(`${this.baseUrl}${path}`, {
            method,
            headers,
            body: body === undefined ? undefined : JSON.stringify(body),
        });

        const text = await response.text();
        const parsedBody = parseBody<T>(text);

        return {
            status: response.status,
            headers: response.headers,
            body: parsedBody,
            rawText: text,
        };
    }
}

// Build a fresh ApiClient authenticated with `token`. Used to spin up one
// client per user persona without mutating the shared sadmin client.
export function clientFor(baseUrl: string, token: string): ApiClient {
    const client = new ApiClient(baseUrl);

    client.setToken(token);

    return client;
}

function parseBody<T>(text: string): ApiBody<T> {
    if (text.length === 0) {
        return null;
    }

    try {
        return JSON.parse(text) as T;
    } catch {
        return text;
    }
}

function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
