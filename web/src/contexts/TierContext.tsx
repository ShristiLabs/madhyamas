/**
 * Tier detection context.
 *
 * Fetches `/api/health/detailed` on app load to determine whether the
 * backend is the enterprise or community build. Enterprise UI elements
 * (login, admin panels) are only shown when `tier === "enterprise"`.
 * In the community tier the enterprise chunks are never imported.
 */
import { createContext, useContext, useEffect, useState, type ReactNode } from "react"
import { apiGet, ApiError } from "@/lib/api/client"

export type Tier = "enterprise" | "community"

export interface TierInfo {
  tier: Tier
  authMode: string
  authRequired: boolean
  license: { licensed: boolean; expiresAt?: string } | null
}

interface TierContextValue {
  tier: Tier | null
  tierInfo: TierInfo | null
  isLoading: boolean
}

const TierContext = createContext<TierContextValue | null>(null)

const COMMUNITY_FALLBACK: TierInfo = {
  tier: "community",
  authMode: "none",
  authRequired: false,
  license: null,
}

export function TierProvider({ children }: { children: ReactNode }) {
  const [tierInfo, setTierInfo] = useState<TierInfo | null>(null)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    let cancelled = false
    apiGet<RawHealthResponse>("/health/detailed")
      .then((data) => {
        if (cancelled) return
        setTierInfo(normalizeHealth(data))
      })
      .catch((err: unknown) => {
        if (cancelled) return
        // 404 or network error → community (endpoint doesn't exist in OSS).
        if (err instanceof ApiError && err.status === 404) {
          setTierInfo(COMMUNITY_FALLBACK)
        } else {
          setTierInfo(COMMUNITY_FALLBACK)
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false)
      })
    return () => { cancelled = true }
  }, [])

  return (
    <TierContext.Provider
      value={{ tier: tierInfo?.tier ?? null, tierInfo, isLoading }}
    >
      {children}
    </TierContext.Provider>
  )
}

// eslint-disable-next-line react-refresh/only-export-components
export function useTier(): TierContextValue {
  const ctx = useContext(TierContext)
  if (!ctx) throw new Error("useTier must be used within TierProvider")
  return ctx
}

interface RawHealthResponse {
  tier?: string
  auth_mode?: string
  auth_required?: boolean
  license?: { licensed?: boolean; expires_at?: string }
}

function normalizeHealth(data: RawHealthResponse): TierInfo {
  const tier: Tier = data.tier === "enterprise" ? "enterprise" : "community"
  return {
    tier,
    authMode: data.auth_mode ?? "none",
    authRequired: data.auth_required ?? false,
    license: data.license
      ? { licensed: data.license.licensed ?? false, expiresAt: data.license.expires_at }
      : null,
  }
}
