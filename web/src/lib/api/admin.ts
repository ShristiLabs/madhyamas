/**
 * Admin API client — wrappers for the enterprise admin endpoints.
 *
 * Covers user management, audit logs, metrics, license info, API keys,
 * and multi-instance management. All calls go through the shared API
 * client which handles auth headers and 401 refresh.
 */
import { apiGet, apiPost, apiPostVoid, apiPut, apiDeleteVoid, apiGetRaw } from "./client"

// ============================================================================
// Users
// ============================================================================

export interface AdminUser {
  id: string
  username: string
  email: string | null
  display_name: string
  role: string
  status: string
  created_at: number
  last_login: number | null
}

export interface CreateUserPayload {
  username: string
  email: string
  password: string
  role: string
}

export interface UpdateUserPayload {
  email?: string
  role?: string
  status?: string
  password?: string
}

export function listUsersApi(): Promise<AdminUser[]> {
  return apiGet<AdminUser[]>("/users")
}

export function createUserApi(data: CreateUserPayload): Promise<AdminUser> {
  return apiPost<AdminUser>("/users", data)
}

export function updateUserApi(id: string, data: UpdateUserPayload): Promise<AdminUser> {
  return apiPut<AdminUser>(`/users/${id}`, data)
}

export function deleteUserApi(id: string): Promise<void> {
  return apiDeleteVoid(`/users/${id}`)
}

// ============================================================================
// Audit
// ============================================================================

export interface AuditEventEntry {
  id: string
  event_type: string
  timestamp: string
  user_id: string | null
  api_key_id: string | null
  client_ip: string | null
  description: string
  metadata: Record<string, unknown>
  prev_hash?: string | null
  hash?: string | null
}

export interface AuditStats {
  total_events: number
  events_today: number
  events_by_type: Record<string, number>
  top_users: string[]
  error_count: number
}

export interface AuditFilter {
  event_types?: string
  user_id?: string
  start_time?: number
  end_time?: number
  limit?: number
  offset?: number
}

export function listAuditApi(filter: AuditFilter): Promise<AuditEventEntry[]> {
  const params = new URLSearchParams()
  if (filter.event_types) params.set("event_types", filter.event_types)
  if (filter.user_id) params.set("user_id", filter.user_id)
  if (filter.start_time) params.set("start_time", String(filter.start_time))
  if (filter.end_time) params.set("end_time", String(filter.end_time))
  if (filter.limit) params.set("limit", String(filter.limit))
  if (filter.offset) params.set("offset", String(filter.offset))
  const qs = params.toString()
  return apiGet<AuditEventEntry[]>(`/audit${qs ? `?${qs}` : ""}`)
}

export function getAuditStatsApi(): Promise<AuditStats> {
  return apiGet<AuditStats>("/audit/stats")
}

export async function exportAuditApi(): Promise<Blob> {
  const res = await apiGetRaw("/audit/export")
  return res.blob()
}

// ============================================================================
// Metrics
// ============================================================================

export interface MetricsSnapshot {
  requests_total: number
  requests_success: number
  requests_failed: number
  avg_latency_ms: number
  requests_per_second: number
}

export interface ClusterMetrics {
  instances: InstanceSummary[]
  total_active_connections: number
  total_request_count: number
  avg_cpu_usage: number
  avg_memory_usage_mb: number
}

export interface InstanceSummary {
  instance_id: string
  addr: string
  last_heartbeat: number
  status: string
  cpu_usage: number
  memory_usage_mb: number
  active_connections: number
  request_count: number
  uptime_secs: number
}

export function getMetricsApi(): Promise<MetricsSnapshot> {
  return apiGet<MetricsSnapshot>("/metrics")
}

export function getClusterMetricsApi(): Promise<ClusterMetrics> {
  return apiGet<ClusterMetrics>("/metrics/cluster")
}

// ============================================================================
// License
// ============================================================================

export interface LicenseInfo {
  licensed: boolean
  license_id?: string
  customer?: string
  plan?: string
  seats?: number
  instance_id?: string
  issued_at?: string
  expires_at?: string
  features?: string[]
  verified_at?: string
}

export function getLicenseApi(): Promise<LicenseInfo> {
  return apiGet<LicenseInfo>("/license")
}

// ============================================================================
// API Keys
// ============================================================================

export interface ApiKeyEntry {
  id: string
  user_id: string
  key: string
  name: string
  created_at: number
  expires_at: number | null
  is_active: boolean
  last_used: number | null
  scopes: string[]
}

export interface CreateApiKeyPayload {
  name: string
  scopes?: string[]
  expires_in_days?: number
}

