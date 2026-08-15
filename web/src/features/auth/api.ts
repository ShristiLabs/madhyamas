/**
 * Auth API client — thin wrappers over the enterprise auth endpoints.
 *
 * All calls go through the shared API client (`@/lib/api/client`) which
 * handles base-path resolution, auth header injection, and 401 refresh.
 */
import { apiGet, apiPost, apiPostVoid } from "@/lib/api/client"

export interface AuthUser {
  id: string
  username: string
  email: string
  role: string
}

export interface LoginResponse {
  token: string
  refresh_token: string
  user: AuthUser
  expires_at: number
}

export interface RefreshResponse {
  token: string
  refresh_token: string
  expires_at: number
}

export function loginApi(username: string, password: string): Promise<LoginResponse> {
  return apiPost<LoginResponse>("/auth/login", { username, password })
}

export function logoutApi(): Promise<void> {
  return apiPostVoid("/auth/logout")
}

export function getMeApi(): Promise<AuthUser> {
  return apiGet<AuthUser>("/auth/me")
}

export function refreshApi(refreshToken: string): Promise<RefreshResponse> {
  return apiPost<RefreshResponse>("/auth/refresh", { refresh_token: refreshToken })
}
