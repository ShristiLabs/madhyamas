import { useState, useMemo, useCallback, useEffect, useRef } from "react"
import {
  useTraffic,
  useTrafficCount,
  useClearTraffic,
  useTrafficEntry,
  useImportHar,
} from "@/hooks/useTraffic"
import { TrafficList } from "./TrafficList"
import { TrafficDetail } from "./TrafficDetail"
import { TrafficToolbar } from "./TrafficToolbar"
import { FocusPanel } from "./FocusPanel"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Trash2,
  RefreshCw,
  Keyboard,
  Download,
  Upload,
  ChevronDown,
  Wifi,
  WifiOff,
  Loader2,
  X,
  Star,
} from "lucide-react"
import type { TrafficEntry } from "@/types/traffic"
import type { ActiveFilter } from "@/types/filters"
import { applyFilters } from "@/types/filters"
import { cn } from "@/lib/utils"
import { apiPostVoid, apiGet } from "@/lib/api/client"
import { useFocusHosts, useAddFocusHost } from "@/lib/api/intercept"
import { hostMatchesAnyPattern } from "@/lib/focus"

const STORAGE_KEY_LIST_WIDTH = "madhyamas-next-list-width"
const DEFAULT_LIST_WIDTH = 40
const MIN_LIST_WIDTH = 22
const MAX_LIST_WIDTH = 60

