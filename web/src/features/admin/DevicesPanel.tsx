/**
 * DevicesPanel — device principal management (enterprise, issues #104/#105).
 *
 * Registers devices, mints per-device connect-only credentials
 * (`mdy_dev_...`), and tracks liveness via the proxy-auth-derived
 * `last_seen`. On create/rotate the plaintext credential is shown ONCE
 * alongside the manual-apply values (host/port/username/password) for
 * clients that cannot scan a QR (QR onboarding is a later issue).
 * Issue #105: per-row "view traffic" opens the traffic view scoped to the
 * device, and the status flips to "Connected — capturing" live while the
 * device's attributed entries stream in over the traffic WebSocket.
 * API: GET/POST /api/devices, POST /api/devices/:id/rotate|revoke,
 * DELETE /api/devices/:id.
 */
import { useEffect, useRef, useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Loader2,
  Plus,
  Trash2,
  Copy,
  AlertTriangle,
  Check,
  RefreshCw,
  Ban,
  Activity,
} from "lucide-react"
import { apiGet } from "@/lib/api/client"
import {
  listDevicesApi,
  createDeviceApi,
  rotateDeviceKeyApi,
  revokeDeviceApi,
  deleteDeviceApi,
  type DeviceEntry,
  type CreateDevicePayload,
} from "@/lib/api/admin"
import { buildTrafficWsUrl } from "@/hooks/useTrafficWebSocket"
import { useWebSocket } from "@/hooks/useWebSocket"
import type { WsServerMessage } from "@/types/websocket"
import { useToast } from "@/components/ui/use-toast"
import { ApiError } from "@/lib/api/client"

/** A device counts as live when its last proxy-auth event is this fresh. */
const LIVE_WINDOW_MS = 60_000

/** A device shows "Connected — capturing" while an attributed entry
 * arrived this recently over the traffic WebSocket (issue #105). */
const CAPTURING_WINDOW_MS = 60_000

function formatSeen(lastSeen: number | null): string {
  if (!lastSeen) return "—"
  const delta = Date.now() - lastSeen * 1000
  if (delta < 60_000) return `${Math.max(1, Math.floor(delta / 1000))}s ago`
  if (delta < 3_600_000) return `${Math.floor(delta / 60_000)}m ago`
  if (delta < 86_400_000) return `${Math.floor(delta / 3_600_000)}h ago`
  return new Date(lastSeen * 1000).toLocaleString()
}

interface DeviceStatus {
  label: string
  variant: "success" | "secondary" | "destructive" | "outline"
}

function deviceStatus(d: DeviceEntry, lastCaptureAt: number | null): DeviceStatus {
  if (d.status === "revoked") return { label: "Revoked", variant: "destructive" }
  // Live capture signal from attributed traffic entries — flips the card
  // from "Pending" on the device's first captured request without waiting
  // for the 15s REST refresh.
  if (lastCaptureAt && Date.now() - lastCaptureAt < CAPTURING_WINDOW_MS) {
    return { label: "Connected — capturing", variant: "success" }
  }
  if (!d.last_seen) return { label: "Pending", variant: "secondary" }
  const live = Date.now() - d.last_seen * 1000 < LIVE_WINDOW_MS
  return live
    ? { label: "Live", variant: "success" }
    : { label: "Seen", variant: "outline" }
}

