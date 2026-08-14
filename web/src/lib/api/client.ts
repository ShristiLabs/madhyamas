/**
 * Shared API client for the Madhyamas frontend.
 *
 * All HTTP calls should go through this module rather than calling
 * `fetch()` directly.  This provides:
 *
 * - A single source of truth for the API base URL
 * - Consistent error handling and status checking
 * - Typed helpers for JSON and text responses
 * - Automatic JSON content-type headers for POST/PUT/PATCH
 * - Base-path awareness for load-balancer / context-path deployments
 *
 * The API base is derived from a `<meta name="madhyamas-base-path">` tag
 * injected by the backend (or falls back to `/`). When the frontend is
 * served at `/madhyamas/`, API calls go to `/madhyamas/api/...`.
 *
 * Usage in React Query hooks:
 *
 * ```ts
 * import { apiGet, apiPost } from '@/lib/api/client';
 *
 * export function useBreakpoints() {
 *   return useQuery({
 *     queryKey: ['breakpoints'],
 *     queryFn: () => apiGet<BreakpointRule[]>('/breakpoints'),
 *   });
 * }
 * ```
 */

/**
 * Resolve the base path for API requests. Reads the
 * `<meta name="madhyamas-base-path" content="...">` tag injected by the
 * backend at runtime. Falls back to `/` when the tag is absent (root
 * deployment). The result always starts with `/` and ends with `/`.
 */
function resolveBasePath(): string {
  if (typeof document !== 'undefined') {
    const meta = document.querySelector('meta[name="madhyamas-base-path"]');
    const content = meta?.getAttribute('content');
    if (content && content.trim()) {
      let p = content.trim();
      if (!p.startsWith('/')) p = '/' + p;
      if (!p.endsWith('/')) p = p + '/';
      return p;
    }
  }
  return '/';
}

/** Base URL for all API requests (relative to the page origin). */
const API_BASE = `${resolveBasePath()}api`;

/** localStorage key for the JWT access token. */
const TOKEN_KEY = 'madhyamas-jwt';
/** localStorage key for the JWT refresh token. */
const REFRESH_TOKEN_KEY = 'madhyamas-refresh-jwt';

/** Callback invoked when a 401 is received and refresh fails. */
type UnauthorizedHandler = () => void;
let unauthorizedHandler: UnauthorizedHandler | null = null;

/** Register a handler called when the token is rejected and cannot be refreshed. */
export function setUnauthorizedHandler(handler: UnauthorizedHandler | null): void {
  unauthorizedHandler = handler;
}

/** Read the stored JWT access token (if any). */
export function getAuthToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

/** Read the stored refresh token (if any). */
export function getRefreshToken(): string | null {
  try {
    return localStorage.getItem(REFRESH_TOKEN_KEY);
  } catch {
    return null;
  }
}

/** Persist the access + refresh tokens. */
export function setAuthTokens(token: string, refreshToken?: string): void {
  try {
    localStorage.setItem(TOKEN_KEY, token);
    if (refreshToken) localStorage.setItem(REFRESH_TOKEN_KEY, refreshToken);
  } catch {
    // ignore storage errors (private mode, quota)
  }
}

/** Clear all stored auth tokens. */
export function clearAuthTokens(): void {
  try {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(REFRESH_TOKEN_KEY);
  } catch {
    // ignore
  }
}

/** Build a headers object, merging auth + caller-supplied headers. */
function buildHeaders(init?: RequestInit): Record<string, string> {
  const headers: Record<string, string> = {};
  const token = getAuthToken();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  const existing = init?.headers;
  if (existing) {
    if (existing instanceof Headers) {
      existing.forEach((v, k) => { headers[k] = v; });
    } else if (Array.isArray(existing)) {
      for (const [k, v] of existing) headers[k] = v;
    } else {
      Object.assign(headers, existing);
    }
  }
  return headers;
}

/** Error thrown when an API request fails. */
export class ApiError extends Error {
  /** HTTP status code (0 for network errors). */
  readonly status: number;
  /** Raw response body text (if available). */
  readonly body: string;

  constructor(status: number, body: string, message?: string) {
    super(message ?? `HTTP ${status}: ${body.slice(0, 200)}`);
    this.name = 'ApiError';
    this.status = status;
    this.body = body;
  }
}

/** Build a full URL from a path relative to the API base. */
function buildUrl(path: string): string {
  if (path.startsWith('http://') || path.startsWith('https://')) {
    return path;
  }
  const sep = path.startsWith('/') ? '' : '/';
  return `${API_BASE}${sep}${path}`;
}

