import { useState, useMemo } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Switch } from '@/components/ui/switch'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu'
import {
  useBlockList,
  useBlockListStats,
  useCreateBlockListEntry,
  useUpdateBlockListEntry,
  useDeleteBlockListEntry,
  useToggleBlockListEntry,
  type BlockListEntry,
} from '@/lib/api/intercept'
import { useToast } from '@/components/ui/use-toast'
import {
  Search,
  Plus,
  MoreVertical,
  Layers,
  Check,
  X,
  Shield,
  ShieldOff,
  Trash2,
  Edit,
} from 'lucide-react'

// Quick-add templates for common block patterns
const BLOCK_TEMPLATES: {
  name: string
  pattern: string
  statusCode: number
  contentType: string
  body: string
}[] = [
  {
    name: '403 Forbidden',
    pattern: '',
    statusCode: 403,
    contentType: 'text/plain',
    body: 'Blocked by Madhyamas',
  },
  {
    name: '503 Service Unavailable',
    pattern: '',
    statusCode: 503,
    contentType: 'application/json',
    body: '{"error":"Service Unavailable"}',
  },
  {
    name: '451 Unavailable for Legal Reasons',
    pattern: '',
    statusCode: 451,
    contentType: 'text/plain',
    body: 'Unavailable For Legal Reasons',
  },
  {
    name: '404 Not Found',
    pattern: '',
    statusCode: 404,
    contentType: 'application/json',
    body: '{"error":"Not Found"}',
  },
]

