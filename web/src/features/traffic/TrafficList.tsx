import { useState, useMemo, useCallback, memo, useEffect, useRef } from "react"
import { useVirtualizer } from "@tanstack/react-virtual"
import { cn } from "@/lib/utils"
import { ArrowUp, ArrowDown, ArrowUpDown } from "lucide-react"
import { Checkbox } from "@/components/ui/checkbox"
import type { TrafficEntry } from "@/types/traffic"

type SortField = "timestamp" | "method" | "status" | "path" | "duration" | "size"
type SortDirection = "asc" | "desc"
type ResizableCol = "method" | "protocol" | "domain" | "status" | "duration" | "size" | "timestamp"

interface TrafficListProps {
  traffic: TrafficEntry[]
  selectedId: string | null
  onSelect: (id: string) => void
  selectedIds?: Set<string>
  onToggleSelect?: (id: string) => void
  onSelectAll?: () => void
}

const DEFAULT_COL_WIDTHS: Record<ResizableCol, number> = {
  method: 60,
  protocol: 54,
  domain: 140,
  status: 48,
  duration: 58,
  size: 54,
  timestamp: 70,
}

const ROW_HEIGHT = 26

interface ColHeaderProps {
  label: string
  field?: SortField
  sortField: SortField
  sortDirection: SortDirection
  onSort: (f: SortField) => void
  width?: number
  flex?: boolean
  align?: "left" | "right"
  onResizeStart?: (e: React.MouseEvent) => void
}

const ColHeader = memo(function ColHeader({
  label,
  field,
  sortField,
  sortDirection,
  onSort,
  width,
  flex = false,
  align = "left",
  onResizeStart,
}: ColHeaderProps) {
  const isActive = field && sortField === field
  return (
    <div
      className={cn(
        "group relative flex shrink-0 select-none items-center",
        flex && "min-w-0 flex-1",
      )}
      style={!flex && width !== undefined ? { width } : undefined}
    >
      <button
        className={cn(
          "flex h-7 w-full items-center gap-1 overflow-hidden px-1 text-2xs font-medium text-muted-foreground hover:text-foreground",
          align === "right" && "justify-end",
        )}
        onClick={() => field && onSort(field)}
        disabled={!field}
      >
        <span className="truncate">{label}</span>
        {field &&
          (isActive ? (
            sortDirection === "asc" ? (
              <ArrowUp className="h-3 w-3 shrink-0" />
            ) : (
              <ArrowDown className="h-3 w-3 shrink-0" />
            )
          ) : (
            <ArrowUpDown className="h-3 w-3 shrink-0 opacity-30" />
          ))}
      </button>
      {onResizeStart && (
        <div
          className="absolute bottom-1 right-0 top-1 w-1 cursor-col-resize rounded bg-border opacity-0 hover:bg-primary group-hover:opacity-100"
          onMouseDown={onResizeStart}
        />
      )}
    </div>
  )
})

