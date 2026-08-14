import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { Toaster } from "@/components/ui/toaster"
import { ErrorBoundary } from "@/components/ErrorBoundary"
import { AppHeader } from "@/features/shell/AppHeader"
import { NavRail, type NavView } from "@/features/shell/NavRail"
import { TrafficView } from "@/features/traffic/TrafficView"
import { TierProvider, useTier } from "@/contexts/TierContext"
import { AuthProvider } from "@/features/auth/AuthContext"
import { ProtectedApp } from "@/features/auth/ProtectedApp"
import { lazy, Suspense, useEffect, useState } from "react"
import { Loader2 } from "lucide-react"

// Lazy-load tool views — they get the full main area.
const BreakpointsPanel = lazy(() => import("@/features/tools/BreakpointsPanel").then((m) => ({ default: m.BreakpointsPanel })))
const BlockListPanel = lazy(() => import("@/features/tools/BlockListPanel").then((m) => ({ default: m.BlockListPanel })))
const ThrottlePanel = lazy(() => import("@/features/tools/ThrottlePanel").then((m) => ({ default: m.ThrottlePanel })))
const MocksPanel = lazy(() => import("@/features/tools/MocksPanel").then((m) => ({ default: m.MocksPanel })))
const RewritesPanel = lazy(() => import("@/features/tools/RewritesPanel").then((m) => ({ default: m.RewritesPanel })))
const ReplayPanel = lazy(() => import("@/features/tools/ReplayPanel").then((m) => ({ default: m.ReplayPanel })))
const GrpcPanel = lazy(() => import("@/features/tools/GrpcPanel").then((m) => ({ default: m.GrpcPanel })))
const ScriptsPanel = lazy(() => import("@/features/tools/ScriptsPanel").then((m) => ({ default: m.ScriptsPanel })))
const PluginsPanel = lazy(() => import("@/features/tools/PluginsPanel").then((m) => ({ default: m.PluginsPanel })))
const MirrorPanel = lazy(() => import("@/features/tools/MirrorPanel").then((m) => ({ default: m.MirrorPanel })))
const SessionsPanel = lazy(() => import("@/features/sessions/SessionsPanel").then((m) => ({ default: m.SessionsPanel })))

// Lazy-load enterprise admin panels (separate chunks, only loaded when tier is enterprise).
const UsersPanel = lazy(() => import("@/features/admin/UsersPanel").then((m) => ({ default: m.UsersPanel })))
const AuditPanel = lazy(() => import("@/features/admin/AuditPanel").then((m) => ({ default: m.AuditPanel })))
const MetricsPanel = lazy(() => import("@/features/admin/MetricsPanel").then((m) => ({ default: m.MetricsPanel })))
const LicensePanel = lazy(() => import("@/features/admin/LicensePanel").then((m) => ({ default: m.LicensePanel })))
const ApiKeysPanel = lazy(() => import("@/features/admin/ApiKeysPanel").then((m) => ({ default: m.ApiKeysPanel })))
const InstancesPanel = lazy(() => import("@/features/admin/InstancesPanel").then((m) => ({ default: m.InstancesPanel })))
const SessionTimeoutWarning = lazy(() => import("@/features/auth/SessionTimeoutWarning").then((m) => ({ default: m.SessionTimeoutWarning })))

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: false, staleTime: 1000 },
  },
})

function useTheme() {
  const [isDark, setIsDark] = useState(() => {
    if (typeof window === "undefined") return true
    // Check localStorage first, then fall back to system preference
    const stored = localStorage.getItem("madhyamas-theme")
    if (stored === "dark") return true
    if (stored === "light") return false
    // No stored preference — use system preference
    return window.matchMedia("(prefers-color-scheme: dark)").matches
  })

  useEffect(() => {
    const root = document.documentElement
    root.classList.toggle("dark", isDark)
    root.classList.toggle("light", !isDark)
    // Persist user's explicit choice
    localStorage.setItem("madhyamas-theme", isDark ? "dark" : "light")
  }, [isDark])

  // Listen for system theme changes (only when user hasn't set a preference)
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)")
    const handler = (e: MediaQueryListEvent) => {
      // Only follow system if no explicit preference is stored
      const stored = localStorage.getItem("madhyamas-theme")
      if (!stored) {
        setIsDark(e.matches)
      }
    }
    mq.addEventListener("change", handler)
    return () => mq.removeEventListener("change", handler)
  }, [])

  return { isDark, toggle: () => setIsDark((d) => !d) }
}

