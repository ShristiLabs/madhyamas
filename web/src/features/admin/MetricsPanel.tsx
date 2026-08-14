/**
 * MetricsPanel — performance metrics dashboard with auto-refresh.
 *
 * Shows local metrics and cluster-wide metrics (from Redis). Uses
 * div-based bars instead of a chart library. Auto-refreshes every 5s.
 */
import { useQuery } from "@tanstack/react-query"
import { Loader2 } from "lucide-react"
import { getMetricsApi, getClusterMetricsApi } from "@/lib/api/admin"

export function MetricsPanel() {
  const { data: metrics, isLoading } = useQuery({
    queryKey: ["admin-metrics"],
    queryFn: getMetricsApi,
    refetchInterval: 5000,
  })

  const { data: cluster } = useQuery({
    queryKey: ["admin-cluster-metrics"],
    queryFn: getClusterMetricsApi,
    refetchInterval: 5000,
    retry: false,
  })

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading metrics…
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col overflow-auto">
      <div className="border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">Performance Metrics</h2>
      </div>

      <div className="grid grid-cols-2 gap-3 p-4 lg:grid-cols-4">
        <MetricCard label="Total Requests" value={metrics?.requests_total ?? 0} />
        <MetricCard label="Successful" value={metrics?.requests_success ?? 0} />
        <MetricCard label="Failed" value={metrics?.requests_failed ?? 0} />
        <MetricCard label="Avg Latency" value={`${(metrics?.avg_latency_ms ?? 0).toFixed(1)} ms`} />
        <MetricCard label="Req/sec" value={(metrics?.requests_per_second ?? 0).toFixed(1)} />
      </div>

      <div className="border-t border-border px-4 py-2">
        <h3 className="text-xs font-semibold">Request Distribution</h3>
      </div>
      <div className="space-y-2 px-4 pb-4">
        <BarRow
          label="Successful"
          value={metrics?.requests_success ?? 0}
          max={metrics?.requests_total ?? 1}
          color="bg-success"
        />
        <BarRow
          label="Failed"
          value={metrics?.requests_failed ?? 0}
          max={metrics?.requests_total ?? 1}
          color="bg-destructive"
        />
      </div>

      {cluster && (
        <>
          <div className="border-t border-border px-4 py-2">
            <h3 className="text-xs font-semibold">Cluster Overview</h3>
          </div>
          <div className="grid grid-cols-2 gap-3 px-4 pb-4 lg:grid-cols-4">
            <MetricCard label="Active Connections" value={cluster.total_active_connections} />
            <MetricCard label="Total Requests" value={cluster.total_request_count} />
            <MetricCard label="Avg CPU" value={`${cluster.avg_cpu_usage.toFixed(1)}%`} />
            <MetricCard label="Avg Memory" value={`${cluster.avg_memory_usage_mb.toFixed(0)} MB`} />
          </div>

          <div className="border-t border-border px-4 py-2">
            <h3 className="text-xs font-semibold">Instances ({cluster.instances.length})</h3>
          </div>
          <div className="overflow-auto px-4 pb-4">
            <table className="w-full text-xs">
              <thead className="text-left text-muted-foreground">
                <tr className="border-b border-border">
                  <th className="py-2 font-medium">Instance</th>
                  <th className="py-2 font-medium">Address</th>
                  <th className="py-2 font-medium">Status</th>
                  <th className="py-2 font-medium">CPU</th>
                  <th className="py-2 font-medium">Memory</th>
                  <th className="py-2 font-medium">Connections</th>
                </tr>
              </thead>
              <tbody>
                {cluster.instances.map((inst) => (
                  <tr key={inst.instance_id} className="border-b border-border/50">
                    <td className="py-2 font-mono text-2xs">{inst.instance_id.slice(0, 12)}</td>
                    <td className="py-2 font-mono text-2xs">{inst.addr}</td>
                    <td className="py-2">
                      <span className="rounded bg-success/10 px-1.5 py-0.5 text-2xs font-medium text-success">
                        {inst.status}
                      </span>
                    </td>
                    <td className="py-2">{inst.cpu_usage.toFixed(1)}%</td>
                    <td className="py-2">{inst.memory_usage_mb} MB</td>
                    <td className="py-2">{inst.active_connections}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  )
}

function MetricCard({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded-md border border-border bg-card p-3">
      <div className="text-2xs text-muted-foreground">{label}</div>
      <div className="text-lg font-semibold">{value}</div>
    </div>
  )
}

function BarRow({ label, value, max, color }: {
  label: string
  value: number
  max: number
  color: string
}) {
  const pct = max > 0 ? Math.min(100, (value / max) * 100) : 0
  return (
    <div className="flex items-center gap-2">
      <span className="w-24 text-2xs text-muted-foreground">{label}</span>
      <div className="h-3 flex-1 overflow-hidden rounded bg-muted">
        <div className={`h-full ${color} transition-all`} style={{ width: `${pct}%` }} />
      </div>
      <span className="w-12 text-right text-2xs font-mono">{value}</span>
    </div>
  )
}
