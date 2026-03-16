import { useState, useMemo, useCallback, useEffect, useRef } from "react";
import {
  useTraffic,
  useTrafficCount,
  useClearTraffic,
} from "@/hooks/useTraffic";
import { TrafficList } from "./TrafficList";
import { TrafficDetail } from "./TrafficDetail";
import { TrafficToolbar } from "./TrafficToolbar";
import { ToolsSidebar } from "./ToolsSidebar";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import {
  Trash2,
  RefreshCw,
  Wrench,
  Keyboard,
  Download,
  ChevronDown,
  Wifi,
  WifiOff,
  Loader2,
} from "lucide-react";
import type { TrafficEntry } from "@/types/traffic";
import type { ActiveFilter } from "@/types/filters";
import { applyFilters } from "@/types/filters";
import { cn } from "@/lib/utils";

const STORAGE_KEY_LIST_WIDTH = "madhyamas-traffic-list-width";
const STORAGE_KEY_TOOLS_WIDTH = "madhyamas-tools-width";
const DEFAULT_LIST_WIDTH = 40;
const DEFAULT_TOOLS_WIDTH = 400;
const MIN_LIST_WIDTH = 20;
const MAX_LIST_WIDTH = 60;
const MIN_TOOLS_WIDTH = 300;
const MAX_TOOLS_WIDTH = 600;

