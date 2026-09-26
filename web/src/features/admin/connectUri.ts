/** Parameters of the `madhyamas://connect` QR payload (issue #106,
 * docs/CREDENTIAL_ONBOARDING.md QR payload section). Exactly one of
 * `token` / `key` must be set: `token` is the default (single-use
 * 15-minute enrollment token the companion exchanges for the real key);
 * `key` is the manual-mode fallback carrying the show-once credential. */
export interface ConnectUriParams {
  host: string
  port: number
  /** TLS flag of the proxy listener (issue #110): 1 when the instance's
   * proxy port is TLS-wrapped (`proxy_tls` from /api/config) — clients
   * must negotiate TLS before CONNECT — 0 for the plaintext listener. */
  tls: boolean
  name: string
  token?: string
  key?: string
}

/** Build the `madhyamas://connect` deep-link payload. `ca` and `api` are
 * derived from the instance's own origin — the web UI is served by the
 * API server, the same source the manual-apply host/port values use. */
export function buildConnectUri(params: ConnectUriParams): string {
  const q = new URLSearchParams({
    host: params.host,
    port: String(params.port),
    tls: params.tls ? "1" : "0",
    name: params.name,
  })
  if (params.token) q.set("token", params.token)
  if (params.key) q.set("key", params.key)
  const origin = typeof window !== "undefined" ? window.location.origin : ""
  q.set("ca", `${origin}/api/cert/ca`)
  q.set("api", `${origin}/api`)
  return `madhyamas://connect?${q.toString()}`
}
