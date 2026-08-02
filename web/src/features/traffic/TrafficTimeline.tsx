import { useState, useMemo, useCallback, useRef, memo } from "react"
import { useVirtualizer } from "@tanstack/react-virtual"
import { cn } from "@/lib/utils"
import { Activity } from "lucide-react"
import type { TrafficEntry } from "@/types/traffic"

const ROW_HEIGHT = 26
const LABEL_WIDTH = 240
const AXIS_HEIGHT = 28
const MIN_BAR_WIDTH = 2

interface TrafficTimelineProps {
  traffic: TrafficEntry[]
  selectedId?: string | null
  onSelect: (id: string) => void
}

function statusBarClass(statusCode?: number): string {
  if (!statusCode) return "bg-muted-foreground/40"
  const cls = Math.floor(statusCode / 100)
  switch (cls) {
    case 2:
      return "bg-success/70"
    case 3:
      return "bg-primary/70"
    case 4:
      return "bg-warning/70"
    case 5:
      return "bg-destructive/70"
    default:
      return "bg-muted-foreground/40"
  }
}

function formatDuration(ms: number): string {
  if (ms < 1) return "<1ms"
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(2)}s`
}

function formatRelative(ms: number): string {
  if (ms < 1) return "0ms"
  if (ms < 1000) return `${Math.round(ms)}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function generateTicks(range: number, count: number): number[] {
  if (range <= 0) return [0]
  const step = range / count
  const magnitude = Math.pow(10, Math.floor(Math.log10(step)))
  const normalized = step / magnitude
  let niceStep: number
  if (normalized < 1.5) niceStep = 1 * magnitude
  else if (normalized < 3) niceStep = 2 * magnitude
  else if (normalized < 7) niceStep = 5 * magnitude
  else niceStep = 10 * magnitude
  const ticks: number[] = []
  for (let t = 0; t <= range; t += niceStep) {
    ticks.push(Math.round(t))
  }
  if (ticks[ticks.length - 1] < range) ticks.push(Math.round(range))
  return ticks
}

