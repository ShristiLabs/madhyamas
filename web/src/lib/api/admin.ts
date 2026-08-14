/**
 * Admin API client — wrappers for the enterprise admin endpoints.
 *
 * Covers user management, audit logs, metrics, license info, API keys,
 * and multi-instance management. All calls go through the shared API
 * client which handles auth headers and 401 refresh.
 */
import { apiGet, apiPost, apiPut, apiDeleteVoid, apiGetRaw } from "./client"

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