export function BlockListPanel() {
  const { data: entries, isLoading } = useBlockList()
  const { data: stats } = useBlockListStats()
  const createEntry = useCreateBlockListEntry()
  const updateEntry = useUpdateBlockListEntry()
  const deleteEntry = useDeleteBlockListEntry()
  const toggleEntry = useToggleBlockListEntry()
  const { toast } = useToast()

  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [editingEntry, setEditingEntry] = useState<BlockListEntry | null>(null)
  const [searchTerm, setSearchTerm] = useState('')
  const [selectedTemplate, setSelectedTemplate] = useState<number | null>(null)
  const [newEntry, setNewEntry] = useState({
    pattern: '',
    note: '',
    statusCode: 403,
    contentType: 'text/plain',
    body: 'Blocked by Madhyamas',
  })

  // Filter entries based on search term
  const filteredEntries = useMemo(() => {
    if (!entries) return []
    if (!searchTerm) return entries

    const term = searchTerm.toLowerCase()
    return entries.filter(
      (entry) =>
        entry.pattern.toLowerCase().includes(term) ||
        (entry.note ?? '').toLowerCase().includes(term) ||
        entry.status_code.toString().includes(term),
    )
  }, [entries, searchTerm])

  const handleCreate = async () => {
    if (!newEntry.pattern.trim()) {
      toast({
        title: 'Error',
        description: 'Pattern is required',
        variant: 'destructive',
      })
      return
    }

    await createEntry.mutateAsync({
      pattern: newEntry.pattern,
      note: newEntry.note || undefined,
      status_code: newEntry.statusCode,
      content_type: newEntry.contentType,
      response_body: newEntry.body,
    })

    setShowCreateDialog(false)
    setNewEntry({
      pattern: '',
      note: '',
      statusCode: 403,
      contentType: 'text/plain',
      body: 'Blocked by Madhyamas',
    })
    setSelectedTemplate(null)

    toast({
      title: 'Block Entry Created',
      description: `Requests to "${newEntry.pattern}" will be blocked`,
    })
  }

  const handleToggle = async (id: string, enabled: boolean, pattern: string) => {
    await toggleEntry.mutateAsync({ id, enabled })
    toast({
      title: enabled ? 'Block Entry Enabled' : 'Block Entry Disabled',
      description: `"${pattern}" ${enabled ? 'will now block requests' : 'will no longer block requests'}`,
    })
  }

  const handleDelete = async (id: string, pattern: string) => {
    await deleteEntry.mutateAsync(id)
    toast({
      title: 'Block Entry Deleted',
      description: `"${pattern}" has been removed from the block list`,
    })
  }

  const handleTemplateSelect = (index: number) => {
    const template = BLOCK_TEMPLATES[index]
    setSelectedTemplate(index)
    setNewEntry({
      ...newEntry,
      statusCode: template.statusCode,
      contentType: template.contentType,
      body: template.body,
    })
  }

  const handleEnableAll = async () => {
    if (!entries) return
    for (const entry of entries) {
      if (!entry.enabled) {
        await toggleEntry.mutateAsync({ id: entry.id, enabled: true })
      }
    }
    toast({ description: 'All block entries enabled' })
  }

  const handleDisableAll = async () => {
    if (!entries) return
    for (const entry of entries) {
      if (entry.enabled) {
        await toggleEntry.mutateAsync({ id: entry.id, enabled: false })
      }
    }
    toast({ description: 'All block entries disabled' })
  }

  const handleExport = () => {
    if (!entries || entries.length === 0) {
      toast({ title: 'No block entries to export', variant: 'destructive' })
      return
    }
    const data = JSON.stringify(entries, null, 2)
    const blob = new Blob([data], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'madhyamas-blocklist.json'
    a.click()
    URL.revokeObjectURL(url)
    toast({ description: 'Block list exported' })
  }

  const handleSaveEdit = async () => {
    if (!editingEntry) return
    if (!editingEntry.pattern.trim()) {
      toast({
        title: 'Error',
        description: 'Pattern cannot be empty',
        variant: 'destructive',
      })
      return
    }

    await updateEntry.mutateAsync({ id: editingEntry.id, entry: editingEntry })
    setEditingEntry(null)
    toast({
      title: 'Block Entry Updated',
      description: `"${editingEntry.pattern}" has been updated`,
    })
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading block list...</div>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Block List</h2>
            <p className="text-xs text-muted-foreground">
              {stats?.enabled ?? 0}/{stats?.total ?? 0} enabled •{' '}
              {stats?.total_hits ?? 0} total blocks
            </p>
          </div>
          <div className="flex items-center gap-2">
            {/* Bulk Actions */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm">
                  <Layers className="h-4 w-4 mr-1" />
                  Bulk
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={handleEnableAll}>
                  <Check className="h-4 w-4 mr-2" />
                  Enable All
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleDisableAll}>
                  <X className="h-4 w-4 mr-2" />
                  Disable All
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={handleExport}>
                  Export
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Create Button */}
            <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
              <DialogTrigger asChild>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-1" />
                  Block
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-lg">
                <DialogHeader>
                  <DialogTitle>Block a Domain</DialogTitle>
                  <DialogDescription>
                    Requests to matching hosts will be blocked with a custom
                    response instead of forwarding upstream.
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  {/* Templates */}
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">
                      Quick Templates
                    </label>
                    <div className="flex flex-wrap gap-1">
                      {BLOCK_TEMPLATES.map((template, index) => (
                        <Button
                          key={template.name}
                          variant={
                            selectedTemplate === index ? 'default' : 'outline'
                          }
                          size="sm"
                          onClick={() => handleTemplateSelect(index)}
                        >
                          {template.name}
                        </Button>
                      ))}
                    </div>
                  </div>

                  {/* Pattern */}
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">
                      Pattern <span className="text-destructive">*</span>
                    </label>
                    <Input
                      placeholder="ads.example.com, *.tracker.com, *ads*"
                      value={newEntry.pattern}
                      onChange={(e) =>
                        setNewEntry({ ...newEntry, pattern: e.target.value })
                      }
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') handleCreate()
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Exact domain (matches subdomains),{' '}
                      <code>*.example.com</code> (subdomains only), or{' '}
                      <code>*ads*</code> (glob match)
                    </p>
                  </div>

                  {/* Note */}
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Note (optional)</label>
                    <Input
                      placeholder="Block ad server"
                      value={newEntry.note}
                      onChange={(e) =>
                        setNewEntry({ ...newEntry, note: e.target.value })
                      }
                    />
                  </div>

                  {/* Status Code */}
                  <div className="grid grid-cols-2 gap-4">
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">Status Code</label>
                      <Input
                        type="number"
                        value={newEntry.statusCode}
                        onChange={(e) =>
                          setNewEntry({
                            ...newEntry,
                            statusCode: parseInt(e.target.value) || 403,
                          })
                        }
                      />
                    </div>
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">Content Type</label>
                      <Select
                        value={newEntry.contentType}
                        onValueChange={(v) =>
                          setNewEntry({ ...newEntry, contentType: v })
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="text/plain">text/plain</SelectItem>
                          <SelectItem value="application/json">
                            application/json
                          </SelectItem>
                          <SelectItem value="text/html">text/html</SelectItem>
                          <SelectItem value="application/xml">
                            application/xml
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>

                  {/* Response Body */}
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Response Body</label>
                    <Input
                      placeholder="Blocked by Madhyamas"
                      value={newEntry.body}
                      onChange={(e) =>
                        setNewEntry({ ...newEntry, body: e.target.value })
                      }
                    />
                  </div>
                </div>
                <DialogFooter>
                  <Button
                    variant="outline"
                    onClick={() => setShowCreateDialog(false)}
                  >
                    Cancel
                  </Button>
                  <Button
                    onClick={handleCreate}
                    disabled={createEntry.isPending}
                  >
                    Block
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        </div>

        {/* Search */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search block list..."
            className="pl-9"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>
      </div>

      {/* List */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-3">
          {filteredEntries.length === 0 && (
            <div className="text-center text-muted-foreground py-8">
              {searchTerm
                ? 'No block entries match your search'
                : 'No domains blocked. Click "Block" to prevent requests from reaching specific hosts.'}
            </div>
          )}

          {filteredEntries.map((entry) => (
            <div
              key={entry.id}
              className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50 transition-colors"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  {entry.enabled ? (
                    <Shield className="h-4 w-4 text-red-500 flex-shrink-0" />
                  ) : (
                    <ShieldOff className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                  )}
                  <span className="font-medium font-mono truncate">
                    {entry.pattern}
                  </span>
                  <span className="text-xs px-2 py-0.5 rounded bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300 flex-shrink-0">
                    {entry.status_code}
                  </span>
                  {entry.hit_count > 0 && (
                    <span className="text-xs text-muted-foreground flex-shrink-0">
                      {entry.hit_count} {entry.hit_count === 1 ? 'hit' : 'hits'}
                    </span>
                  )}
                </div>
                {entry.note && (
                  <div className="text-sm text-muted-foreground truncate ml-6">
                    {entry.note}
                  </div>
                )}
                {entry.response_body !== 'Blocked by Madhyamas' && (
                  <div className="text-xs text-muted-foreground/70 truncate ml-6 font-mono">
                    Body: {entry.response_body}
                  </div>
                )}
              </div>
              <div className="flex items-center gap-2 flex-shrink-0">
                <Switch
                  checked={entry.enabled}
                  onCheckedChange={(checked) =>
                    handleToggle(entry.id, checked, entry.pattern)
                  }
                />
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-8 w-8 p-0"
                    >
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      onClick={() => {
                        setEditingEntry({ ...entry })
                      }}
                    >
                      <Edit className="h-4 w-4 mr-2" />
                      Edit
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      className="text-destructive"
                      onClick={() => handleDelete(entry.id, entry.pattern)}
                    >
                      <Trash2 className="h-4 w-4 mr-2" />
                      Delete
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          ))}
        </div>
      </ScrollArea>

      {/* Edit Dialog */}
      <Dialog
        open={!!editingEntry}
        onOpenChange={(open) => !open && setEditingEntry(null)}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Edit Block Entry</DialogTitle>
            <DialogDescription>
              Update the pattern, response, or note for this block entry.
            </DialogDescription>
          </DialogHeader>
          {editingEntry && (
            <div className="grid gap-4 py-4">
              <div className="grid gap-2">
                <label className="text-sm font-medium">
                  Pattern <span className="text-destructive">*</span>
                </label>
                <Input
                  value={editingEntry.pattern}
                  onChange={(e) =>
                    setEditingEntry({
                      ...editingEntry,
                      pattern: e.target.value,
                      updated_at: new Date().toISOString(),
                    })
                  }
                />
              </div>
              <div className="grid gap-2">
                <label className="text-sm font-medium">Note</label>
                <Input
                  value={editingEntry.note ?? ''}
                  onChange={(e) =>
                    setEditingEntry({
                      ...editingEntry,
                      note: e.target.value || null,
                      updated_at: new Date().toISOString(),
                    })
                  }
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="grid gap-2">
                  <label className="text-sm font-medium">Status Code</label>
                  <Input
                    type="number"
                    value={editingEntry.status_code}
                    onChange={(e) =>
                      setEditingEntry({
                        ...editingEntry,
                        status_code: parseInt(e.target.value) || 403,
                        updated_at: new Date().toISOString(),
                      })
                    }
                  />
                </div>
                <div className="grid gap-2">
                  <label className="text-sm font-medium">Content Type</label>
                  <Input
                    value={editingEntry.content_type}
                    onChange={(e) =>
                      setEditingEntry({
                        ...editingEntry,
                        content_type: e.target.value,
                        updated_at: new Date().toISOString(),
                      })
                    }
                  />
                </div>
              </div>
              <div className="grid gap-2">
                <label className="text-sm font-medium">Response Body</label>
                <Input
                  value={editingEntry.response_body}
                  onChange={(e) =>
                    setEditingEntry({
                      ...editingEntry,
                      response_body: e.target.value,
                      updated_at: new Date().toISOString(),
                    })
                  }
                />
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditingEntry(null)}>
              Cancel
            </Button>
            <Button
              onClick={handleSaveEdit}
              disabled={updateEntry.isPending}
            >
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
