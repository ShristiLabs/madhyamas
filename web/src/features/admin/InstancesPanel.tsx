/**
 * InstancesPanel — multi-instance cluster overview.
 *
 * Lists all active instances registered in Redis with their heartbeat
 * and status. API: GET /api/instances.
 */
import { useQuery } from "@tanstack/react-query"
import { Loader2, Server } from "lucide-react"
import { listInstancesApi } from "@/lib/api/admin"

export function InstancesPanel() {
  const { data, isLoading, isError } = useQuery({
    queryKey: ["admin-instances"],
    queryFn: listInstancesApi,
    refetchInterval: 10000,
    retry: false,
  })

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading instances…
      </div>
    )
  }

  if (isError) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
        <Server className="h-8 w-8" />
        <p className="text-sm">Cluster mode not configured</p>
        <p className="text-2xs">Start the server with --redis-url to enable multi-instance.</p>
      </div>
    )
  }

  const instances = data?.instances ?? []

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">Active Instances ({instances.length})</h2>
      </div>

      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 bg-card text-left text-muted-foreground">
            <tr className="border-b border-border">
              <th className="px-4 py-2 font-medium">Instance ID</th>
              <th className="px-4 py-2 font-medium">Address</th>
              <th className="px-4 py-2 font-medium">Last Heartbeat</th>
              <th className="px-4 py-2 font-medium">Status</th>
            </tr>
          </thead>
          <tbody>
            {instances.map((inst) => {
              const hbAge = Date.now() / 1000 - inst.last_heartbeat
              const isStale = hbAge > 120
              return (
                <tr key={inst.instance_id} className="border-b border-border/50 hover:bg-muted/30">
                  <td className="px-4 py-2 font-mono text-2xs">{inst.instance_id}</td>
                  <td className="px-4 py-2 font-mono text-2xs">{inst.addr}</td>
                  <td className="px-4 py-2 text-muted-foreground">
                    {isStale ? `${Math.floor(hbAge / 60)}m ago` : "recent"}
                  </td>
                  <td className="px-4 py-2">
                    <span className={
                      isStale
                        ? "rounded bg-warning/10 px-1.5 py-0.5 text-2xs font-medium text-warning"
                        : "rounded bg-success/10 px-1.5 py-0.5 text-2xs font-medium text-success"
                    }>
                      {isStale ? "stale" : inst.status}
                    </span>
                  </td>
                </tr>
              )
            })}
            {instances.length === 0 && (
              <tr>
                <td colSpan={4} className="px-4 py-8 text-center text-muted-foreground">
                  No active instances registered.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
