import { useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  RotateCcw,
  Theater,
  Download,
  Trash2,
  X,
  Loader2,
} from 'lucide-react'
import {
  useCreateMockFromTraffic,
  useSaveRequestsFromTraffic,
} from '@/lib/api/intercept'
import { useToast } from '@/components/ui/use-toast'
import { cn } from '@/lib/utils'

interface TrafficActionBarProps {
  /** IDs of the currently selected traffic entries. */
  selectedIds: Set<string>
  /** Called when the user clears the selection. */
  onClear: () => void
  /** Called when the user requests an export of the selected entries. */
  onExport: () => void
  /** Called when the user requests clearing the selected entries. */
  onClearEntries: () => void
}

/**
 * Floating action bar that appears at the bottom of the traffic list when
 * one or more entries are selected. Provides bulk actions: save to replay,
 * create mock, export HAR, clear.
 */
export function TrafficActionBar({
  selectedIds,
  onClear,
  onExport,
  onClearEntries,
}: TrafficActionBarProps) {
  const createMockFromTraffic = useCreateMockFromTraffic()
  const saveRequestsFromTraffic = useSaveRequestsFromTraffic()
  const { toast } = useToast()
  const [showNameDialog, setShowNameDialog] = useState<
    'replay' | 'mock' | null
  >(null)
  const [namePrefix, setNamePrefix] = useState('')

  const count = selectedIds.size
  const ids = Array.from(selectedIds)

  if (count === 0) return null

  const handleSaveToReplay = async () => {
    try {
      const result = await saveRequestsFromTraffic.mutateAsync({
        entry_ids: ids,
        name_prefix: namePrefix || undefined,
      })
      toast({
        title: 'Saved to Replay',
        description:
          result.errors.length > 0
            ? `Saved ${result.saved} of ${result.total} requests (${result.errors.length} failed).`
            : `Saved ${result.saved} request${result.saved === 1 ? '' : 's'} to Replay.`,
      })
    } catch (e) {
      toast({
        title: 'Save Failed',
        description: String(e),
        variant: 'destructive',
      })
    }
    setShowNameDialog(null)
    setNamePrefix('')
    onClear()
  }

  const handleCreateMock = async () => {
    try {
      const result = await createMockFromTraffic.mutateAsync({
        entry_ids: ids,
        name_prefix: namePrefix || undefined,
      })
      toast({
        title: 'Mock Created',
        description:
          result.errors.length > 0
            ? `Created ${result.created} of ${result.total} mocks (${result.errors.length} failed).`
            : `Created ${result.created} mock${result.created === 1 ? '' : 's'} from traffic.`,
      })
    } catch (e) {
      toast({
        title: 'Mock Creation Failed',
        description: String(e),
        variant: 'destructive',
      })
    }
    setShowNameDialog(null)
    setNamePrefix('')
    onClear()
  }

  const isPending =
    createMockFromTraffic.isPending || saveRequestsFromTraffic.isPending

  return (
    <>
      <div
        className={cn(
          'absolute bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1',
          'rounded-lg border border-border bg-background p-1.5 shadow-lg',
          'animate-in fade-in slide-in-from-bottom-2 duration-200',
        )}
      >
        <span className="px-2 text-sm font-medium">
          {count} selected
        </span>
        <div className="mx-1 h-5 w-px bg-border" />
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setNamePrefix('')
            setShowNameDialog('replay')
          }}
          disabled={isPending}
          title="Save selected requests for replay"
        >
          {saveRequestsFromTraffic.isPending ? (
            <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
          ) : (
            <RotateCcw className="mr-1.5 h-3.5 w-3.5" />
          )}
          Save to Replay
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setNamePrefix('')
            setShowNameDialog('mock')
          }}
          disabled={isPending}
          title="Create mock responses from selected traffic"
        >
          {createMockFromTraffic.isPending ? (
            <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
          ) : (
            <Theater className="mr-1.5 h-3.5 w-3.5" />
          )}
          Create Mock
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={onExport}
          title="Export selected as HAR"
        >
          <Download className="mr-1.5 h-3.5 w-3.5" />
          Export
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={onClearEntries}
          title="Clear selected entries"
        >
          <Trash2 className="mr-1.5 h-3.5 w-3.5" />
          Clear
        </Button>
        <div className="mx-1 h-5 w-px bg-border" />
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onClear}
          title="Deselect all"
        >
          <X className="h-3.5 w-3.5" />
        </Button>
      </div>

      {/* Name prefix dialog — shown for Save to Replay / Create Mock */}
      {showNameDialog && (
        <div
          className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50"
          onClick={() => setShowNameDialog(null)}
        >
          <div
            className="w-96 rounded-lg border border-border bg-background p-4 shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="mb-3 text-sm font-semibold">
              {showNameDialog === 'replay'
                ? 'Save to Replay'
                : 'Create Mock'}
            </h3>
            <label className="mb-1 block text-xs text-muted-foreground">
              Name prefix (optional)
            </label>
            <input
              type="text"
              autoFocus
              value={namePrefix}
              onChange={(e) => setNamePrefix(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  if (showNameDialog === 'replay') {
                    handleSaveToReplay()
                  } else {
                    handleCreateMock()
                  }
                }
                if (e.key === 'Escape') setShowNameDialog(null)
              }}
              placeholder={
                showNameDialog === 'replay'
                  ? 'e.g. Login flow'
                  : 'e.g. API v2'
              }
              className="mb-3 w-full rounded-md border border-border bg-transparent px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
            />
            <p className="mb-3 text-xs text-muted-foreground">
              {count} request{count === 1 ? '' : 's'} will be saved with names
              like "{namePrefix || (showNameDialog === 'replay' ? 'GET' : 'Mock')}: METHOD URL".
            </p>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowNameDialog(null)}
              >
                Cancel
              </Button>
              <Button
                size="sm"
                onClick={
                  showNameDialog === 'replay'
                    ? handleSaveToReplay
                    : handleCreateMock
                }
                disabled={isPending}
              >
                {isPending && <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />}
                {showNameDialog === 'replay' ? 'Save' : 'Create'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