export function TrafficView() {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [activeFilters, setActiveFilters] = useState<ActiveFilter[]>([]);
  const [showTools, setShowTools] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [listWidth, setListWidth] = useState(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem(STORAGE_KEY_LIST_WIDTH);
      return saved ? parseInt(saved, 10) : DEFAULT_LIST_WIDTH;
    }
    return DEFAULT_LIST_WIDTH;
  });
  const [toolsWidth, setToolsWidth] = useState(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem(STORAGE_KEY_TOOLS_WIDTH);
      return saved ? parseInt(saved, 10) : DEFAULT_TOOLS_WIDTH;
    }
    return DEFAULT_TOOLS_WIDTH;
  });

  const {
    data: traffic,
    isLoading,
    refetch,
    connectionInfo,
    isWebSocketMode,
    setWebSocketMode,
  } = useTraffic(search ? { filter: { search } } : undefined);
  const { data: count } = useTrafficCount();
  const clearTraffic = useClearTraffic();

  // Derive connection status for UI
  const wsConnected = connectionInfo?.state === "connected";
  const wsReconnecting = connectionInfo?.state === "reconnecting";

  const containerRef = useRef<HTMLDivElement>(null);
  const [isResizingList, setIsResizingList] = useState(false);
  const [isResizingTools, setIsResizingTools] = useState(false);

  const filteredTraffic = useMemo(() => {
    if (!traffic) return [];
    return applyFilters(traffic, activeFilters);
  }, [traffic, activeFilters]);

  const selectedEntry = useMemo(() => {
    if (!traffic || !selectedId) return null;
    return traffic.find((t: TrafficEntry) => t.id === selectedId) || null;
  }, [traffic, selectedId]);

  const handleClear = useCallback(
    async (clearAll: boolean) => {
      if (clearAll) {
        if (!confirm("Clear all traffic?")) return;
        clearTraffic.mutate();
        setSelectedId(null);
        setSelectedIds(new Set());
      } else {
        if (selectedIds.size === 0) return;
        if (
          !confirm(
            `Clear ${selectedIds.size} selected ${selectedIds.size === 1 ? "entry" : "entries"}?`,
          )
        )
          return;

        try {
          const ids = Array.from(selectedIds).join(",");
          const response = await fetch(
            `/api/traffic/clear?ids=${encodeURIComponent(ids)}`,
            { method: "POST" },
          );
          if (!response.ok) throw new Error("Failed to clear selected traffic");

          // Clear selection and refetch
          setSelectedIds(new Set());
          if (selectedIds.has(selectedId || "")) {
            setSelectedId(null);
          }
          refetch();
        } catch (error) {
          console.error("Failed to clear selected traffic:", error);
        }
      }
    },
    [clearTraffic, selectedIds, selectedId, refetch],
  );

  const handleToggleSelect = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  const handleSelectAll = useCallback(() => {
    if (selectedIds.size === filteredTraffic.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredTraffic.map((t) => t.id)));
    }
  }, [filteredTraffic, selectedIds.size]);

  const handleExportHar = useCallback(
    async (exportAll: boolean) => {
      try {
        let url = "/api/export/har";
        if (!exportAll && selectedIds.size > 0) {
          const ids = Array.from(selectedIds).join(",");
          url = `/api/export/har?ids=${encodeURIComponent(ids)}`;
        }

        const response = await fetch(url);
        if (!response.ok) throw new Error("Failed to export HAR");

        const har = await response.json();
        const blob = new Blob([JSON.stringify(har, null, 2)], {
          type: "application/json",
        });
        const blobUrl = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = blobUrl;
        a.download = `madhyamas-${new Date().toISOString().slice(0, 10)}.har`;
        a.click();
        URL.revokeObjectURL(blobUrl);
      } catch (error) {
        console.error("Failed to export HAR:", error);
      }
    },
    [selectedIds],
  );

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger shortcuts when typing in inputs
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      switch (e.key.toLowerCase()) {
        case "r":
          if (!e.metaKey && !e.ctrlKey) {
            e.preventDefault();
            refetch();
          }
          break;
        case "c":
          if (!e.metaKey && !e.ctrlKey) {
            e.preventDefault();
            handleClear(true);
          }
          break;
        case "t":
          e.preventDefault();
          setShowTools((prev) => !prev);
          break;
        case "?":
          e.preventDefault();
          setShowShortcuts((prev) => !prev);
          break;
        case "escape":
          setShowShortcuts(false);
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [refetch, handleClear]);

  // List panel resize handlers
  const handleListMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizingList(true);
  }, []);

  const handleToolsMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizingTools(true);
  }, []);

  useEffect(() => {
    if (!isResizingList && !isResizingTools) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();

      if (isResizingList) {
        const percent = ((e.clientX - rect.left) / rect.width) * 100;
        const newWidth = Math.min(
          MAX_LIST_WIDTH,
          Math.max(MIN_LIST_WIDTH, percent),
        );
        setListWidth(newWidth);
        localStorage.setItem(STORAGE_KEY_LIST_WIDTH, newWidth.toString());
      } else if (isResizingTools && showTools) {
        const percent = ((rect.right - e.clientX) / rect.width) * 100;
        const newWidth = Math.min(
          MAX_TOOLS_WIDTH,
          Math.max(MIN_TOOLS_WIDTH, rect.width * (percent / 100)),
        );
        setToolsWidth(newWidth);
        localStorage.setItem(STORAGE_KEY_TOOLS_WIDTH, newWidth.toString());
      }
    };

    const handleMouseUp = () => {
      setIsResizingList(false);
      setIsResizingTools(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizingList, isResizingTools, showTools]);

  return (
    <div className="h-full flex flex-col">
      <TrafficToolbar
        search={search}
        onSearchChange={setSearch}
        filters={activeFilters}
        onFiltersChange={setActiveFilters}
        count={filteredTraffic.length}
      />

      <div className="flex-1 flex overflow-hidden border-t" ref={containerRef}>
        <div className="flex-1 flex flex-col min-w-0">
          <div className="px-3 py-2 border-b bg-muted/50 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-sm text-muted-foreground">
                {count ?? 0} requests
              </span>
              {/* WebSocket Connection Status */}
              {isWebSocketMode && (
                <div className="flex items-center gap-1.5">
                  {wsReconnecting ? (
                    <>
                      <Loader2 className="h-3.5 w-3.5 text-yellow-500 animate-spin" />
                      <span className="text-xs text-yellow-500">
                        Reconnecting...
                      </span>
                    </>
                  ) : wsConnected ? (
                    <>
                      <Wifi className="h-3.5 w-3.5 text-green-500" />
                      <span className="text-xs text-green-500">Live</span>
                    </>
                  ) : (
                    <>
                      <WifiOff className="h-3.5 w-3.5 text-red-500" />
                      <span className="text-xs text-red-500">Disconnected</span>
                    </>
                  )}
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-2 text-xs"
                    onClick={() => setWebSocketMode(!isWebSocketMode)}
                    title={
                      isWebSocketMode
                        ? "Switch to polling mode"
                        : "Switch to WebSocket mode"
                    }
                  >
                    {isWebSocketMode ? "WS" : "Poll"}
                  </Button>
                </div>
              )}
              {!isWebSocketMode && (
                <div className="flex items-center gap-1.5">
                  <span className="text-xs text-muted-foreground">Polling</span>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-2 text-xs"
                    onClick={() => setWebSocketMode(true)}
                    title="Switch to WebSocket mode"
                  >
                    Enable Live
                  </Button>
                </div>
              )}
            </div>
            <div className="flex gap-2">
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm">
                    <Download className="h-4 w-4 mr-1" />
                    Export
                    <ChevronDown className="h-3 w-3 ml-1" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem
                    onClick={() => handleExportHar(false)}
                    disabled={selectedIds.size === 0}
                  >
                    Export Selected ({selectedIds.size})
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => handleExportHar(true)}>
                    Export All ({filteredTraffic.length})
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
              <Button
                variant={showTools ? "default" : "ghost"}
                size="sm"
                onClick={() => setShowTools(!showTools)}
                title="Toggle Tools Panel (T)"
              >
                <Wrench className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowShortcuts(!showShortcuts)}
                title="Keyboard Shortcuts (?)"
              >
                <Keyboard className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => refetch()}
                title="Refresh (R)"
              >
                <RefreshCw className="h-4 w-4" />
              </Button>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={clearTraffic.isPending}
                  >
                    <Trash2 className="h-4 w-4 mr-1" />
                    Clear
                    <ChevronDown className="h-3 w-3 ml-1" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem
                    onClick={() => handleClear(false)}
                    disabled={selectedIds.size === 0}
                  >
                    Clear Selected ({selectedIds.size})
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => handleClear(true)}>
                    Clear All ({filteredTraffic.length})
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          <div className="flex-1 flex overflow-hidden">
            {/* Traffic List Panel */}
            <div
              style={{ width: `${listWidth}%` }}
              className="flex flex-col min-w-[200px]"
            >
              <div className="flex-1 overflow-auto">
                {isLoading ? (
                  <div className="flex items-center justify-center h-full text-muted-foreground">
                    Loading...
                  </div>
                ) : (
                  <TrafficList
                    traffic={filteredTraffic}
                    selectedId={selectedId}
                    onSelect={setSelectedId}
                    selectedIds={selectedIds}
                    onToggleSelect={handleToggleSelect}
                    onSelectAll={handleSelectAll}
                  />
                )}
              </div>
            </div>

            {/* Resize Handle for List */}
            <div
              className={cn(
                "w-1 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0",
                isResizingList && "bg-primary",
              )}
              onMouseDown={handleListMouseDown}
            />

            {/* Traffic Detail Panel */}
            <div className="flex-1 flex flex-col min-w-0">
              {selectedEntry ? (
                <TrafficDetail entry={selectedEntry} />
              ) : (
                <div className="flex-1 flex items-center justify-center text-muted-foreground">
                  Select a request to view details
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Tools Sidebar with Resize */}
        {showTools && (
          <>
            {/* Resize Handle for Tools */}
            <div
              className={cn(
                "w-1 cursor-col-resize hover:bg-primary/50 transition-colors flex-shrink-0",
                isResizingTools && "bg-primary",
              )}
              onMouseDown={handleToolsMouseDown}
            />
            <div
              style={{ width: `${toolsWidth}px` }}
              className="border-l flex-shrink-0"
            >
              <ToolsSidebar
                selectedEntry={selectedEntry}
                isOpen={showTools}
                onClose={() => setShowTools(false)}
              />
            </div>
          </>
        )}
      </div>

      {/* Keyboard Shortcuts Modal */}
      {showShortcuts && (
        <div
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={() => setShowShortcuts(false)}
        >
          <div
            className="bg-background border rounded-lg shadow-lg p-6 max-w-md w-full mx-4"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="text-lg font-semibold mb-4">Keyboard Shortcuts</h2>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Refresh traffic</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">
                  R
                </kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Clear all traffic</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">
                  C
                </kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  Toggle tools panel
                </span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">
                  T
                </kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Show shortcuts</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">
                  ?
                </kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Close modal</span>
                <kbd className="px-2 py-1 bg-muted rounded text-xs font-mono">
                  Esc
                </kbd>
              </div>
            </div>
            <Button
              className="mt-4 w-full"
              onClick={() => setShowShortcuts(false)}
            >
              Close
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