export function TrafficList({
  traffic,
  selectedId,
  onSelect,
  selectedIds,
  onToggleSelect,
  onSelectAll,
}: TrafficListProps) {
  const [sortField, setSortField] = useState<SortField>("timestamp")
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc")
  const [colWidths, setColWidths] = useState<Record<ResizableCol, number>>(DEFAULT_COL_WIDTHS)
  const resizeRef = useRef<{ col: ResizableCol; startX: number; startWidth: number } | null>(null)
  const scrollRef = useRef<HTMLDivElement>(null)

  const startResize = useCallback(
    (col: ResizableCol, e: React.MouseEvent) => {
      e.preventDefault()
      e.stopPropagation()
      resizeRef.current = { col, startX: e.clientX, startWidth: colWidths[col] }
    },
    [colWidths],
  )

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      if (!resizeRef.current) return
      const { col, startX, startWidth } = resizeRef.current
      const newWidth = Math.max(40, startWidth + e.clientX - startX)
      setColWidths((prev) => ({ ...prev, [col]: newWidth }))
    }
    const onMouseUp = () => {
      resizeRef.current = null
    }
    document.addEventListener("mousemove", onMouseMove)
    document.addEventListener("mouseup", onMouseUp)
    return () => {
      document.removeEventListener("mousemove", onMouseMove)
      document.removeEventListener("mouseup", onMouseUp)
    }
  }, [])

  const handleSort = useCallback(
    (field: SortField) => {
      if (sortField === field) {
        setSortDirection((prev) => (prev === "asc" ? "desc" : "asc"))
      } else {
        setSortField(field)
        setSortDirection("desc")
      }
    },
    [sortField],
  )

  const sortedTraffic = useMemo(() => {
    return [...traffic].sort((a, b) => {
      let cmp = 0
      switch (sortField) {
        case "timestamp":
          cmp = new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
          break
        case "method":
          cmp = a.request.method.localeCompare(b.request.method)
          break
        case "status":
          cmp = (a.response?.status_code || 0) - (b.response?.status_code || 0)
          break
        case "path":
          cmp = a.request.path.localeCompare(b.request.path)
          break
        case "duration":
          cmp = (a.response?.duration_ms || 0) - (b.response?.duration_ms || 0)
          break
        case "size":
          cmp = calculateSize(a) - calculateSize(b)
          break
      }
      return sortDirection === "asc" ? cmp : -cmp
    })
  }, [traffic, sortField, sortDirection])

  const virtualizer = useVirtualizer({
    count: sortedTraffic.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 16,
  })

  if (traffic.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-muted-foreground">
        <div>
          <p className="mb-1 text-xs">No traffic captured yet</p>
          <p className="text-2xs">Configure your browser or app to use localhost:8888 as proxy</p>
        </div>
      </div>
    )
  }

  const headerProps = { sortField, sortDirection, onSort: handleSort }
  const allChecked = !!selectedIds && selectedIds.size === traffic.length && traffic.length > 0

  return (
    <div className="flex h-full flex-col" role="list" aria-label="Traffic entries">
      {/* Column Headers */}
      <div className="flex shrink-0 items-center border-b border-border bg-muted/30 px-2">
        {onSelectAll && (
          <div className="flex w-7 shrink-0 items-center justify-center py-1.5">
            <Checkbox checked={allChecked} onCheckedChange={onSelectAll} aria-label="Select all" />
          </div>
        )}
        <ColHeader {...headerProps} label="Method" field="method" width={colWidths.method} onResizeStart={(e) => startResize("method", e)} />
        <ColHeader {...headerProps} label="Proto" width={colWidths.protocol} onResizeStart={(e) => startResize("protocol", e)} />
        <ColHeader {...headerProps} label="Domain" width={colWidths.domain} onResizeStart={(e) => startResize("domain", e)} />
        <ColHeader {...headerProps} label="Path" field="path" flex />
        <ColHeader {...headerProps} label="Status" field="status" width={colWidths.status} align="right" onResizeStart={(e) => startResize("status", e)} />
        <ColHeader {...headerProps} label="Time" field="duration" width={colWidths.duration} onResizeStart={(e) => startResize("duration", e)} />
        <ColHeader {...headerProps} label="Size" field="size" width={colWidths.size} align="right" onResizeStart={(e) => startResize("size", e)} />
        <ColHeader {...headerProps} label="When" field="timestamp" width={colWidths.timestamp} align="right" onResizeStart={(e) => startResize("timestamp", e)} />
      </div>

      {/* Virtualized list */}
      <div ref={scrollRef} className="flex-1 overflow-auto">
        <div style={{ height: virtualizer.getTotalSize(), width: "100%", position: "relative" }}>
          {virtualizer.getVirtualItems().map((virtualRow) => {
            const entry = sortedTraffic[virtualRow.index]
            return (
              <div
                key={entry.id}
                data-index={virtualRow.index}
                ref={virtualizer.measureElement}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              >
                <TrafficListItem
                  entry={entry}
                  isSelected={entry.id === selectedId}
                  onClick={() => onSelect(entry.id)}
                  colWidths={colWidths}
                  isChecked={selectedIds?.has(entry.id)}
                  onToggleCheck={onToggleSelect ? () => onToggleSelect(entry.id) : undefined}
                />
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}

interface TrafficListItemProps {
  entry: TrafficEntry
  isSelected: boolean
  onClick: () => void
  colWidths: Record<ResizableCol, number>
  isChecked?: boolean
  onToggleCheck?: () => void
}

const TrafficListItem = memo(function TrafficListItem({
  entry,
  isSelected,
  onClick,
  colWidths,
  isChecked,
  onToggleCheck,
}: TrafficListItemProps) {
  const methodClass = `method-${entry.request.method.toLowerCase()}`
  const statusClass = entry.response
    ? `status-${Math.floor(entry.response.status_code / 100)}xx`
    : ""
  const isPassthrough = entry.is_passthrough === true

  const time = new Date(entry.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })
  const size = entry.response ? formatSize(calculateSize(entry)) : "—"
  const duration = entry.response ? `${entry.response.duration_ms}ms` : "—"
  const protocol = entry.request.http_version
    || (entry.request.url.startsWith("https://") ? "HTTPS" : "HTTP")

  return (
    <div
      className={cn(
        "flex cursor-pointer items-center px-2 text-2xs transition-colors hover:bg-muted/40",
        isSelected && "bg-primary/10 hover:bg-primary/15",
        isPassthrough && "opacity-70",
      )}
      style={{ height: ROW_HEIGHT }}
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault()
          onClick()
        }
      }}
      aria-selected={isSelected}
    >
      {onToggleCheck && (
        <div
          className="flex w-7 shrink-0 items-center justify-center"
          onClick={(e) => {
            e.stopPropagation()
            onToggleCheck()
          }}
        >
          <Checkbox checked={isChecked || false} onCheckedChange={onToggleCheck} onClick={(e) => e.stopPropagation()} />
        </div>
      )}
      <span className={cn("shrink-0 truncate font-mono font-semibold", methodClass)} style={{ width: colWidths.method }}>
        {entry.request.method}
      </span>
      <span className="shrink-0 px-1 text-2xs text-muted-foreground" style={{ width: colWidths.protocol }}>
        {protocol}
      </span>
      <span
        className="shrink-0 truncate px-1 text-2xs text-muted-foreground"
        style={{ width: colWidths.domain }}
        title={isPassthrough ? `${entry.request.host} (SSL passthrough)` : entry.request.host}
      >
        {isPassthrough && (
          <span className="mr-1 inline-block rounded bg-amber-500/20 px-1 py-px text-2xs font-semibold text-amber-600 dark:text-amber-400" title="SSL Passthrough">
            PT
          </span>
        )}
        {entry.request.host}
      </span>
      <span className="min-w-0 flex-1 truncate px-1 font-mono text-2xs" title={entry.request.path}>
        {entry.request.path}
      </span>
      <span className={cn("shrink-0 text-right font-mono", isPassthrough ? "text-amber-600 dark:text-amber-400" : statusClass)} style={{ width: colWidths.status }}>
        {isPassthrough ? "PASS" : (entry.response?.status_code || "—")}
      </span>
      <span className="shrink-0 text-right text-2xs text-muted-foreground" style={{ width: colWidths.duration }}>
        {duration}
      </span>
      <span className="shrink-0 text-right text-2xs text-muted-foreground" style={{ width: colWidths.size }}>
        {size}
      </span>
      <span className="shrink-0 text-right text-2xs text-muted-foreground" style={{ width: colWidths.timestamp }}>
        {time}
      </span>
    </div>
  )
})

function calculateSize(entry: TrafficEntry): number {
  const reqSize = entry.request_size ?? entry.request.body?.length ?? 0
  const resSize = entry.response_size ?? entry.response?.body?.length ?? 0
  return reqSize + resSize
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
}
