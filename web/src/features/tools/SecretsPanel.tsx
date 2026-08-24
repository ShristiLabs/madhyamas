import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useToast } from '@/components/ui/use-toast'
import { useSecrets, useSetSecret, useDeleteSecret } from '@/lib/api/secrets'
import { KeyRound, Plus, Trash2 } from 'lucide-react'

/**
 * Secrets management panel (issue #87).
 *
 * Secret values are write-only: this panel lists names only, and setting a
 * secret sends the value once — it is never displayed or echoed back.
 * Grants are declared per-plugin in `madhyamas-plugin.toml`
 * (`env_grants` / `secret_grants`) and per-script via the scripts API.
 * In the enterprise tier these endpoints are RBAC-gated to admins.
 */
export function SecretsPanel() {
  const { data, isLoading, error } = useSecrets()
  const setSecret = useSetSecret()
  const deleteSecret = useDeleteSecret()
  const { toast } = useToast()

  const [newName, setNewName] = useState('')
  const [newValue, setNewValue] = useState('')
  const [updating, setUpdating] = useState<string | null>(null)
  const [updateValue, setUpdateValue] = useState('')

  const handleCreate = async () => {
    if (!newName.trim() || !newValue) return
    try {
      await setSecret.mutateAsync({ name: newName.trim(), value: newValue })
      toast({ title: 'Secret Saved', description: `Secret "${newName.trim()}" stored (value hidden).` })
      setNewName('')
      setNewValue('')
    } catch (e) {
      toast({ title: 'Error', description: String(e), variant: 'destructive' })
    }
  }

  const handleUpdate = async (name: string) => {
    if (!updateValue) return
    try {
      await setSecret.mutateAsync({ name, value: updateValue })
      toast({ title: 'Secret Updated', description: `Secret "${name}" updated (value hidden).` })
      setUpdating(null)
      setUpdateValue('')
    } catch (e) {
      toast({ title: 'Error', description: String(e), variant: 'destructive' })
    }
  }

  const handleDelete = async (name: string) => {
    try {
      await deleteSecret.mutateAsync(name)
      toast({ title: 'Secret Deleted', description: `Secret "${name}" removed.` })
    } catch (e) {
      toast({ title: 'Error', description: String(e), variant: 'destructive' })
    }
  }

  return (
    <div className="flex h-full flex-col">
      <div className="shrink-0 border-b border-border px-3 py-2">
        <div className="flex items-center gap-2">
          <KeyRound className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="text-2xs font-semibold uppercase tracking-wider text-muted-foreground">
            Secrets (values write-only)
          </span>
        </div>
        <p className="mt-1 text-2xs text-muted-foreground">
          Reference in plugin settings or script source as {'${SECRET:name}'} — only names granted
          to a plugin/script are substituted. Values are redacted from traffic capture, HAR export,
          plugin logs, and script traces.
        </p>
      </div>

      <div className="shrink-0 space-y-1.5 border-b border-border px-3 py-2">
        <div className="flex items-center gap-1.5">
          <Input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="name (e.g. api_token)"
            className="h-7 flex-1 text-2xs"
          />
          <Input
            type="password"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="value"
            className="h-7 flex-1 text-2xs"
          />
          <Button size="sm" variant="outline" className="h-7" onClick={handleCreate} disabled={!newName.trim() || !newValue}>
            <Plus className="mr-1 h-3 w-3" /> Add
          </Button>
        </div>
      </div>

      <ScrollArea className="min-h-0 flex-1">
        <div className="space-y-1.5 px-3 py-2">
          {isLoading && <div className="text-2xs text-muted-foreground">Loading…</div>}
          {error && (
            <div className="text-2xs text-destructive">
              Failed to load secrets: {String(error)}
            </div>
          )}
          {data?.names.length === 0 && !isLoading && (
            <div className="text-2xs text-muted-foreground">
              No secrets defined. Add one above.
            </div>
          )}
          {data?.names.map((name) => (
            <div key={name} className="rounded border border-border px-2 py-1.5">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <KeyRound className="h-3 w-3 text-muted-foreground" />
                  <span className="font-mono text-2xs">{name}</span>
                  <span className="text-2xs text-muted-foreground">value hidden</span>
                </div>
                <div className="flex items-center gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-6 text-2xs"
                    onClick={() => setUpdating(updating === name ? null : name)}
                  >
                    Update
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-6 text-2xs text-destructive"
                    onClick={() => handleDelete(name)}
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              </div>
              {updating === name && (
                <div className="mt-1.5 flex items-center gap-1.5">
                  <Input
                    type="password"
                    value={updateValue}
                    onChange={(e) => setUpdateValue(e.target.value)}
                    placeholder="new value"
                    className="h-7 flex-1 text-2xs"
                  />
                  <Button size="sm" variant="outline" className="h-7" onClick={() => handleUpdate(name)} disabled={!updateValue}>
                    Save
                  </Button>
                </div>
              )}
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  )
}