export function TrafficTimeline({ traffic, selectedId, onSelect }: TrafficTimelineProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const [hoveredId, setHoveredId] = useState<string | null>(null)
  const [tooltipPos, setTooltipPos] = useState<{ x: number; y: number } | null>(null)

  const sorted = useMemo(
    () =>
      [...traffic].sort(
        (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
      ),
    [traffic],
  )

  const { minTime, range } = useMemo(() => {
    if (sorted.length === 0) return { minTime: 0, range: 0 }
    let min = Infinity
    let max = -Infinity
    for (const e of sorted) {
      const t = new Date(e.timestamp).getTime()
      const dur = e.response?.duration_ms ?? 0
      if (t < min) min = t
      if (t + dur > max) max = t + dur
    }
    return { minTime: min, range: Math.max(max - min, 1) }
  }, [sorted])

  const ticks = useMemo(() => generateTicks(range, 6), [range])

  const virtualizer = useVirtualizer({
    count: sorted.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 16,
  })

  const handleMouseMove = useCallback((e: React.MouseEvent, id: string) => {
    setHoveredId(id)
    setTooltipPos({ x: e.clientX, y: e.clientY })
  }, [])

  const handleMouseLeave = useCallback(() => {
    setHoveredId(null)
    setTooltipPos(null)
  }, [])

  if (traffic.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-muted-foreground">
        <div>
          <Activity className="mx-auto mb-2 h-8 w-8 opacity-30" />
          <p className="mb-1 text-xs">No traffic to visualize</p>
          <p className="text-2xs">Capture some requests to see the waterfall timeline</p>
        </div>
      </div>
    )
  }

  const hoveredEntry = hoveredId ? sorted.find((e) => e.id === hoveredId) : null

  return (
    <div className="relative flex h-full flex-col" role="list" aria-label="Traffic timeline">
      {/* Legend */}
      <div className="flex shrink-0 items-center gap-3 border-b border-border bg-muted/30 px-2 py-1 text-2xs text-muted-foreground">
        <span className="font-medium">Waterfall</span>
        <div className="flex items-center gap-1">
          <span className="inline-block h-2 w-3 rounded-sm bg-success/70" /> 2xx
        </div>
        <div className="flex items-center gap-1">
          <span className="inline-block h-2 w-3 rounded-sm bg-primary/70" /> 3xx
        </div>
        <div className="flex items-center gap-1">
          <span className="inline-block h-2 w-3 rounded-sm bg-warning/70" /> 4xx
        </div>
        <div className="flex items-center gap-1">
          <span className="inline-block h-2 w-3 rounded-sm bg-destructive/70" /> 5xx
        </div>
        <div className="flex items-center gap-1">
          <span className="inline-block h-2 w-3 rounded-sm bg-muted-foreground/40" /> Pending
        </div>
      </div>

      {/* Time axis */}
      <div className="flex shrink-0 border-b border-border" style={{ height: AXIS_HEIGHT }}>
        <div className="shrink-0 border-r border-border" style={{ width: LABEL_WIDTH }} />
        <div className="relative flex-1 overflow-hidden">
          {ticks.map((t) => (
            <div key={t} className="absolute top-0 h-full" style={{ left: `${(t / range) * 100}%` }}>
              <div className="h-full w-px bg-border/60" />
              <span className="absolute bottom-0 left-0.5 whitespace-nowrap text-2xs text-muted-foreground">
                {formatRelative(t)}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Virtualized rows */}
      <div ref={scrollRef} className="flex-1 overflow-auto">
        <div style={{ height: virtualizer.getTotalSize(), width: "100%", position: "relative" }}>
          {virtualizer.getVirtualItems().map((virtualRow) => {
            const entry = sorted[virtualRow.index]
            const start = new Date(entry.timestamp).getTime() - minTime
            const dur = entry.response?.duration_ms ?? 0
            const leftPct = (start / range) * 100
            const widthPct = (dur / range) * 100
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
                <TimelineRow
                  entry={entry}
                  isSelected={entry.id === selectedId}
                  onSelect={onSelect}
                  leftPct={leftPct}
                  widthPct={widthPct}
                  isHovered={hoveredId === entry.id}
                  onMouseEnter={(e) => handleMouseMove(e, entry.id)}
                  onMouseMove={(e) => handleMouseMove(e, entry.id)}
                  onMouseLeave={handleMouseLeave}
                />
              </div>
            )
          })}
        </div>
      </div>

      {/* Tooltip */}
      {hoveredEntry && tooltipPos && (
        <div
          className="pointer-events-none fixed z-50 max-w-xs rounded-md border border-border bg-popover px-2 py-1.5 text-2xs shadow-md"
          style={{ left: tooltipPos.x + 12, top: tooltipPos.y + 12 }}
        >
          <div className="flex items-center gap-1.5">
            <span
              className={cn(
                "font-mono font-semibold",
                `method-${hoveredEntry.request.method.toLowerCase()}`,
              )}
            >
              {hoveredEntry.request.method}
            </span>
            <span className="truncate font-mono text-muted-foreground">
              {hoveredEntry.request.host}
              {hoveredEntry.request.path}
            </span>
          </div>
          <div className="mt-0.5 flex items-center justify-between gap-3">
            <span className="text-muted-foreground">Status</span>
            <span
              className={cn(
                "font-mono",
                hoveredEntry.response
                  ? `status-${Math.floor(hoveredEntry.response.status_code / 100)}xx`
                  : "",
              )}
            >
              {hoveredEntry.response?.status_code ?? "Pending"}
            </span>
          </div>
          <div className="flex items-center justify-between gap-3">
            <span className="text-muted-foreground">Duration</span>
            <span className="font-mono">
              {hoveredEntry.response ? formatDuration(hoveredEntry.response.duration_ms) : "—"}
            </span>
          </div>
          <div className="flex items-center justify-between gap-3">
            <span className="text-muted-foreground">Time</span>
            <span className="font-mono">
              {new Date(hoveredEntry.timestamp).toLocaleTimeString()}
            </span>
          </div>
        </div>
      )}
    </div>
  )
}

interface TimelineRowProps {
  entry: TrafficEntry
  isSelected: boolean
  onSelect: (id: string) => void
  leftPct: number
  widthPct: number
  isHovered: boolean
  onMouseEnter: (e: React.MouseEvent) => void
  onMouseMove: (e: React.MouseEvent) => void
  onMouseLeave: () => void
}

const TimelineRow = memo(function TimelineRow({
  entry,
  isSelected,
  onSelect,
  leftPct,
  widthPct,
  isHovered,
  onMouseEnter,
  onMouseMove,
  onMouseLeave,
}: TimelineRowProps) {
  const colorClass = statusBarClass(entry.response?.status_code)
  const isPassthrough = entry.is_passthrough === true

  return (
    <div
      className={cn(
        "flex cursor-pointer items-center transition-colors hover:bg-muted/40",
        isSelected && "bg-primary/10 hover:bg-primary/15",
        isPassthrough && "opacity-70",
      )}
      style={{ height: ROW_HEIGHT }}
      onClick={() => onSelect(entry.id)}
      onMouseEnter={onMouseEnter}
      onMouseMove={onMouseMove}
      onMouseLeave={onMouseLeave}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault()
          onSelect(entry.id)
        }
      }}
      aria-selected={isSelected}
    >
      {/* Label column */}
      <div
        className="flex shrink-0 items-center gap-1 overflow-hidden border-r border-border px-2"
        style={{ width: LABEL_WIDTH }}
      >
        <span
          className={cn(
            "shrink-0 font-mono text-2xs font-semibold",
            `method-${entry.request.method.toLowerCase()}`,
          )}
        >
          {entry.request.method}
        </span>
        <span
          className="truncate text-2xs text-muted-foreground"
          title={`${entry.request.host}${entry.request.path}`}
        >
          {entry.request.host}
          <span className="text-foreground/60">{entry.request.path}</span>
        </span>
      </div>
      {/* Bar area */}
      <div className="relative h-full flex-1">
        <div
          className={cn(
            "absolute top-1/2 h-3.5 -translate-y-1/2 rounded-sm transition-shadow",
            colorClass,
            isHovered && "ring-1 ring-ring",
          )}
          style={{ left: `${leftPct}%`, width: `${widthPct}%`, minWidth: MIN_BAR_WIDTH }}
        />
      </div>
    </div>
  )
})
