/**
 * AuditPanel — audit event table with filters, pagination, export, and stats.
 *
 * API: GET /api/audit, GET /api/audit/stats, GET /api/audit/export.
 */
import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Loader2, Download, ChevronLeft, ChevronRight } from "lucide-react"
import {
  listAuditApi,
  getAuditStatsApi,
  exportAuditApi,
  type AuditFilter,
} from "@/lib/api/admin"
import { useToast } from "@/components/ui/use-toast"

const EVENT_TYPES = [
  "login", "logout", "api_key_created", "api_key_revoked",
  "traffic_exported", "session_created", "session_deleted",
  "mock_created", "mock_deleted", "breakpoint_created", "breakpoint_deleted",
  "config_changed",
]

const PAGE_SIZE = 50

export function AuditPanel() {
  const { toast } = useToast()
  const [filter, setFilter] = useState<AuditFilter>({ limit: PAGE_SIZE, offset: 0 })
  const [eventType, setEventType] = useState<string>("all")
  const [userId, setUserId] = useState("")

  const { data: events, isLoading } = useQuery({
    queryKey: ["admin-audit", filter],
    queryFn: () => listAuditApi(filter),
  })

  const { data: stats } = useQuery({
    queryKey: ["admin-audit-stats"],
    queryFn: getAuditStatsApi,
  })

  const handleFilter = () => {
    setFilter({
      event_types: eventType !== "all" ? eventType : undefined,
      user_id: userId || undefined,
      limit: PAGE_SIZE,
      offset: 0,
    })
  }

  const handleExport = async () => {
    try {
      const blob = await exportAuditApi()
      const url = URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = `audit-export-${Date.now()}.json`
      a.click()
      URL.revokeObjectURL(url)
      toast({ title: "Audit log exported" })
    } catch {
      toast({ title: "Export failed", variant: "destructive" })
    }
  }

  const handlePrev = () => {
    setFilter((f) => ({ ...f, offset: Math.max(0, (f.offset ?? 0) - PAGE_SIZE) }))
  }
  const handleNext = () => {
    setFilter((f) => ({ ...f, offset: (f.offset ?? 0) + PAGE_SIZE }))
  }

  const currentPage = Math.floor((filter.offset ?? 0) / PAGE_SIZE) + 1

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">Audit Log</h2>
        <Button size="sm" variant="outline" onClick={() => void handleExport()}>
          <Download className="mr-1 h-3.5 w-3.5" /> Export
        </Button>
      </div>

      {stats && (
        <div className="flex gap-4 border-b border-border px-4 py-2 text-2xs text-muted-foreground">
          <span>Total: <strong className="text-foreground">{stats.total_events}</strong></span>
          <span>Today: <strong className="text-foreground">{stats.events_today}</strong></span>
          <span>Errors: <strong className="text-foreground">{stats.error_count}</strong></span>
          {Object.entries(stats.events_by_type).slice(0, 4).map(([type, count]) => (
            <span key={type}>{type}: <strong className="text-foreground">{count}</strong></span>
          ))}
        </div>
      )}

      <div className="flex items-end gap-2 border-b border-border px-4 py-2">
        <div className="space-y-1">
          <Label className="text-2xs">Event Type</Label>
          <Select value={eventType} onValueChange={setEventType}>
            <SelectTrigger className="h-8 w-[160px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All types</SelectItem>
              {EVENT_TYPES.map((t) => (
                <SelectItem key={t} value={t}>{t}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-1">
          <Label className="text-2xs">User ID</Label>
          <Input
            className="h-8 w-[160px]"
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            placeholder="Filter by user"
          />
        </div>
        <Button size="sm" onClick={handleFilter}>Filter</Button>
      </div>

      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading…
          </div>
        ) : (
          <table className="w-full text-xs">
            <thead className="sticky top-0 bg-card text-left text-muted-foreground">
              <tr className="border-b border-border">
                <th className="px-4 py-2 font-medium">Timestamp</th>
                <th className="px-4 py-2 font-medium">Type</th>
                <th className="px-4 py-2 font-medium">User</th>
                <th className="px-4 py-2 font-medium">Description</th>
                <th className="px-4 py-2 font-medium">IP</th>
              </tr>
            </thead>
            <tbody>
              {events?.map((e) => (
                <tr key={e.id} className="border-b border-border/50 hover:bg-muted/30">
                  <td className="px-4 py-2 text-muted-foreground whitespace-nowrap">
                    {new Date(e.timestamp).toLocaleString()}
                  </td>
                  <td className="px-4 py-2">
                    <span className="rounded bg-primary/10 px-1.5 py-0.5 text-2xs font-medium text-primary">
                      {e.event_type}
                    </span>
                  </td>
                  <td className="px-4 py-2 text-muted-foreground">{e.user_id || "—"}</td>
                  <td className="px-4 py-2">{e.description}</td>
                  <td className="px-4 py-2 text-muted-foreground">{e.client_ip || "—"}</td>
                </tr>
              ))}
              {events?.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                    No audit events found.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}
      </div>

      <div className="flex items-center justify-between border-t border-border px-4 py-2 text-2xs text-muted-foreground">
        <span>Page {currentPage}</span>
        <div className="flex gap-1">
          <Button
            variant="outline"
            size="sm"
            onClick={handlePrev}
            disabled={(filter.offset ?? 0) === 0}
          >
            <ChevronLeft className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleNext}
            disabled={(events?.length ?? 0) < PAGE_SIZE}
          >
            <ChevronRight className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
    </div>
  )
}
