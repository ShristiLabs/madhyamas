/**
 * ProtectedApp — wraps the main app, showing LoginPage when auth is required
 * but the user is not authenticated.
 *
 * - If tier is "community" or auth is not required → render children directly.
 * - If tier is "enterprise" and auth is required and user is not
 *   authenticated → render LoginPage.
 */
import { lazy, Suspense, type ReactNode } from "react"
import { Loader2 } from "lucide-react"
import { useTier, type TierInfo } from "@/contexts/TierContext"
import { useAuth } from "./AuthContext"

const LoginPage = lazy(() =>
  import("./LoginPage").then((m) => ({ default: m.LoginPage })),
)

export function ProtectedApp({ tierInfo, children }: {
  tierInfo: TierInfo
  children: ReactNode
}) {
  const { isAuthenticated, isLoading } = useAuth()

  // Community tier or auth not required → no gate.
  if (tierInfo.tier === "community" || !tierInfo.authRequired) {
    return <>{children}</>
  }

  // Enterprise tier with auth required.
  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (!isAuthenticated) {
    return (
      <Suspense
        fallback={
          <div className="flex h-full items-center justify-center">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <LoginPage />
      </Suspense>
    )
  }

  return <>{children}</>
}

/** Convenience hook for components that need both tier + auth state. */
export function useEnterpriseGate() {
  const { tierInfo } = useTier()
  const { isAuthenticated } = useAuth()
  const isEnterprise = tierInfo?.tier === "enterprise"
  const isAdmin = isEnterprise && isAuthenticated
  return { isEnterprise, isAuthenticated, tierInfo, isAdmin }
}
