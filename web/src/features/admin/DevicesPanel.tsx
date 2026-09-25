/**
 * DevicesPanel — device principal management (enterprise, issues #104/#105/#106).
 *
 * Registers devices, mints per-device connect-only credentials
 * (`mdy_dev_...`), and tracks liveness via the proxy-auth-derived
 * `last_seen`. On create/rotate the plaintext credential is shown ONCE
 * alongside the manual-apply values (host/port/username/password) for
 * clients that cannot scan a QR.
 * Issue #105: per-row "view traffic" opens the traffic view scoped to the
 * device, and the status flips to "Connected — capturing" live while the
 * device's attributed entries stream in over the traffic WebSocket.
 * Issue #106: the credential dialog adds a QR carrying a
 * `madhyamas://connect` payload with a single-use 15-minute enrollment
 * token (default mode — the QR is never a standing credential), stays
 * open with a live "waiting for device… connected — capturing" loop, and
 * auto-navigates to the device's traffic view on first connect. The raw
 * values remain rendered alongside for manual entry, with a
 * screenshot warning and an instant-rotation affordance.
 * API: GET/POST /api/devices, POST /api/devices/:id/rotate|revoke|
 * enrollment-token, POST /api/devices/enroll, DELETE /api/devices/:id.
 */
import { useEffect, useRef, useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { QRCodeSVG } from "qrcode.react"
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
  Check,
  RefreshCw,
  Ban,
  Activity,
  Camera,
} from "lucide-react"
import { apiGet } from "@/lib/api/client"
import {
  listDevicesApi,
  createDeviceApi,
  rotateDeviceKeyApi,
  revokeDeviceApi,
  deleteDeviceApi,
  createEnrollmentTokenApi,
  type DeviceEntry,
  type DeviceEnrollmentToken,
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

      <CredentialDialog
        issued={issued}
        onClose={() => setIssued(null)}
        lastCaptureAt={issued ? (capturingAt[issued.device.id] ?? null) : null}
        onRotate={(id) => rotateMut.mutate(id)}
        rotatePending={rotateMut.isPending}
      />

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

/** Parameters of the `madhyamas://connect` QR payload (issue #106,
 * docs/CREDENTIAL_ONBOARDING.md QR payload section). Exactly one of
 * `token` / `key` must be set: `token` is the default (single-use
 * 15-minute enrollment token the companion exchanges for the real key);
 * `key` is the manual-mode fallback carrying the show-once credential. */
interface ConnectUriParams {
  host: string
  port: number
  /** TLS flag of the proxy listener — 0 today (listener TLS is a later
   * issue) but the field must exist and round-trip. */
  tls: boolean
  name: string
  token?: string
  key?: string
}

/** Build the `madhyamas://connect` deep-link payload. `ca` and `api` are
 * derived from the instance's own origin — the web UI is served by the
 * API server, the same source the manual-apply host/port values use. */
export function buildConnectUri(params: ConnectUriParams): string {
  const q = new URLSearchParams({
    host: params.host,
    port: String(params.port),
    tls: params.tls ? "1" : "0",
    name: params.name,
  })
  if (params.token) q.set("token", params.token)
  if (params.key) q.set("key", params.key)
  const origin = typeof window !== "undefined" ? window.location.origin : ""
  q.set("ca", `${origin}/api/cert/ca`)
  q.set("api", `${origin}/api`)
  return `madhyamas://connect?${q.toString()}`
}

/** Auto-navigate to the device's traffic view this long after the
 * "connected" flip, so the user sees the status change before the view
 * switches. */
const CONNECT_NAV_DELAY_MS = 1500

function formatCountdown(secondsLeft: number): string {
  const m = Math.floor(secondsLeft / 60)
  const s = secondsLeft % 60
  return `${m}:${String(s).padStart(2, "0")}`
}

/**
 * Credential dialog (issues #104/#106): the QR carries a
 * `madhyamas://connect` payload with a single-use enrollment token
 * (default mode — a photographed QR expires), while the raw values
 * (host/port/username/password) are ALWAYS rendered alongside for
 * manual entry. The dialog stays open with a live status loop
 * ("waiting for device… connected — capturing") driven by the traffic
 * WebSocket, and auto-navigates to the device's traffic view on the
 * first attributed entry. The manual section warns against screenshots
 * and offers instant rotation.
 */
function CredentialDialog({ issued, onClose, lastCaptureAt, onRotate, rotatePending }: {
  issued: { device: DeviceEntry; key: string } | null
  onClose: () => void
  /** Epoch ms of the device's last attributed WS entry (panel-level
   * capture tracker), or null when none arrived yet. */
  lastCaptureAt: number | null
  onRotate: (id: string) => void
  rotatePending: boolean
}) {
  const [host, setHost] = useState("")
  const [port, setPort] = useState(8888)
  const [enrollment, setEnrollment] = useState<DeviceEnrollmentToken | null>(null)
  const [enrollmentError, setEnrollmentError] = useState(false)
  const [nowMs, setNowMs] = useState(() => Date.now())
  /** When this dialog instance opened — only entries arriving AFTER this
   * count as the device connecting through it. */
  const [openedAt, setOpenedAt] = useState(() => Date.now())
  const navigated = useRef(false)

  const deviceId = issued?.device.id ?? null

  const requestEnrollmentToken = (id: string) => {
    setEnrollmentError(false)
    createEnrollmentTokenApi(id)
      .then(setEnrollment)
      .catch(() => setEnrollmentError(true))
  }

  // Resolve the proxy listener address once per dialog from /api/config
  // (same source as the header's proxy address display), reset the
  // connect-tracking state, and fetch a fresh enrollment token for the QR.
  useEffect(() => {
    if (!deviceId) return
    setOpenedAt(Date.now())
    navigated.current = false
    setEnrollment(null)
    setEnrollmentError(false)
    apiGet<{ host?: string; proxy_port?: number; public_ip?: string }>("/config")
      .then((c) => {
        setHost(c.public_ip || c.host || window.location.hostname)
        setPort(c.proxy_port || 8888)
      })
      .catch(() => {
        setHost(window.location.hostname)
        setPort(8888)
      })
    requestEnrollmentToken(deviceId)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [deviceId])

  // One-second tick for the enrollment-token countdown.
  useEffect(() => {
    if (!issued) return
    const t = setInterval(() => setNowMs(Date.now()), 1000)
    return () => clearInterval(t)
  }, [issued])

  const secondsLeft = enrollment
    ? Math.max(0, Math.floor((Date.parse(enrollment.expires_at) - nowMs) / 1000))
    : 0
  const qrExpired = !!enrollment && secondsLeft <= 0

  const connectUri = enrollment && !qrExpired
    ? buildConnectUri({
        host,
        port,
        tls: false,
        name: issued?.device.name ?? "",
        token: enrollment.token,
      })
    : ""

  // Live status loop: an attributed entry that arrived after this dialog
  // opened means the device connected (manual apply or token redemption
  // both land here — the entries carry the device_id either way).
  const connected = lastCaptureAt !== null && lastCaptureAt >= openedAt

  // Auto-navigate to the device's traffic view on first connect (the
  // AppShell handles the event and syncs the shareable ?device= URL).
  useEffect(() => {
    if (!issued || !connected || navigated.current) return
    navigated.current = true
    const deviceId = issued.device.id
    const t = setTimeout(() => {
      window.dispatchEvent(
        new CustomEvent("madhyamas:view-device-traffic", { detail: { device: deviceId } }),
      )
    }, CONNECT_NAV_DELAY_MS)
    return () => clearTimeout(t)
  }, [issued, connected])

  return (
    <Dialog open={!!issued} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>Connect Your Device</DialogTitle>
          <DialogDescription>
            Scan the QR to enroll, or apply the manual values below. The QR carries a
            single-use enrollment token — it expires and cannot be reused.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="flex flex-col items-center gap-2 rounded-md border border-border p-3">
            {connectUri ? (
              <>
                <QRCodeSVG
                  value={connectUri}
                  size={160}
                  level="M"
                  bgColor="#ffffff"
                  fgColor="#000000"
                />
                <p className="text-2xs text-muted-foreground">
                  Scan to connect — expires in {formatCountdown(secondsLeft)}
                </p>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => issued && requestEnrollmentToken(issued.device.id)}
                  title="Issue a fresh enrollment token (the old one stays valid until it expires or is used)"
                >
                  <RefreshCw className="mr-1 h-3 w-3" /> New QR
                </Button>
              </>
            ) : qrExpired ? (
              <div className="flex h-40 w-40 flex-col items-center justify-center gap-2 text-center">
                <p className="text-2xs text-muted-foreground">QR expired</p>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => issued && requestEnrollmentToken(issued.device.id)}
                >
                  <RefreshCw className="mr-1 h-3 w-3" /> Generate new QR
                </Button>
              </div>
            ) : enrollmentError ? (
              <div className="flex h-40 w-40 flex-col items-center justify-center gap-2 text-center">
                <p className="text-2xs text-muted-foreground">
                  QR unavailable — use the manual values below.
                </p>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => issued && requestEnrollmentToken(issued.device.id)}
                >
                  <RefreshCw className="mr-1 h-3 w-3" /> Retry
                </Button>
              </div>
            ) : (
              <div className="h-40 w-40 animate-pulse rounded bg-muted" />
            )}
            {connected ? (
              <div className="flex items-center gap-2 rounded-md border border-success/30 bg-success/10 p-2 text-2xs font-medium text-success">
                <Check className="h-3.5 w-3.5 shrink-0" />
                Connected — capturing. Opening this device&apos;s traffic view…
              </div>
            ) : (
              <div className="flex items-center gap-2 text-2xs text-muted-foreground">
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                Waiting for device…
              </div>
            )}
          </div>
          <div className="space-y-2 rounded-md border border-border p-3">
            <p className="text-2xs font-medium text-muted-foreground">
              Manual proxy configuration (fallback — values shown once)
            </p>
            <CopyRow label="Password" value={issued?.key ?? ""} />
            <CopyRow label="Host" value={host} />
            <CopyRow label="Port" value={String(port)} />
            <CopyRow label="Username" value={issued?.device.name ?? ""} />
            <div className="flex items-center justify-between gap-2 pt-1">
              <div className="flex items-center gap-2 text-2xs text-warning">
                <Camera className="h-3.5 w-3.5 shrink-0" />
                Do not photograph or screenshot — anyone with the password
                captures this device&apos;s traffic.
              </div>
              <Button
                size="sm"
                variant="outline"
                onClick={() => issued && onRotate(issued.device.id)}
                disabled={rotatePending || issued?.device.status === "revoked"}
                title="Rotate now if the credential was exposed"
              >
                {rotatePending ? (
                  <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                ) : (
                  <RefreshCw className="mr-1 h-3 w-3" />
                )}
                Rotate
              </Button>
            </div>
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
