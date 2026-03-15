import { useState, useMemo, useCallback, memo, useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import { ArrowUp, ArrowDown, ArrowUpDown } from "lucide-react";
import { Checkbox } from "./ui/checkbox";
import type { TrafficEntry } from "@/types/traffic";

type SortField =
  | "timestamp"
  | "method"
  | "status"
  | "path"
  | "duration"
  | "size";
type SortDirection = "asc" | "desc";
type ResizableCol =
  | "method"
  | "protocol"
  | "domain"
  | "status"
  | "duration"
  | "size"
  | "timestamp";

interface TrafficListProps {
  traffic: TrafficEntry[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  selectedIds?: Set<string>;
  onToggleSelect?: (id: string) => void;
  onSelectAll?: () => void;
}

const DEFAULT_COL_WIDTHS: Record<ResizableCol, number> = {
  method: 64,
  protocol: 60,
  domain: 130,
  status: 52,
  duration: 60,
  size: 56,
  timestamp: 80,
};

interface ColHeaderProps {
  label: string;
  field?: SortField;
  sortField: SortField;
  sortDirection: SortDirection;
  onSort: (f: SortField) => void;
  width?: number;
  flex?: boolean;
  align?: "left" | "right";
  onResizeStart?: (e: React.MouseEvent) => void;
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
  const isActive = field && sortField === field;
  return (
    <div
      className={cn(
        "relative group flex items-center shrink-0 select-none",
        flex && "flex-1 min-w-0",
      )}
      style={!flex && width !== undefined ? { width } : undefined}
    >
      <button
        className={cn(
          "flex items-center gap-1 h-7 px-1 text-xs font-medium text-muted-foreground hover:text-foreground w-full overflow-hidden",
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
          className="absolute right-0 top-1 bottom-1 w-1 cursor-col-resize rounded opacity-0 group-hover:opacity-100 bg-border hover:bg-primary"
          onMouseDown={onResizeStart}
        />
      )}
    </div>
  );
});

export function TrafficList({
  traffic,
  selectedId,
  onSelect,
  selectedIds,
  onToggleSelect,
  onSelectAll,
}: TrafficListProps) {
  const [sortField, setSortField] = useState<SortField>("timestamp");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");
  const [colWidths, setColWidths] =
    useState<Record<ResizableCol, number>>(DEFAULT_COL_WIDTHS);
  const resizeRef = useRef<{
    col: ResizableCol;
    startX: number;
    startWidth: number;
  } | null>(null);

  const startResize = useCallback(
    (col: ResizableCol, e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      resizeRef.current = {
        col,
        startX: e.clientX,
        startWidth: colWidths[col],
      };
    },
    [colWidths],
  );

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      if (!resizeRef.current) return;
      const { col, startX, startWidth } = resizeRef.current;
      const newWidth = Math.max(40, startWidth + e.clientX - startX);
      setColWidths((prev) => ({ ...prev, [col]: newWidth }));
    };
    const onMouseUp = () => {
      resizeRef.current = null;
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    return () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };
  }, []);

  const handleSort = useCallback(
    (field: SortField) => {
      if (sortField === field) {
        setSortDirection((prev) => (prev === "asc" ? "desc" : "asc"));
      } else {
        setSortField(field);
        setSortDirection("desc");
      }
    },
    [sortField],
  );

  const sortedTraffic = useMemo(() => {
    return [...traffic].sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case "timestamp":
          cmp =
            new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
          break;
        case "method":
          cmp = a.request.method.localeCompare(b.request.method);
          break;
        case "status":
          cmp = (a.response?.status_code || 0) - (b.response?.status_code || 0);
          break;
        case "path":
          cmp = a.request.path.localeCompare(b.request.path);
          break;
        case "duration":
          cmp = (a.response?.duration_ms || 0) - (b.response?.duration_ms || 0);
          break;
        case "size":
          cmp = calculateSize(a) - calculateSize(b);
          break;
      }
      return sortDirection === "asc" ? cmp : -cmp;
    });
  }, [traffic, sortField, sortDirection]);

  if (traffic.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground p-4 text-center">
        <div>
          <p className="mb-2">No traffic captured yet</p>
          <p className="text-sm">
            Configure your browser or app to use localhost:8888 as proxy
          </p>
        </div>
      </div>
    );
  }

  const headerProps = { sortField, sortDirection, onSort: handleSort };

  return (
    <div
      className="flex flex-col h-full"
      role="list"
      aria-label="Traffic entries"
    >
      {/* Column Headers */}
      <div className="flex items-center border-b bg-muted/30 px-3">
        {onSelectAll && (
          <div className="w-8 flex items-center justify-center py-2 shrink-0">
            <Checkbox
              checked={
                selectedIds &&
                selectedIds.size === traffic.length &&
                traffic.length > 0
              }
              onCheckedChange={onSelectAll}
              aria-label="Select all"
            />
          </div>
        )}
        <ColHeader
          {...headerProps}
          label="Method"
          field="method"
          width={colWidths.method}
          onResizeStart={(e) => startResize("method", e)}
        />
        <ColHeader
          {...headerProps}
          label="Protocol"
          width={colWidths.protocol}
          onResizeStart={(e) => startResize("protocol", e)}
        />
        <ColHeader
          {...headerProps}
          label="Domain"
          width={colWidths.domain}
          onResizeStart={(e) => startResize("domain", e)}
        />
        <ColHeader {...headerProps} label="Path" field="path" flex />
        <ColHeader
          {...headerProps}
          label="Status"
          field="status"
          width={colWidths.status}
          align="right"
          onResizeStart={(e) => startResize("status", e)}
        />
        <ColHeader
          {...headerProps}
          label="Time"
          field="duration"
          width={colWidths.duration}
          onResizeStart={(e) => startResize("duration", e)}
        />
        <ColHeader
          {...headerProps}
          label="Size"
          field="size"
          width={colWidths.size}
          align="right"
          onResizeStart={(e) => startResize("size", e)}
        />
        <ColHeader
          {...headerProps}
          label="Time"
          field="timestamp"
          width={colWidths.timestamp}
          align="right"
          onResizeStart={(e) => startResize("timestamp", e)}
        />
      </div>

      {/* Traffic List */}
      <div className="flex-1 overflow-auto">
        <div className="divide-y" role="listitem">
          {sortedTraffic.map((entry) => (
            <TrafficListItem
              key={entry.id}
              entry={entry}
              isSelected={entry.id === selectedId}
              onClick={() => onSelect(entry.id)}
              colWidths={colWidths}
              isChecked={selectedIds?.has(entry.id)}
              onToggleCheck={
                onToggleSelect ? () => onToggleSelect(entry.id) : undefined
              }
            />
          ))}
        </div>
      </div>
    </div>
  );
}

