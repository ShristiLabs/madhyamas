import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { Toaster } from "@/components/ui/toaster"
import { AppHeader } from "@/features/shell/AppHeader"
import { NavRail, type NavView } from "@/features/shell/NavRail"
import { TrafficView } from "@/features/traffic/TrafficView"
import { lazy, Suspense, useEffect, useState } from "react"
import { Loader2 } from "lucide-react"

// Lazy-load tool views — they get the full main area.
const BreakpointsPanel = lazy(() => import("@/features/tools/BreakpointsPanel").then((m) => ({ default: m.BreakpointsPanel })))
const ThrottlePanel = lazy(() => import("@/features/tools/ThrottlePanel").then((m) => ({ default: m.ThrottlePanel })))
const MocksPanel = lazy(() => import("@/features/tools/MocksPanel").then((m) => ({ default: m.MocksPanel })))
const RewritesPanel = lazy(() => import("@/features/tools/RewritesPanel").then((m) => ({ default: m.RewritesPanel })))
const ReplayPanel = lazy(() => import("@/features/tools/ReplayPanel").then((m) => ({ default: m.ReplayPanel })))
const GrpcPanel = lazy(() => import("@/features/tools/GrpcPanel").then((m) => ({ default: m.GrpcPanel })))
const ScriptsPanel = lazy(() => import("@/features/tools/ScriptsPanel").then((m) => ({ default: m.ScriptsPanel })))
const PluginsPanel = lazy(() => import("@/features/tools/PluginsPanel").then((m) => ({ default: m.PluginsPanel })))
const SessionsPanel = lazy(() => import("@/features/sessions/SessionsPanel").then((m) => ({ default: m.SessionsPanel })))

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: false, staleTime: 1000 },
  },
})

function useTheme() {
  const [isDark, setIsDark] = useState(() => {
    if (typeof window === "undefined") return true
    return document.documentElement.classList.contains("dark")
  })

  useEffect(() => {
    const root = document.documentElement
    root.classList.toggle("dark", isDark)
    root.classList.toggle("light", !isDark)
  }, [isDark])

  return { isDark, toggle: () => setIsDark((d) => !d) }
}

const TOOL_VIEWS: NavView[] = [
  { id: "traffic", label: "Traffic", icon: "Activity" },
  { id: "breakpoints", label: "Breakpoints", icon: "Pause" },
  { id: "throttle", label: "Throttle", icon: "Gauge" },
  { id: "mocks", label: "Mocks", icon: "Theater" },
  { id: "rewrites", label: "Rewrites", icon: "Pencil" },
  { id: "replay", label: "Replay", icon: "RotateCcw" },
  { id: "grpc", label: "gRPC", icon: "Zap" },
  { id: "scripts", label: "Scripts", icon: "Code" },
  { id: "plugins", label: "Plugins", icon: "Puzzle" },
  { id: "sessions", label: "Sessions", icon: "FolderTree" },
]

function PanelFallback() {
  return (
    <div className="flex h-full items-center justify-center text-muted-foreground">
      <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading…
    </div>
  )
}

export default function App() {
  const { isDark, toggle } = useTheme()
  const [activeView, setActiveView] = useState<NavView["id"]>("traffic")

  return (
    <QueryClientProvider client={queryClient}>
      <div className="flex h-full flex-col bg-background text-foreground">
        <AppHeader isDark={isDark} onToggleTheme={toggle} />
        <div className="flex min-h-0 flex-1">
          <NavRail views={TOOL_VIEWS} activeView={activeView} onSelect={setActiveView} />
          <main className="min-w-0 flex-1 overflow-hidden">
            <Suspense fallback={<PanelFallback />}>
              {activeView === "traffic" && <TrafficView />}
              {activeView === "breakpoints" && <BreakpointsPanel />}
              {activeView === "throttle" && <ThrottlePanel />}
              {activeView === "mocks" && <MocksPanel />}
              {activeView === "rewrites" && <RewritesPanel />}
              {activeView === "replay" && <ReplayPanel selectedEntry={null} />}
              {activeView === "grpc" && <GrpcPanel />}
              {activeView === "scripts" && <ScriptsPanel />}
              {activeView === "plugins" && <PluginsPanel />}
              {activeView === "sessions" && <SessionsPanel />}
            </Suspense>
          </main>
        </div>
      </div>
      <Toaster />
    </QueryClientProvider>
  )
}
