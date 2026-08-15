import { useCallback, useEffect, useState, lazy, Suspense } from "react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Moon,
  Sun,
  Settings,
  SlidersHorizontal,
  CircleDot,
  CircleSlash,
  Loader2,
  HelpCircle,
  Github,
  BookOpen,
  ExternalLink,
  Database,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { apiGet, apiPost } from "@/lib/api/client"
import { useCaptureStats } from "@/hooks/useCaptureStats"
import { UserMenu } from "@/features/shell/UserMenu"
import type { TierInfo } from "@/contexts/TierContext"

// Lazy-load heavy dialogs that are only opened on demand.
const CertificateHelper = lazy(() =>
  import("@/features/cert/CertificateHelper").then((m) => ({ default: m.CertificateHelper })),
)
const ConfigDialog = lazy(() =>
  import("@/features/config/ConfigDialog").then((m) => ({ default: m.ConfigDialog })),
)

interface AppHeaderProps {
  isDark: boolean
  onToggleTheme: () => void
  tierInfo?: TierInfo | null
}

const DOCS_URL = "https://shristilabs.github.io/madhyamas/"

export function AppHeader({ isDark, onToggleTheme, tierInfo }: AppHeaderProps) {
  const isEnterprise = tierInfo?.tier === "enterprise"
  const [proxyAddress, setProxyAddress] = useState("localhost:8888")
  const [captureEnabled, setCaptureEnabled] = useState(true)
  const [captureLoading, setCaptureLoading] = useState(false)
  const { data: captureStats } = useCaptureStats({ enabled: captureEnabled })

  useEffect(() => {
    apiGet<{ capture_enabled?: boolean }>("/capture")
      .then((d) => setCaptureEnabled(d.capture_enabled ?? true))
      .catch(() => {})
  }, [])

  useEffect(() => {
    apiGet<{ host?: string; proxy_port?: number }>("/config")
      .then((c) => {
        const host = c.host || "localhost"
        const port = c.proxy_port || 8888
        setProxyAddress(`${host}:${port}`)
      })
      .catch(() => {})
  }, [])

  const handleToggleCapture = useCallback(async () => {
    setCaptureLoading(true)
    try {
      const data = await apiPost<{ capture_enabled: boolean }>("/capture/toggle")
      setCaptureEnabled(data.capture_enabled)
    } catch {
      // ignore
    } finally {
      setCaptureLoading(false)
    }
  }, [])

  return (
    <header className="flex h-11 shrink-0 items-center justify-between border-b border-border bg-card px-3">
      {/* Brand */}
      <div className="flex items-center gap-2">
        <div className="flex h-6 w-6 items-center justify-center rounded bg-primary text-[10px] font-bold text-primary-foreground">
          M
        </div>
        <span className="text-sm font-semibold tracking-tight">Madhyamas</span>
        {isEnterprise ? (
          <Badge variant="success" className="text-2xs">
            Enterprise
          </Badge>
        ) : (
          <span className="hidden font-mono text-2xs text-muted-foreground sm:inline">
            HTTP Debugging Proxy
          </span>
        )}
      </div>

      {/* Right controls */}
      <div className="flex items-center gap-1.5">
        {/* Proxy address */}
        <TooltipProvider delayDuration={300}>
          <Tooltip>
            <TooltipTrigger asChild>
              <span className="hidden rounded border border-border bg-muted/40 px-2 py-0.5 font-mono text-2xs text-muted-foreground md:inline">
                {proxyAddress}
              </span>
            </TooltipTrigger>
            <TooltipContent>Proxy listen address</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        {/* Capture toggle */}
        <button
          onClick={handleToggleCapture}
          disabled={captureLoading}
          title={
            captureEnabled
              ? "Recording — click for passthrough"
              : "Passthrough — click to resume recording"
          }
          className={cn(
            "inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-2xs font-medium transition-colors select-none",
            captureEnabled
              ? "border-success/30 bg-success/10 text-success hover:bg-success/20"
              : "border-warning/30 bg-warning/10 text-warning hover:bg-warning/20",
            captureLoading && "opacity-60",
          )}
        >
          {captureLoading ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : captureEnabled ? (
            <CircleDot className="h-3 w-3 animate-pulse-dot" />
          ) : (
            <CircleSlash className="h-3 w-3" />
          )}
          <span className="hidden sm:inline">
            {captureEnabled ? "Recording" : "Passthrough"}
          </span>
        </button>

        {/* Recording quota indicator */}
        {captureStats && captureStats.max_entries > 0 && (
          <TooltipProvider delayDuration={300}>
            <Tooltip>
              <TooltipTrigger asChild>
                <span
                  className={cn(
                    "hidden items-center gap-1 rounded border px-2 py-0.5 font-mono text-2xs select-none md:inline-flex",
                    captureStats.entry_count / captureStats.max_entries > 0.8
                      ? "border-warning/30 bg-warning/10 text-warning"
                      : "border-border bg-muted/40 text-muted-foreground",
                  )}
                >
                  <Database className="h-3 w-3" />
                  {captureStats.entry_count}/{captureStats.max_entries}
                </span>
              </TooltipTrigger>
              <TooltipContent>
                {captureStats.entry_count} of {captureStats.max_entries} entries
                {captureStats.max_total_size_bytes > 0 && (
                  <>
                    {" · "}
                    {formatBytes(captureStats.total_size_bytes)} /{" "}
                    {formatBytes(captureStats.max_total_size_bytes)}
                  </>
                )}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}

        <Suspense fallback={null}>
          <CertificateHelper
            trigger={
              <Button variant="ghost" size="sm" title="Certificate setup">
                <Settings className="h-3.5 w-3.5" />
                <span className="hidden lg:inline">Setup</span>
              </Button>
            }
          />
        </Suspense>

        <Suspense fallback={null}>
          <ConfigDialog
            trigger={
              <Button variant="ghost" size="sm" title="Configuration">
                <SlidersHorizontal className="h-3.5 w-3.5" />
                <span className="hidden lg:inline">Config</span>
              </Button>
            }
          />
        </Suspense>

        {/* Help / links */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon-sm" title="Help">
              <HelpCircle className="h-3.5 w-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>Links</DropdownMenuLabel>
            <DropdownMenuItem onClick={() => window.open(DOCS_URL, "_blank")}>
              <BookOpen className="mr-2 h-3.5 w-3.5" /> Documentation
              <ExternalLink className="ml-auto h-3 w-3 text-muted-foreground" />
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => window.open("https://windsurf.com/support", "_blank")}>
              <HelpCircle className="mr-2 h-3.5 w-3.5" /> Support
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onClick={() => window.open("https://github.com", "_blank")}>
              <Github className="mr-2 h-3.5 w-3.5" /> Source
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        {/* User menu (enterprise only) */}
        {isEnterprise && <UserMenu />}

        {/* Theme toggle */}
        <Button variant="ghost" size="icon-sm" onClick={onToggleTheme} title="Toggle theme">
          {isDark ? <Sun className="h-3.5 w-3.5" /> : <Moon className="h-3.5 w-3.5" />}
        </Button>
      </div>
    </header>
  )
}

/** Format a byte count as a human-readable string (e.g. "1.2 MB"). */
function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const units = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  const val = bytes / Math.pow(1024, i)
  return `${val.toFixed(val < 10 ? 1 : 0)} ${units[i]}`
}