export function TrafficView() {
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [search, setSearch] = useState("")
  const [activeFilters, setActiveFilters] = useState<ActiveFilter[]>([])
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [mobileDetailOpen, setMobileDetailOpen] = useState(false)
  const [showFocusPanel, setShowFocusPanel] = useState(false)
  const [showOnlyFocused, setShowOnlyFocused] = useState(false)

  const [listWidth, setListWidth] = useState(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem(STORAGE_KEY_LIST_WIDTH)
      return saved ? parseInt(saved, 10) : DEFAULT_LIST_WIDTH
    }
    return DEFAULT_LIST_WIDTH
  })

  const {
    data: traffic,
    isLoading,
    refetch,
    connectionInfo,
    isWebSocketMode,
    setWebSocketMode,
  } = useTraffic(search ? { filter: { search } } : undefined)
  const { data: count } = useTrafficCount()
  const clearTraffic = useClearTraffic()
  const importHar = useImportHar()
  const { data: focusHosts } = useFocusHosts()
  const addFocusHost = useAddFocusHost()

  const fileInputRef = useRef<HTMLInputElement>(null)

  const wsConnected = connectionInfo?.state === "connected"
  const wsReconnecting = connectionInfo?.state === "reconnecting"

  const containerRef = useRef<HTMLDivElement>(null)
  const [isResizingList, setIsResizingList] = useState(false)

  const filteredTraffic = useMemo(() => {
    if (!traffic) return []
    let result = applyFilters(traffic, activeFilters)
    if (showOnlyFocused && focusHosts && focusHosts.length > 0) {
      result = result.filter((t) => hostMatchesAnyPattern(t.request.host, focusHosts))
    }
    return result
  }, [traffic, activeFilters, showOnlyFocused, focusHosts])

  const listEntry = useMemo(() => {
    if (!traffic || !selectedId) return null
    return traffic.find((t: TrafficEntry) => t.id === selectedId) || null
  }, [traffic, selectedId])

  const { data: fullEntry } = useTrafficEntry(selectedId)
  const selectedEntry: TrafficEntry | null = fullEntry ?? listEntry ?? null

  const handleClear = useCallback(
    async (clearAll: boolean) => {
      if (clearAll) {
        if (!confirm("Clear all traffic?")) return
        clearTraffic.mutate()
        setSelectedId(null)
        setSelectedIds(new Set())
      } else {
        if (selectedIds.size === 0) return
        if (!confirm(`Clear ${selectedIds.size} selected ${selectedIds.size === 1 ? "entry" : "entries"}?`)) return
        try {
          const ids = Array.from(selectedIds).join(",")
          await apiPostVoid(`/traffic/clear?ids=${encodeURIComponent(ids)}`)
          setSelectedIds(new Set())
          if (selectedIds.has(selectedId || "")) setSelectedId(null)
          refetch()
        } catch (error) {
          console.error("Failed to clear selected traffic:", error)
        }
      }
    },
    [clearTraffic, selectedIds, selectedId, refetch],
  )

  const handleToggleSelect = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }, [])

  const handleSelectAll = useCallback(() => {
    if (selectedIds.size === filteredTraffic.length) setSelectedIds(new Set())
    else setSelectedIds(new Set(filteredTraffic.map((t) => t.id)))
  }, [filteredTraffic, selectedIds.size])

  const handleExportHar = useCallback(
    async (exportAll: boolean) => {
      try {
        let path = "/export/har"
        if (!exportAll && selectedIds.size > 0) {
          const ids = Array.from(selectedIds).join(",")
          path = `/export/har?ids=${encodeURIComponent(ids)}`
        }
        const har = await apiGet<unknown>(path)
        const blob = new Blob([JSON.stringify(har, null, 2)], { type: "application/json" })
        const blobUrl = URL.createObjectURL(blob)
        const a = document.createElement("a")
        a.href = blobUrl
        a.download = `madhyamas-${new Date().toISOString().slice(0, 10)}.har`
        a.click()
        URL.revokeObjectURL(blobUrl)
      } catch (error) {
        console.error("Failed to export HAR:", error)
      }
    },
    [selectedIds],
  )

  const handleImportHar = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0]
      if (!file) return
      try {
        const text = await file.text()
        const har = JSON.parse(text)
        importHar.mutate(
          { har, switchSession: true },
          {
            onSuccess: (result) => {
              const skippedMsg =
                result.skipped_count > 0 ? ` (${result.skipped_count} skipped)` : ""
              alert(`Imported ${result.imported_count} entries${skippedMsg} into a new session.`)
            },
            onError: (error) => {
              alert(`Failed to import HAR: ${error instanceof Error ? error.message : String(error)}`)
            },
          },
        )
      } catch (error) {
        alert(`Failed to read HAR file: ${error instanceof Error ? error.message : String(error)}`)
      } finally {
        // Reset the input so the same file can be selected again
        if (fileInputRef.current) fileInputRef.current.value = ""
      }
    },
    [importHar],
  )

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return
      switch (e.key.toLowerCase()) {
        case "r":
          if (!e.metaKey && !e.ctrlKey) {
            e.preventDefault()
            refetch()
          }
          break
        case "c":
          if (!e.metaKey && !e.ctrlKey) {
            e.preventDefault()
            handleClear(true)
          }
          break
        case "?":
          e.preventDefault()
          setShowShortcuts((prev) => !prev)
          break
        case "escape":
          setShowShortcuts(false)
          setMobileDetailOpen(false)
          break
      }
    }
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [refetch, handleClear])

  // List resize handler
  const handleListMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsResizingList(true)
  }, [])

  useEffect(() => {
    if (!isResizingList) return
    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return
      const rect = containerRef.current.getBoundingClientRect()
      const percent = ((e.clientX - rect.left) / rect.width) * 100
      const newWidth = Math.min(MAX_LIST_WIDTH, Math.max(MIN_LIST_WIDTH, percent))
      setListWidth(newWidth)
      localStorage.setItem(STORAGE_KEY_LIST_WIDTH, newWidth.toString())
    }
    const handleMouseUp = () => setIsResizingList(false)
    document.addEventListener("mousemove", handleMouseMove)
    document.addEventListener("mouseup", handleMouseUp)
    return () => {
      document.removeEventListener("mousemove", handleMouseMove)
      document.removeEventListener("mouseup", handleMouseUp)
    }
  }, [isResizingList])

  const handleSelect = useCallback((id: string) => {
    setSelectedId(id)
    setMobileDetailOpen(true)
  }, [])

  const handleFocusHost = useCallback(
    (host: string) => {
      if (!host) return
      addFocusHost.mutate(host)
    },
    [addFocusHost],
  )

  return (
    <div className="flex h-full flex-col">
      <TrafficToolbar
        search={search}
        onSearchChange={setSearch}
        filters={activeFilters}
        onFiltersChange={setActiveFilters}
        count={filteredTraffic.length}
      />

      <div className="flex flex-1 overflow-hidden border-t border-border" ref={containerRef}>
        <div className="flex min-w-0 flex-1 flex-col">
          {/* Sub-toolbar: count + connection + actions */}
          <div className="flex items-center justify-between border-b border-border bg-muted/30 px-2 py-1">
            <div className="flex items-center gap-2">
              <span className="text-2xs text-muted-foreground">{count ?? 0} req</span>
              {isWebSocketMode ? (
                <div className="flex items-center gap-1">
                  {wsReconnecting ? (
                    <>
                      <Loader2 className="h-3 w-3 animate-spin text-warning" />
                      <span className="text-2xs text-warning">Reconnecting…</span>
                    </>
                  ) : wsConnected ? (
                    <>
                      <Wifi className="h-3 w-3 text-success" />
                      <span className="text-2xs text-success">Live</span>
                    </>
                  ) : (
                    <>
                      <WifiOff className="h-3 w-3 text-destructive" />
                      <span className="text-2xs text-destructive">Off</span>
                    </>
                  )}
                  <Button variant="ghost" size="sm" className="h-5 px-1.5 text-2xs" onClick={() => setWebSocketMode(false)} title="Switch to polling">
                    Poll
                  </Button>
                </div>
              ) : (
                <div className="flex items-center gap-1">
                  <span className="text-2xs text-muted-foreground">Polling</span>
                  <Button variant="ghost" size="sm" className="h-5 px-1.5 text-2xs" onClick={() => setWebSocketMode(true)} title="Switch to live WebSocket">
                    Live
                  </Button>
                </div>
              )}
              <Button
                variant={showFocusPanel ? "default" : "ghost"}
                size="sm"
                className="h-5 px-1.5 text-2xs"
                onClick={() => setShowFocusPanel((prev) => !prev)}
                title="Toggle focus panel"
              >
                <Star className={cn("h-3 w-3", showFocusPanel && "fill-current")} />
                Focus
                {focusHosts && focusHosts.length > 0 && (
                  <span className="ml-0.5 rounded-full bg-muted px-1 text-2xs">
                    {focusHosts.length}
                  </span>
                )}
              </Button>
            </div>

            <div className="flex items-center gap-1">
              <input
                ref={fileInputRef}
                type="file"
                accept=".har,application/json"
                className="hidden"
                onChange={handleImportHar}
              />
              <Button
                variant="ghost"
                size="sm"
                title="Import HAR"
                disabled={importHar.isPending}
                onClick={() => fileInputRef.current?.click()}
              >
                {importHar.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Upload className="h-3.5 w-3.5" />
                )}
                <span className="hidden sm:inline">Import</span>
              </Button>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm" title="Export HAR">
                    <Download className="h-3.5 w-3.5" />
                    <span className="hidden sm:inline">Export</span>
                    <ChevronDown className="h-3 w-3" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={() => handleExportHar(false)} disabled={selectedIds.size === 0}>
                    Selected ({selectedIds.size})
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => handleExportHar(true)}>All ({filteredTraffic.length})</DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>

              <Button variant="ghost" size="icon-sm" onClick={() => setShowShortcuts(!showShortcuts)} title="Shortcuts (?)">
                <Keyboard className="h-3.5 w-3.5" />
              </Button>

              <Button variant="ghost" size="icon-sm" onClick={() => refetch()} title="Refresh (R)">
                <RefreshCw className="h-3.5 w-3.5" />
              </Button>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm" disabled={clearTraffic.isPending} title="Clear traffic">
                    <Trash2 className="h-3.5 w-3.5" />
                    <span className="hidden sm:inline">Clear</span>
                    <ChevronDown className="h-3 w-3" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={() => handleClear(false)} disabled={selectedIds.size === 0}>
                    Selected ({selectedIds.size})
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => handleClear(true)}>All ({filteredTraffic.length})</DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          {/* List + Detail (desktop: side-by-side; mobile: toggled) */}
          <div className="flex flex-1 overflow-hidden">
            {/* Traffic List Panel */}
            <div
              className={cn("flex min-w-0 flex-col", mobileDetailOpen && "hidden md:flex")}
              style={{ width: `${listWidth}%` }}
            >
              <div className="flex-1 overflow-hidden">
                {isLoading ? (
                  <div className="flex h-full items-center justify-center text-2xs text-muted-foreground">
                    <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> Loading…
                  </div>
                ) : (
                  <TrafficList
                    traffic={filteredTraffic}
                    selectedId={selectedId}
                    onSelect={handleSelect}
                    selectedIds={selectedIds}
                    onToggleSelect={handleToggleSelect}
                    onSelectAll={handleSelectAll}
                    focusHosts={focusHosts}
                    onFocusHost={handleFocusHost}
                  />
                )}
              </div>
            </div>

            {/* Resize Handle for List (desktop only) */}
            <div
              className={cn(
                "hidden w-px shrink-0 cursor-col-resize bg-border transition-colors hover:bg-primary md:block",
                isResizingList && "bg-primary",
                mobileDetailOpen && "block",
              )}
              onMouseDown={handleListMouseDown}
            />

            {/* Traffic Detail Panel */}
            <div className={cn("flex min-w-0 flex-1 flex-col", !mobileDetailOpen && "hidden md:flex")}>
              {selectedEntry ? (
                <>
                  {/* Mobile back button */}
                  <button
                    className="flex items-center gap-1 border-b border-border px-2 py-1 text-2xs text-muted-foreground md:hidden"
                    onClick={() => setMobileDetailOpen(false)}
                  >
                    <X className="h-3 w-3" /> Back to list
                  </button>
                  <TrafficDetail entry={selectedEntry} />
                </>
              ) : (
                <div className="flex flex-1 items-center justify-center text-2xs text-muted-foreground">
                  Select a request to view details
                </div>
              )}
            </div>

            {/* Focus Panel (optional sidebar) */}
            {showFocusPanel && (
              <>
                <div className="hidden w-px shrink-0 bg-border md:block" />
                <div className="hidden w-56 shrink-0 flex-col border-l border-border md:flex">
                  <FocusPanel
                    showOnlyFocused={showOnlyFocused}
                    onShowOnlyFocusedChange={setShowOnlyFocused}
                  />
                </div>
              </>
            )}
          </div>
        </div>
      </div>

      {/* Keyboard Shortcuts Modal */}
      {showShortcuts && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4" onClick={() => setShowShortcuts(false)}>
          <div className="w-full max-w-md rounded-md border border-border bg-popover p-4 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h2 className="mb-3 text-sm font-semibold">Keyboard Shortcuts</h2>
            <div className="space-y-1.5 text-xs">
              {[
                ["Refresh traffic", "R"],
                ["Clear all traffic", "C"],
                ["Show shortcuts", "?"],
                ["Close modal", "Esc"],
              ].map(([label, key]) => (
                <div key={key} className="flex justify-between">
                  <span className="text-muted-foreground">{label}</span>
                  <kbd className="rounded bg-muted px-1.5 py-0.5 font-mono text-2xs">{key}</kbd>
                </div>
              ))}
            </div>
            <Button className="mt-3 w-full" size="sm" onClick={() => setShowShortcuts(false)}>
              Close
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