interface TrafficListItemProps {
  entry: TrafficEntry;
  isSelected: boolean;
  onClick: () => void;
  colWidths: Record<ResizableCol, number>;
  isChecked?: boolean;
  onToggleCheck?: () => void;
}

const TrafficListItem = memo(function TrafficListItem({
  entry,
  isSelected,
  onClick,
  colWidths,
  isChecked,
  onToggleCheck,
}: TrafficListItemProps) {
  const methodClass = `method-${entry.request.method.toLowerCase()}`;
  const statusClass = entry.response
    ? `status-${Math.floor(entry.response.status_code / 100)}xx`
    : "";

  const time = new Date(entry.timestamp).toLocaleTimeString();
  const size = entry.response ? formatSize(calculateSize(entry)) : "-";
  const duration = entry.response ? `${entry.response.duration_ms}ms` : "-";
  const protocol = entry.request.url.startsWith("https://") ? "HTTPS" : "HTTP";

  return (
    <div
      className={cn(
        "flex items-center px-3 py-1.5 cursor-pointer hover:bg-muted/50 transition-colors text-sm",
        isSelected && "bg-primary/10",
      )}
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      }}
      aria-selected={isSelected}
    >
      {onToggleCheck && (
        <div
          className="w-8 flex items-center justify-center shrink-0"
          onClick={(e) => {
            e.stopPropagation();
            onToggleCheck();
          }}
        >
          <Checkbox
            checked={isChecked || false}
            onCheckedChange={onToggleCheck}
            onClick={(e) => e.stopPropagation()}
          />
        </div>
      )}
      <span
        className={cn("font-mono font-semibold shrink-0 truncate", methodClass)}
        style={{ width: colWidths.method }}
      >
        {entry.request.method}
      </span>
      <span
        className="shrink-0 text-xs text-muted-foreground px-1"
        style={{ width: colWidths.protocol }}
      >
        {protocol}
      </span>
      <span
        className="shrink-0 truncate text-xs text-muted-foreground px-1"
        style={{ width: colWidths.domain }}
        title={entry.request.host}
      >
        {entry.request.host}
      </span>
      <span
        className="flex-1 min-w-0 truncate font-mono text-xs px-1"
        title={entry.request.path}
      >
        {entry.request.path}
      </span>
      <span
        className={cn("shrink-0 text-right", statusClass)}
        style={{ width: colWidths.status }}
      >
        {entry.response?.status_code || "-"}
      </span>
      <span
        className="shrink-0 text-right text-xs text-muted-foreground"
        style={{ width: colWidths.duration }}
      >
        {duration}
      </span>
      <span
        className="shrink-0 text-right text-xs text-muted-foreground"
        style={{ width: colWidths.size }}
      >
        {size}
      </span>
      <span
        className="shrink-0 text-right text-xs text-muted-foreground"
        style={{ width: colWidths.timestamp }}
      >
        {time}
      </span>
    </div>
  );
});

function calculateSize(entry: TrafficEntry): number {
  const reqSize = entry.request.body?.length || 0;
  const resSize = entry.response?.body?.length || 0;
  return reqSize + resSize;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}
