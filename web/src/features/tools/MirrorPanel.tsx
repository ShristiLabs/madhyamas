import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useToast } from '@/components/ui/use-toast'
import {
  useMirrorStatus,
  useToggleMirror,
  useUpdateMirrorConfig,
} from '@/lib/api/mirror'
import { HardDriveDownload, Save, Trash2 } from 'lucide-react'

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
}

export function MirrorPanel() {
  const { data: status, isLoading } = useMirrorStatus()
  const toggleMirror = useToggleMirror()
  const updateConfig = useUpdateMirrorConfig()
  const { toast } = useToast()

  const [enabled, setEnabled] = useState(false)
  const [outputDir, setOutputDir] = useState('')
  const [hostFilter, setHostFilter] = useState('')
  const [saveRequestBodies, setSaveRequestBodies] = useState(false)

  useEffect(() => {
    if (status) {
      setEnabled(status.enabled)
      setOutputDir(status.output_dir)
      setHostFilter(status.host_filter?.join(', ') ?? '')
      setSaveRequestBodies(status.save_request_bodies)
    }
  }, [status])

  const handleToggle = async (checked: boolean) => {
    setEnabled(checked)
    try {
      await toggleMirror.mutateAsync({ enabled: checked })
      toast({
        title: checked ? 'Mirroring Enabled' : 'Mirroring Disabled',
        description: checked
          ? 'Response bodies will be saved to disk'
          : 'Response bodies will no longer be saved',
      })
    } catch (e) {
      setEnabled(!checked)
      toast({
        title: 'Error',
        description: String(e),
        variant: 'destructive',
      })
    }
  }

  const handleSaveConfig = async () => {
    const payload: {
      output_dir?: string
      host_filter?: string[] | null
      save_request_bodies?: boolean
    } = {}

    if (outputDir.trim()) {
      payload.output_dir = outputDir.trim()
    }

    const trimmedFilter = hostFilter.trim()
    if (trimmedFilter === '' || trimmedFilter.toLowerCase() === 'none') {
      payload.host_filter = null
    } else {
      payload.host_filter = trimmedFilter
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0)
    }

    payload.save_request_bodies = saveRequestBodies

    try {
      await updateConfig.mutateAsync(payload)
      toast({
        title: 'Mirror Configuration Saved',
        description: 'Configuration has been updated and persisted',
      })
    } catch (e) {
      toast({
        title: 'Error',
        description: String(e),
        variant: 'destructive',
      })
    }
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-muted-foreground">Loading mirror settings...</div>
      </div>
    )
  }

  return (
    <ScrollArea className="h-full">
      <div className="p-4 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <HardDriveDownload className="h-5 w-5 text-muted-foreground" />
            <h2 className="text-lg font-semibold">Mirror Tool</h2>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">Enable</span>
            <Switch checked={enabled} onCheckedChange={handleToggle} />
          </div>
        </div>

        {/* Description */}
        <p className="text-sm text-muted-foreground">
          Save response bodies to disk following the URL path structure
          (<code className="text-xs">output_dir/host/path/content</code>).
          A <code className="text-xs">.meta.json</code> sidecar is written
          alongside each file with request/response metadata.
        </p>

        {/* Statistics */}
        {status && (
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-lg border border-border bg-muted/30 p-3">
              <div className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                Files Written
              </div>
              <div className="mt-1 text-xl font-semibold tabular-nums">
                {status.files_written.toLocaleString()}
              </div>
            </div>
            <div className="rounded-lg border border-border bg-muted/30 p-3">
              <div className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
                Bytes Written
              </div>
              <div className="mt-1 text-xl font-semibold tabular-nums">
                {formatBytes(status.bytes_written)}
              </div>
            </div>
          </div>
        )}

        {/* Configuration */}
        <div className="space-y-4 rounded-lg border border-border p-4">
          <h3 className="text-sm font-semibold">Configuration</h3>

          {/* Output directory */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Output Directory</label>
            <Input
              value={outputDir}
              onChange={(e) => setOutputDir(e.target.value)}
              placeholder="~/.madhyamas/mirror"
              className="font-mono text-xs"
            />
            <p className="text-2xs text-muted-foreground">
              Directory where mirrored response bodies are written. Created if it does not exist.
            </p>
          </div>

          {/* Host filter */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Host Filter</label>
            <Input
              value={hostFilter}
              onChange={(e) => setHostFilter(e.target.value)}
              placeholder="(none — mirror all hosts)"
              className="font-mono text-xs"
            />
            <p className="text-2xs text-muted-foreground">
              Comma-separated host patterns to mirror (e.g.{' '}
              <code className="text-xs">*.example.com, api.test.com</code>).
              Leave empty to mirror all hosts.
            </p>
          </div>

          {/* Save request bodies */}
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Save Request Bodies</label>
              <p className="text-2xs text-muted-foreground">
                Also save request bodies alongside responses (as{' '}
                <code className="text-xs">.request</code> files)
              </p>
            </div>
            <Switch
              checked={saveRequestBodies}
              onCheckedChange={setSaveRequestBodies}
            />
          </div>

          {/* Save button */}
          <div className="flex justify-end gap-2 pt-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                if (status) {
                  setOutputDir(status.output_dir)
                  setHostFilter(status.host_filter?.join(', ') ?? '')
                  setSaveRequestBodies(status.save_request_bodies)
                }
              }}
            >
              <Trash2 className="mr-1.5 h-3.5 w-3.5" />
              Reset
            </Button>
            <Button size="sm" onClick={handleSaveConfig} disabled={updateConfig.isPending}>
              <Save className="mr-1.5 h-3.5 w-3.5" />
              {updateConfig.isPending ? 'Saving...' : 'Save Config'}
            </Button>
          </div>
        </div>

        {/* Path mapping reference */}
        <div className="space-y-2 rounded-lg border border-border bg-muted/20 p-4">
          <h3 className="text-sm font-semibold">Path Mapping</h3>
          <div className="space-y-1.5 text-xs">
            <div className="flex flex-col gap-0.5">
              <span className="text-muted-foreground">URL</span>
              <code className="rounded bg-muted px-1.5 py-0.5">
                https://api.example.com/v1/users/123
              </code>
            </div>
            <div className="flex flex-col gap-0.5">
              <span className="text-muted-foreground">File</span>
              <code className="rounded bg-muted px-1.5 py-0.5">
                output_dir/api.example.com/v1/users/123/index.json
              </code>
            </div>
          </div>
          <div className="space-y-1.5 text-xs pt-2">
            <div className="flex flex-col gap-0.5">
              <span className="text-muted-foreground">URL</span>
              <code className="rounded bg-muted px-1.5 py-0.5">
                https://cdn.example.com/assets/img/logo.png
              </code>
            </div>
            <div className="flex flex-col gap-0.5">
              <span className="text-muted-foreground">File</span>
              <code className="rounded bg-muted px-1.5 py-0.5">
                output_dir/cdn.example.com/assets/img/logo.png
              </code>
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  )
}
