/**
 * ApiKeysPanel — API key management with create/revoke.
 *
 * On key creation, the plaintext key is shown ONCE with a copy button
 * and a warning that it won't be shown again.
 * API: GET/POST /api/auth/api-keys, DELETE /api/auth/api-keys/:id.
 */
import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Loader2, Plus, Trash2, Copy, AlertTriangle, Check } from "lucide-react"
import {
  listApiKeysApi,
  createApiKeyApi,
  revokeApiKeyApi,
  type ApiKeyEntry,
  type CreateApiKeyPayload,
} from "@/lib/api/admin"
import { useToast } from "@/components/ui/use-toast"
import { ApiError } from "@/lib/api/client"

const SCOPES = ["traffic:read", "traffic:write", "mocks:read", "mocks:write", "config:read", "config:write", "*"]
const EXPIRY_OPTIONS = [
  { label: "Never", value: 0 },
  { label: "7 days", value: 7 },
  { label: "30 days", value: 30 },
  { label: "90 days", value: 90 },
  { label: "365 days", value: 365 },
]

export function ApiKeysPanel() {
  const { toast } = useToast()
  const queryClient = useQueryClient()
  const { data: keys, isLoading } = useQuery({
    queryKey: ["admin-api-keys"],
    queryFn: listApiKeysApi,
  })

  const [createOpen, setCreateOpen] = useState(false)
  const [newKey, setNewKey] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const [revokeKey, setRevokeKey] = useState<ApiKeyEntry | null>(null)

  const createMut = useMutation({
    mutationFn: (data: CreateApiKeyPayload) => createApiKeyApi(data),
    onSuccess: (key) => {
      queryClient.invalidateQueries({ queryKey: ["admin-api-keys"] })
      setCreateOpen(false)
      setNewKey(key.key)
      setCopied(false)
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to create API key",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  const revokeMut = useMutation({
    mutationFn: (id: string) => revokeApiKeyApi(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin-api-keys"] })
      setRevokeKey(null)
      toast({ title: "API key revoked" })
    },
    onError: (e: unknown) => {
      toast({
        title: "Failed to revoke key",
        description: e instanceof ApiError ? e.body : "Unknown error",
        variant: "destructive",
      })
    },
  })

  const handleCopy = async () => {
    if (!newKey) return
    try {
      await navigator.clipboard.writeText(newKey)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast({ title: "Copy failed", variant: "destructive" })
    }
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading API keys…
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex items-center justify-between border-b border-border px-4 py-2">
        <h2 className="text-sm font-semibold">API Keys</h2>
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="mr-1 h-3.5 w-3.5" /> Create Key
        </Button>
      </div>

      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 bg-card text-left text-muted-foreground">
            <tr className="border-b border-border">
              <th className="px-4 py-2 font-medium">Name</th>
              <th className="px-4 py-2 font-medium">Prefix</th>
              <th className="px-4 py-2 font-medium">Scopes</th>
              <th className="px-4 py-2 font-medium">Created</th>
              <th className="px-4 py-2 font-medium">Expires</th>
              <th className="px-4 py-2 font-medium">Last Used</th>
              <th className="px-4 py-2 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {keys?.map((k) => (
              <tr key={k.id} className="border-b border-border/50 hover:bg-muted/30">
                <td className="px-4 py-2 font-medium">{k.name}</td>
                <td className="px-4 py-2 font-mono text-2xs text-muted-foreground">
                  {k.key.slice(0, 12)}…
                </td>
                <td className="px-4 py-2">
                  <div className="flex flex-wrap gap-1">
                    {k.scopes.map((s) => (
                      <span key={s} className="rounded bg-primary/10 px-1 py-0.5 text-2xs text-primary">
                        {s}
                      </span>
                    ))}
                  </div>
                </td>
                <td className="px-4 py-2 text-muted-foreground">
                  {new Date(k.created_at * 1000).toLocaleDateString()}
                </td>
                <td className="px-4 py-2 text-muted-foreground">
                  {k.expires_at ? new Date(k.expires_at * 1000).toLocaleDateString() : "Never"}
                </td>
                <td className="px-4 py-2 text-muted-foreground">
                  {k.last_used ? new Date(k.last_used * 1000).toLocaleDateString() : "—"}
                </td>
                <td className="px-4 py-2">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={() => setRevokeKey(k)}
                    title="Revoke key"
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </td>
              </tr>
            ))}
            {keys?.length === 0 && (
              <tr>
                <td colSpan={7} className="px-4 py-8 text-center text-muted-foreground">
                  No API keys found.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <CreateKeyDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        onSubmit={(d) => createMut.mutate(d)}
        loading={createMut.isPending}
      />

      <Dialog open={!!newKey} onOpenChange={(open) => !open && setNewKey(null)}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>API Key Created</DialogTitle>
            <DialogDescription>
              Copy your API key now. It won't be shown again.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="flex items-center gap-2 rounded-md border border-warning/30 bg-warning/10 p-3 text-xs text-warning">
              <AlertTriangle className="h-4 w-4 shrink-0" />
              Store this key securely. You will not be able to see it again.
            </div>
            <div className="flex items-center gap-2">
              <code className="flex-1 truncate rounded bg-muted p-2 font-mono text-2xs">
                {newKey}
              </code>
              <Button size="sm" variant="outline" onClick={() => void handleCopy()}>
                {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
              </Button>
            </div>
          </div>
          <DialogFooter>
            <Button onClick={() => setNewKey(null)}>Done</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!revokeKey} onOpenChange={(open) => !open && setRevokeKey(null)}>
        <DialogContent className="sm:max-w-[400px]">
          <DialogHeader>
            <DialogTitle>Revoke API Key</DialogTitle>
            <DialogDescription>
              Are you sure you want to revoke <strong>{revokeKey?.name}</strong>?
              This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRevokeKey(null)}>Cancel</Button>
            <Button
              variant="destructive"
              onClick={() => revokeKey && revokeMut.mutate(revokeKey.id)}
              disabled={revokeMut.isPending}
            >
              {revokeMut.isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Revoke
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

function CreateKeyDialog({ open, onOpenChange, onSubmit, loading }: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: CreateApiKeyPayload) => void
  loading: boolean
}) {
  const [name, setName] = useState("")
  const [scopes, setScopes] = useState<string[]>([])
  const [expiry, setExpiry] = useState(0)

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    const data: CreateApiKeyPayload = { name, scopes }
    if (expiry > 0) data.expires_in_days = expiry
    onSubmit(data)
    setName("")
    setScopes([])
    setExpiry(0)
  }

  const toggleScope = (s: string) => {
    setScopes((prev) =>
      prev.includes(s) ? prev.filter((x) => x !== s) : [...prev, s]
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>Create API Key</DialogTitle>
          <DialogDescription>Generate a new API key with scoped permissions.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="ak-name">Name</Label>
            <Input id="ak-name" value={name} onChange={(e) => setName(e.target.value)} required placeholder="e.g. CI pipeline" />
          </div>
          <div className="space-y-1.5">
            <Label>Scopes</Label>
            <div className="flex flex-wrap gap-1.5">
              {SCOPES.map((s) => (
                <button
                  key={s}
                  type="button"
                  onClick={() => toggleScope(s)}
                  className={
                    scopes.includes(s)
                      ? "rounded bg-primary px-2 py-1 text-2xs font-medium text-primary-foreground"
                      : "rounded border border-border bg-muted px-2 py-1 text-2xs text-muted-foreground hover:bg-accent"
                  }
                >
                  {s}
                </button>
              ))}
            </div>
          </div>
          <div className="space-y-1.5">
            <Label>Expiry</Label>
            <Select value={String(expiry)} onValueChange={(v) => setExpiry(Number(v))}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {EXPIRY_OPTIONS.map((o) => (
                  <SelectItem key={o.value} value={String(o.value)}>{o.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <DialogFooter>
            <Button type="submit" disabled={loading}>
              {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Create
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
