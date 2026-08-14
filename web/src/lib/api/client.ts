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

/** Check response status and throw an `ApiError` on failure. */
async function checkResponse(res: Response): Promise<void> {
  if (!res.ok) {
    const body = await res.text().catch(() => '');
    throw new ApiError(res.status, body);
  }
}

/** Perform a GET request and parse the response as JSON. */
export async function apiGet<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'GET',
  });
  await checkResponse(res);
  return res.json() as Promise<T>;
}

/** Perform a GET request and return the response as text. */
export async function apiGetText(path: string, init?: RequestInit): Promise<string> {
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'GET',
  });
  await checkResponse(res);
  return res.text();
}

/** Perform a GET request and return the raw `Response` (e.g. for blob downloads). */
export async function apiGetRaw(path: string, init?: RequestInit): Promise<Response> {
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'GET',
  });
  await checkResponse(res);
  return res;
}

/** Perform a POST request with a JSON body and parse the response as JSON. */
export async function apiPost<T>(path: string, body?: unknown, init?: RequestInit): Promise<T> {
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  await checkResponse(res);
  // Handle 204 No Content and empty bodies
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
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  await checkResponse(res);
}

/** Perform a PUT request with a JSON body and parse the response as JSON. */
export async function apiPut<T>(path: string, body?: unknown, init?: RequestInit): Promise<T> {
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  await checkResponse(res);
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
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  await checkResponse(res);
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
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'DELETE',
  });
  await checkResponse(res);
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
  const res = await fetch(buildUrl(path), {
    ...init,
    method: 'DELETE',
  });
  await checkResponse(res);
}

/** Export the base URL for components that need raw URL construction. */
export { API_BASE };