const TOOL_VIEWS: NavView[] = [
  { id: "traffic", label: "Traffic", icon: "Activity" },
  { id: "breakpoints", label: "Breakpoints", icon: "Pause" },
  { id: "blocklist", label: "Block List", icon: "Shield" },
  { id: "throttle", label: "Throttle", icon: "Gauge" },
  { id: "mocks", label: "Mocks", icon: "Theater" },
  { id: "rewrites", label: "Rewrites", icon: "Pencil" },
  { id: "replay", label: "Replay", icon: "RotateCcw" },
  { id: "mirror", label: "Mirror", icon: "HardDriveDownload" },
  { id: "grpc", label: "gRPC", icon: "Zap", experimental: true },
  { id: "scripts", label: "Scripts", icon: "Code", experimental: true },
  { id: "plugins", label: "Plugins", icon: "Puzzle", experimental: true },
  { id: "sessions", label: "Sessions", icon: "FolderTree" },
]

const ADMIN_VIEWS: NavView[] = [
  { id: "users", label: "Users", icon: "Users" },
  { id: "audit", label: "Audit Log", icon: "ScrollText" },
  { id: "metrics", label: "Metrics", icon: "BarChart3" },
  { id: "license", label: "License", icon: "KeyRound" },
  { id: "apikeys", label: "API Keys", icon: "Key" },
  { id: "instances", label: "Instances", icon: "Server" },
]

function PanelFallback() {
  return (
    <div className="flex h-full items-center justify-center text-muted-foreground">
      <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading…
    </div>
  )
}

function AppShell() {
  const { isDark, toggle } = useTheme()
  const { tierInfo } = useTier()
  const [activeView, setActiveView] = useState<NavView["id"]>("traffic")

  const isEnterprise = tierInfo?.tier === "enterprise"
  const navViews = isEnterprise ? [...TOOL_VIEWS, ...ADMIN_VIEWS] : TOOL_VIEWS

  return (
    <div className="flex h-full flex-col bg-background text-foreground">
      <AppHeader isDark={isDark} onToggleTheme={toggle} tierInfo={tierInfo} />
      <div className="flex min-h-0 flex-1">
        <NavRail views={navViews} activeView={activeView} onSelect={setActiveView} />
        <main className="min-w-0 flex-1 overflow-hidden">
          <ErrorBoundary label="Panel">
            <Suspense fallback={<PanelFallback />}>
              {activeView === "traffic" && <TrafficView />}
              {activeView === "breakpoints" && <BreakpointsPanel />}
              {activeView === "blocklist" && <BlockListPanel />}
              {activeView === "throttle" && <ThrottlePanel />}
              {activeView === "mocks" && <MocksPanel />}
              {activeView === "rewrites" && <RewritesPanel />}
              {activeView === "replay" && <ReplayPanel />}
              {activeView === "mirror" && <MirrorPanel />}
              {activeView === "grpc" && <GrpcPanel />}
              {activeView === "scripts" && <ScriptsPanel />}
              {activeView === "plugins" && <PluginsPanel />}
              {activeView === "sessions" && <SessionsPanel />}
              {isEnterprise && activeView === "users" && <UsersPanel />}
              {isEnterprise && activeView === "audit" && <AuditPanel />}
              {isEnterprise && activeView === "metrics" && <MetricsPanel />}
              {isEnterprise && activeView === "license" && <LicensePanel />}
              {isEnterprise && activeView === "apikeys" && <ApiKeysPanel />}
              {isEnterprise && activeView === "instances" && <InstancesPanel />}
            </Suspense>
          </ErrorBoundary>
        </main>
      </div>
      {isEnterprise && (
        <Suspense fallback={null}>
          <SessionTimeoutWarning />
        </Suspense>
      )}
    </div>
  )
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <TierProvider>
        <AppGate />
      </TierProvider>
      <Toaster />
    </QueryClientProvider>
  )
}

function AppGate() {
  const { tierInfo, isLoading } = useTier()

  if (isLoading || !tierInfo) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (tierInfo.tier === "enterprise") {
    return (
      <AuthProvider>
        <ProtectedApp tierInfo={tierInfo}>
          <AppShell />
        </ProtectedApp>
      </AuthProvider>
    )
  }

  return <AppShell />
}