export function listApiKeysApi(): Promise<ApiKeyEntry[]> {
  return apiGet<ApiKeyEntry[]>("/auth/api-keys")
}

export function createApiKeyApi(data: CreateApiKeyPayload): Promise<ApiKeyEntry> {
  return apiPost<ApiKeyEntry>("/auth/api-keys", data)
}

export function revokeApiKeyApi(id: string): Promise<void> {
  return apiDeleteVoid(`/auth/api-keys/${id}`)
}

// ============================================================================
// Devices (enterprise, issue #104)
// ============================================================================

export interface DeviceEntry {
  id: string
  name: string
  owner_user_id: string
  install_uuid: string | null
  mac_address: string | null
  status: string
  created_at: number
  last_seen: number | null
}

export interface DeviceWithKey {
  device: DeviceEntry
  /** Plaintext mdy_dev_ credential — shown once at creation/rotation. */
  key: string
}

export interface CreateDevicePayload {
  name: string
  install_uuid?: string
  mac_address?: string
}

export function listDevicesApi(): Promise<DeviceEntry[]> {
  return apiGet<DeviceEntry[]>("/devices")
}

export function createDeviceApi(data: CreateDevicePayload): Promise<DeviceWithKey> {
  return apiPost<DeviceWithKey>("/devices", data)
}

export function rotateDeviceKeyApi(id: string): Promise<DeviceWithKey> {
  return apiPost<DeviceWithKey>(`/devices/${id}/rotate`, {})
}

export function revokeDeviceApi(id: string): Promise<void> {
  return apiPostVoid(`/devices/${id}/revoke`, {})
}

export function deleteDeviceApi(id: string): Promise<void> {
  return apiDeleteVoid(`/devices/${id}`)
}

/** Issued enrollment token (issue #106): plaintext mdy_enroll_ token shown
 * once in the QR payload, plus the RFC 3339 instant it expires. */
export interface DeviceEnrollmentToken {
  device: DeviceEntry
  /** Plaintext mdy_enroll_ token — carried by the QR, single-use, 15-min TTL. */
  token: string
  /** RFC 3339 timestamp after which redemption is rejected. */
  expires_at: string
}

export function createEnrollmentTokenApi(id: string): Promise<DeviceEnrollmentToken> {
  return apiPost<DeviceEnrollmentToken>(`/devices/${id}/enrollment-token`, {})
}

// ============================================================================
// Device-derived agent keys (enterprise, issue #108)
// ============================================================================

/** An agent key row as listed on a device — metadata only, never a secret. */
export interface AgentKeyEntry {
  id: string
  parent_device_id: string
  owner_user_id: string
  name: string
  /** Non-secret preview, e.g. "mdy_agent_ab…". */
  key_prefix: string
  scopes: string[]
  /** Unix seconds. */
  created_at: number
  /** Unix seconds, null = never expires. */
  expires_at: number | null
  /** "active" | "revoked". */
  status: string
  /** Unix seconds of the last request made with this key. */
  last_used: number | null
}

/** A freshly minted agent key: metadata + the show-once plaintext. */
export interface AgentKeyWithSecret {
  key: AgentKeyEntry
  /** Plaintext mdy_agent_ credential — shown once at mint, never stored. */
  secret: string
}

export interface CreateAgentKeyPayload {
  name?: string
  /** Preset shortcut: "read-only-agent" | "intercept-agent". */
  preset?: string
  /** Explicit feature scopes (unioned with the preset). */
  scopes?: string[]
  /** Optional expiry in days (> 0). */
  expires_in_days?: number
}

export function listAgentKeysApi(deviceId: string): Promise<AgentKeyEntry[]> {
  return apiGet<AgentKeyEntry[]>(`/devices/${deviceId}/agent-keys`)
}

export function createAgentKeyApi(
  deviceId: string,
  data: CreateAgentKeyPayload,
): Promise<AgentKeyWithSecret> {
  return apiPost<AgentKeyWithSecret>(`/devices/${deviceId}/agent-keys`, data)
}

export function revokeAgentKeyApi(deviceId: string, keyId: string): Promise<void> {
  return apiDeleteVoid(`/devices/${deviceId}/agent-keys/${keyId}`)
}

// ============================================================================
// Instances
// ============================================================================

export interface InstanceEntry {
  instance_id: string
  addr: string
  last_heartbeat: number
  status: string
}

export interface InstancesResponse {
  instances: InstanceEntry[]
}

export function listInstancesApi(): Promise<InstancesResponse> {
  return apiGet<InstancesResponse>("/instances")
}