/** Attempt a token refresh. Returns true on success. */
async function attemptRefresh(): Promise<boolean> {
  const refreshToken = getRefreshToken();
  if (!refreshToken) return false;
  try {
    const res = await fetch(buildUrl('/auth/refresh'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
    if (!res.ok) return false;
    const data = await res.json() as { token: string; refresh_token?: string };
    setAuthTokens(data.token, data.refresh_token);
    return true;
  } catch {
    return false;
  }
}

/** Tracks whether a refresh is already in-flight to avoid parallel refresh storms. */
let refreshPromise: Promise<boolean> | null = null;

/** Deduplicate concurrent refresh attempts. */
function getRefreshPromise(): Promise<boolean> {
  if (!refreshPromise) {
    refreshPromise = attemptRefresh().finally(() => { refreshPromise = null; });
  }
  return refreshPromise;
}

/** Check response status and throw an `ApiError` on failure. Handles 401 with refresh+retry. */
async function checkResponse(res: Response, retry?: () => Promise<Response>): Promise<void> {
  if (res.status === 401) {
    // Try to refresh the token and retry the original request once.
    if (retry) {
      const refreshed = await getRefreshPromise();
      if (refreshed) {
        const retryRes = await retry();
        if (retryRes.ok) return;
        if (retryRes.status !== 401) {
          const body = await retryRes.text().catch(() => '');
          throw new ApiError(retryRes.status, body);
        }
      }
    }
    // Refresh failed or no retry provided — clear tokens and notify.
    clearAuthTokens();
    if (unauthorizedHandler) unauthorizedHandler();
    throw new ApiError(401, 'Authentication required');
  }
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new ApiError(res.status, body);
  }
}

/** Perform a GET request and parse the response as JSON. */
export async function apiGet<T>(path: string, init?: RequestInit): Promise<T> {
  const doFetch = () => fetch(buildUrl(path), { ...init, method: 'GET', headers: buildHeaders(init) });
  const res = await doFetch();
  await checkResponse(res, doFetch);
  return res.json() as Promise<T>;
}

/** Perform a GET request and return the response as text. */
export async function apiGetText(path: string, init?: RequestInit): Promise<string> {
  const doFetch = () => fetch(buildUrl(path), { ...init, method: 'GET', headers: buildHeaders(init) });
  const res = await doFetch();
  await checkResponse(res, doFetch);
  return res.text();
}

/** Perform a GET request and return the raw `Response` (e.g. for blob downloads). */
export async function apiGetRaw(path: string, init?: RequestInit): Promise<Response> {
  const doFetch = () => fetch(buildUrl(path), { ...init, method: 'GET', headers: buildHeaders(init) });
  const res = await doFetch();
  await checkResponse(res, doFetch);
  return res;
}

/** Perform a POST request with a JSON body and parse the response as JSON. */
export async function apiPost<T>(path: string, body?: unknown, init?: RequestInit): Promise<T> {
  const doFetch = () => fetch(buildUrl(path), {
    ...init,
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...buildHeaders(init) },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const res = await doFetch();
  await checkResponse(res, doFetch);
  if (res.status === 204) {
    return undefined as T;
  }
  const text = await res.text();
  if (!text) {
    return undefined as T;
  }
  return JSON.parse(text) as T;
}

/** Perform a POST request without parsing the response (fire-and-forget). */
export async function apiPostVoid(path: string, body?: unknown, init?: RequestInit): Promise<void> {
  const doFetch = () => fetch(buildUrl(path), {
    ...init,
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...buildHeaders(init) },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const res = await doFetch();
  await checkResponse(res, doFetch);
}

/** Perform a PUT request with a JSON body and parse the response as JSON. */
export async function apiPut<T>(path: string, body?: unknown, init?: RequestInit): Promise<T> {
  const doFetch = () => fetch(buildUrl(path), {
    ...init,
    method: 'PUT',
    headers: { 'Content-Type': 'application/json', ...buildHeaders(init) },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const res = await doFetch();
  await checkResponse(res, doFetch);
  if (res.status === 204) {
    return undefined as T;
  }
  const text = await res.text();
  if (!text) {
    return undefined as T;
  }
  return JSON.parse(text) as T;
}

/** Perform a PATCH request with a JSON body and parse the response as JSON. */
export async function apiPatch<T>(path: string, body?: unknown, init?: RequestInit): Promise<T> {
  const doFetch = () => fetch(buildUrl(path), {
    ...init,
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json', ...buildHeaders(init) },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const res = await doFetch();
  await checkResponse(res, doFetch);
  if (res.status === 204) {
    return undefined as T;
  }
  const text = await res.text();
  if (!text) {
    return undefined as T;
  }
  return JSON.parse(text) as T;
}

/** Perform a DELETE request and parse the response as JSON (if any). */
export async function apiDelete<T>(path: string, init?: RequestInit): Promise<T> {
  const doFetch = () => fetch(buildUrl(path), { ...init, method: 'DELETE', headers: buildHeaders(init) });
  const res = await doFetch();
  await checkResponse(res, doFetch);
  if (res.status === 204) {
    return undefined as T;
  }
  const text = await res.text();
  if (!text) {
    return undefined as T;
  }
  return JSON.parse(text) as T;
}

/** Perform a DELETE request without parsing the response. */
export async function apiDeleteVoid(path: string, init?: RequestInit): Promise<void> {
  const doFetch = () => fetch(buildUrl(path), { ...init, method: 'DELETE', headers: buildHeaders(init) });
  const res = await doFetch();
  await checkResponse(res, doFetch);
}

/** Export the base URL for components that need raw URL construction. */
export { API_BASE };
