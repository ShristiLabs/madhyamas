const API_BASE = '/api'

function getToken(): string | null {
  return localStorage.getItem('portal_token')
}

export function setToken(token: string): void {
  localStorage.setItem('portal_token', token)
}

export function clearToken(): void {
  localStorage.removeItem('portal_token')
}

export function isAuthenticated(): boolean {
  return getToken() !== null
}

async function request<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const token = getToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string>),
  }
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }
  const resp = await fetch(`${API_BASE}${path}`, { ...options, headers })
  if (!resp.ok) {
    const body = await resp.json().catch(() => ({ error: resp.statusText }))
    throw new Error(body.error || `HTTP ${resp.status}`)
  }
  return resp.json() as Promise<T>
}

export interface AuthResponse {
  token: string
  account_id: string
}

export interface MeResponse {
  account_id: string
  email: string
  name: string
  status: string
  customer: {
    id: string
    company_name: string
    contact_email: string
  } | null
}

export interface License {
  id: string
  customer_id: string
  license_id: string
  plan: string
  seats: number
  status: string
  issued_at: string
  expires_at: string
  features: string[]
}

export interface TeamMember {
  id: string
  email: string
  role: string
  status: string
  created_at: string
}

export interface BillingSummary {
  stripe_configured: boolean
  invoices: unknown[]
}

export const api = {
  register: (email: string, password: string, company_name: string) =>
    request<AuthResponse>('/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, password, company_name }),
    }),

  login: (email: string, password: string) =>
    request<AuthResponse>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    }),

  me: () => request<MeResponse>('/auth/me'),

  licenses: () => request<License[]>('/customer/licenses'),

  licenseDetail: (id: string) => request<License>(`/customer/licenses/${id}`),

  seats: (licenseId: string) =>
    request<unknown[]>(`/customer/seats/${licenseId}`),

  team: () => request<TeamMember[]>('/customer/team'),

  invite: (email: string, role: string) =>
    request<TeamMember>('/customer/team', {
      method: 'POST',
      body: JSON.stringify({ email, role }),
    }),

  removeMember: (id: string) =>
    request<{ success: boolean }>(`/customer/team/${id}`, { method: 'POST' }),

  billing: () => request<BillingSummary>('/customer/billing'),

  checkout: (plan: string, successUrl: string, cancelUrl: string) =>
    request<{ checkout_url: string }>('/billing/checkout', {
      method: 'POST',
      body: JSON.stringify({ plan, success_url: successUrl, cancel_url: cancelUrl }),
    }),
}
