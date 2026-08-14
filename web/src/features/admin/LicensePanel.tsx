/**
 * LicensePanel — displays license info, seat usage, and expiry warnings.
 *
 * API: GET /api/license. Also fetches cluster metrics for seat usage.
 */
import { useQuery } from "@tanstack/react-query"
import { Loader2, AlertTriangle, CheckCircle2, XCircle } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { getLicenseApi, getClusterMetricsApi } from "@/lib/api/admin"

export function LicensePanel() {
  const { data: license, isLoading } = useQuery({
    queryKey: ["admin-license"],
    queryFn: getLicenseApi,
  })

  const { data: cluster } = useQuery({
    queryKey: ["admin-license-cluster"],
    queryFn: getClusterMetricsApi,
    retry: false,
  })

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading license…
      </div>
    )
  }

  if (!license?.licensed) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
        <XCircle className="h-8 w-8" />
        <p className="text-sm">No active license</p>
        <p className="text-2xs">Running in unlicensed enterprise mode.</p>
      </div>
    )
  }

  const expiresAt = license.expires_at ? new Date(license.expires_at) : null
  const daysRemaining = expiresAt
    ? Math.floor((expiresAt.getTime() - Date.now()) / (1000 * 60 * 60 * 24))
    : null
  const expiringSoon = daysRemaining !== null && daysRemaining < 30
  const expired = daysRemaining !== null && daysRemaining < 0

  const seatUsage = cluster?.instances.length ?? 0
  const maxSeats = license.seats ?? 0

  return (
    <div className="flex h-full flex-col overflow-auto">
      <div className="border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">License Information</h2>
      </div>

      <div className="space-y-4 p-4">
        <div className="rounded-md border border-border bg-card p-4">
          <div className="mb-3 flex items-center gap-2">
            <CheckCircle2 className="h-5 w-5 text-success" />
            <span className="text-sm font-semibold">{license.customer ?? "Licensed"}</span>
            <Badge variant="success" className="ml-auto">{license.plan ?? "enterprise"}</Badge>
          </div>

          <div className="grid grid-cols-2 gap-3 text-xs">
            <InfoRow label="License ID" value={license.license_id ?? "—"} mono />
            <InfoRow label="Instance ID" value={license.instance_id ?? "—"} mono />
            <InfoRow label="Issued" value={license.issued_at ? new Date(license.issued_at).toLocaleDateString() : "—"} />
            <InfoRow label="Expires" value={expiresAt ? expiresAt.toLocaleDateString() : "Never"} />
            <InfoRow label="Seats" value={`${seatUsage} / ${maxSeats}`} />
            <InfoRow label="Days Remaining" value={daysRemaining !== null ? String(daysRemaining) : "∞"} />
          </div>
        </div>

        {expired && (
          <div className="flex items-center gap-2 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
            <AlertTriangle className="h-4 w-4" />
            License has expired. Please renew to continue using enterprise features.
          </div>
        )}

        {expiringSoon && !expired && (
          <div className="flex items-center gap-2 rounded-md border border-warning/30 bg-warning/10 p-3 text-sm text-warning">
            <AlertTriangle className="h-4 w-4" />
            License expires in {daysRemaining} days. Please renew soon.
          </div>
        )}

        {license.features && license.features.length > 0 && (
          <div className="rounded-md border border-border bg-card p-4">
            <h3 className="mb-2 text-xs font-semibold">Enabled Features</h3>
            <div className="flex flex-wrap gap-1.5">
              {license.features.map((f) => (
                <Badge key={f} variant="secondary" className="text-2xs">{f}</Badge>
              ))}
            </div>
          </div>
        )}

        {maxSeats > 0 && (
          <div className="rounded-md border border-border bg-card p-4">
            <h3 className="mb-2 text-xs font-semibold">Seat Usage</h3>
            <div className="flex items-center gap-2">
              <div className="h-3 flex-1 overflow-hidden rounded bg-muted">
                <div
                  className="h-full bg-primary transition-all"
                  style={{ width: `${Math.min(100, (seatUsage / maxSeats) * 100)}%` }}
                />
              </div>
              <span className="text-2xs font-mono">{seatUsage} / {maxSeats}</span>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-2xs text-muted-foreground">{label}</span>
      <span className={mono ? "font-mono text-2xs" : "text-xs"}>{value}</span>
    </div>
  )
}