export function DevicesPanel() {
  const { toast } = useToast()
  const queryClient = useQueryClient()
  const { data: devices, isLoading } = useQuery({
    queryKey: ["admin-devices"],
    queryFn: listDevicesApi,
    refetchInterval: 15_000,
  })

  // ── Live "Connected — capturing" status (issue #105) ─────────────────
  // Subscribe to the traffic WebSocket and watch for attributed entries
  // (Added events carrying a device_id). Lighter than extending the device
  // event payload: the entries already flow through this stream.
  const [capturingAt, setCapturingAt] = useState<Record<string, number>>({})
  const wsUrl = useRef(buildTrafficWsUrl())
  const handleWsMessage = useRef((message: WsServerMessage) => {
    if (message.type !== "Traffic" || message.data.type !== "Added") return
    const deviceId = message.data.data.device_id
    if (!deviceId) return
    setCapturingAt((prev) => ({ ...prev, [deviceId]: Date.now() }))
  }).current
  useWebSocket({
    url: wsUrl.current,
    onMessage: handleWsMessage,
    autoConnect: true,
    reconnect: true,
  })

  // Open the per-device traffic view: AppShell listens for this event and
  // switches to the traffic view with the device filter pre-applied (the
  // view syncs the shareable `?device=` URL).
  const viewTraffic = (device: DeviceEntry) => {
    window.dispatchEvent(
      new CustomEvent("madhyamas:view-device-traffic", { detail: { device: device.id } }),
    )
  }

  const [createOpen, setCreateOpen] = useState(false)
  const [issued, setIssued] = useState<{ device: DeviceEntry; key: string } | null>(null)
  const [revokeTarget, setRevokeTarget] = useState<DeviceEntry | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<DeviceEntry | null>(null)

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ["admin-devices"] })

  const createMut = useMutation({
    mutationFn: (data: CreateDevicePayload) => createDeviceApi(data),
    onSuccess: (res) => {
      invalidate()
      setCreateOpen(false)
      setIssued(res)
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to register device",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  const rotateMut = useMutation({
    mutationFn: (id: string) => rotateDeviceKeyApi(id),
    onSuccess: (res) => {
      invalidate()
      setIssued(res)
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to rotate device key",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  const revokeMut = useMutation({
    mutationFn: (id: string) => revokeDeviceApi(id),
    onSuccess: () => {
      invalidate()
      setRevokeTarget(null)
      toast({ title: "Device revoked" })
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to revoke device",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  const deleteMut = useMutation({
    mutationFn: (id: string) => deleteDeviceApi(id),
    onSuccess: () => {
      invalidate()
      setDeleteTarget(null)
      toast({ title: "Device deleted" })
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to delete device",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading devices…
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">Devices</h2>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="mr-1 h-3.5 w-3.5" /> Register Device
        </Button>
      </div>

      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 bg-card text-left text-muted-foreground">
            <tr className="border-b border-border">
              <th className="px-4 py-2 font-medium">Name</th>
              <th className="px-4 py-2 font-medium">Status</th>
              <th className="px-4 py-2 font-medium">Created</th>
              <th className="px-4 py-2 font-medium">Last Seen</th>
              <th className="px-4 py-2 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {devices?.map((d) => {
              const status = deviceStatus(d, capturingAt[d.id] ?? null)
              return (
                <tr key={d.id} className="border-b border-border/50 hover:bg-muted/30">
                  <td className="px-4 py-2 font-medium">{d.name}</td>
                  <td className="px-4 py-2">
                    <Badge variant={status.variant} className="text-2xs">
                      {status.label}
                    </Badge>
                  </td>
                  <td className="px-4 py-2 text-muted-foreground">
                    {new Date(d.created_at * 1000).toLocaleDateString()}
                  </td>
                  <td className="px-4 py-2 text-muted-foreground">{formatSeen(d.last_seen)}</td>
                  <td className="px-4 py-2">
                    <div className="flex items-center gap-1">
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        onClick={() => viewTraffic(d)}
                        disabled={d.status === "revoked"}
                        title="View this device's traffic"
                      >
                        <Activity className="h-3 w-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        onClick={() => rotateMut.mutate(d.id)}
                        disabled={rotateMut.isPending || d.status === "revoked"}
                        title="Rotate device key"
                      >
                        <RefreshCw className="h-3 w-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        onClick={() => setRevokeTarget(d)}
                        disabled={d.status === "revoked"}
                        title="Revoke device"
                      >
                        <Ban className="h-3 w-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        onClick={() => setDeleteTarget(d)}
                        title="Delete device"
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  </td>
                </tr>
              )
            })}
            {devices?.length === 0 && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                  No devices registered. Register one to mint a per-device proxy credential.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <CreateDeviceDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        onSubmit={(d) => createMut.mutate(d)}
        loading={createMut.isPending}
      />

      <CredentialDialog issued={issued} onClose={() => setIssued(null)} />

      <Dialog open={!!revokeTarget} onOpenChange={(open) => !open && setRevokeTarget(null)}>
        <DialogContent className="sm:max-w-[400px]">
          <DialogHeader>
            <DialogTitle>Revoke Device</DialogTitle>
            <DialogDescription>
              Are you sure you want to revoke <strong>{revokeTarget?.name}</strong>? Its
              credential stops authenticating immediately and the device record is kept in
              the revoked state.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRevokeTarget(null)}>Cancel</Button>
            <Button
              variant="destructive"
              onClick={() => revokeTarget && revokeMut.mutate(revokeTarget.id)}
              disabled={revokeMut.isPending}
            >
              {revokeMut.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Revoke
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!deleteTarget} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <DialogContent className="sm:max-w-[400px]">
          <DialogHeader>
            <DialogTitle>Delete Device</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete <strong>{deleteTarget?.name}</strong>? Its
              credential is revoked and the device record is removed. This action cannot be
              undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>Cancel</Button>
            <Button
              variant="destructive"
              onClick={() => deleteTarget && deleteMut.mutate(deleteTarget.id)}
              disabled={deleteMut.isPending}
            >
              {deleteMut.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

function CreateDeviceDialog({ open, onOpenChange, onSubmit, loading }: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: CreateDevicePayload) => void
  loading: boolean
}) {
  const [name, setName] = useState("")
  const [installUuid, setInstallUuid] = useState("")
  const [macAddress, setMacAddress] = useState("")

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const data: CreateDevicePayload = { name: name.trim() }
    if (installUuid.trim()) data.install_uuid = installUuid.trim()
    if (macAddress.trim()) data.mac_address = macAddress.trim()
    onSubmit(data)
    setName("")
    setInstallUuid("")
    setMacAddress("")
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>Register Device</DialogTitle>
          <DialogDescription>
            Name the device first — the name labels its traffic from the first captured
            entry. A connect-only proxy credential is minted on creation.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="dev-name">Name</Label>
            <Input
              id="dev-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              placeholder="e.g. Hari's Pixel"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="dev-uuid">Install UUID (optional)</Label>
            <Input
              id="dev-uuid"
              value={installUuid}
              onChange={(e) => setInstallUuid(e.target.value)}
              placeholder="Companion-reported install identifier"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="dev-mac">MAC address (optional)</Label>
            <Input
              id="dev-mac"
              value={macAddress}
              onChange={(e) => setMacAddress(e.target.value)}
              placeholder="aa:bb:cc:dd:ee:ff"
            />
          </div>
          <DialogFooter>
            <Button type="submit" disabled={loading || !name.trim()}>
              {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Register
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

/** One row of the manual-apply block: label + copyable value. */
function CopyRow({ label, value }: { label: string; value: string }) {
  const { toast } = useToast()
  const [copied, setCopied] = useState(false)
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast({ title: "Copy failed", variant: "destructive" })
    }
  }
  return (
    <div className="flex items-center gap-2">
      <span className="w-20 shrink-0 text-muted-foreground">{label}</span>
      <code className="flex-1 truncate rounded bg-muted p-1.5 font-mono text-2xs">{value}</code>
      <Button size="sm" variant="outline" onClick={() => void handleCopy()}>
        {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      </Button>
    </div>
  )
}

/**
 * Show-once credential dialog: the plaintext `mdy_dev_` key plus the
 * manual-apply values (host/port/username/password) rendered as
 * copyable text for devices without QR support.
 */
function CredentialDialog({ issued, onClose }: {
  issued: { device: DeviceEntry; key: string } | null
  onClose: () => void
}) {
  const [host, setHost] = useState("")
  const [port, setPort] = useState(8888)

  // Resolve the proxy listener address once per dialog from /api/config
  // (same source as the header's proxy address display).
  useEffect(() => {
    if (!issued) return
    apiGet<{ host?: string; proxy_port?: number; public_ip?: string }>("/config")
      .then((c) => {
        setHost(c.public_ip || c.host || window.location.hostname)
        setPort(c.proxy_port || 8888)
      })
      .catch(() => {
        setHost(window.location.hostname)
        setPort(8888)
      })
  }, [issued])

  return (
    <Dialog open={!!issued} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[520px]">
        <DialogHeader>
          <DialogTitle>Device Credential Issued</DialogTitle>
          <DialogDescription>
            Apply these values in the device's manual proxy settings. The password is
            shown only once — copy it now.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="flex items-center gap-2 rounded-md border border-warning/30 bg-warning/10 p-3 text-xs text-warning">
            <AlertTriangle className="h-4 w-4 shrink-0" />
            Store this credential securely. It will not be shown again.
          </div>
          <CopyRow label="Password" value={issued?.key ?? ""} />
          <div className="space-y-2 rounded-md border border-border p-3">
            <p className="text-2xs font-medium text-muted-foreground">
              Manual proxy configuration
            </p>
            <CopyRow label="Host" value={host} />
            <CopyRow label="Port" value={String(port)} />
            <CopyRow label="Username" value={issued?.device.name ?? ""} />
          </div>
          <p className="text-2xs text-muted-foreground">
            The credential authenticates proxy connections only — it cannot access the
            REST API. Traffic from this device is attributed to "{issued?.device.name}".
          </p>
        </div>
        <DialogFooter>
          <Button onClick={onClose}>Done</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
