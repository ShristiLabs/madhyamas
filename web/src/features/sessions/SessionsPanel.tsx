import { useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  useSessions,
  useCreateSession,
  useDeleteSession,
  useSwitchSession,
  useImportSession,
  exportSession,
  type Session,
} from '@/lib/api/sessions';
import { useToast } from '@/components/ui/use-toast';
import {
  Plus,
  Upload,
  MoreVertical,
  Trash2,
  Download,
  ArrowRightLeft,
  FolderTree,
  Loader2,
} from 'lucide-react';

export function SessionsPanel() {
  const { data: sessions, isLoading } = useSessions();
  const createSession = useCreateSession();
  const deleteSession = useDeleteSession();
  const switchSession = useSwitchSession();
  const importSession = useImportSession();
  const { toast } = useToast();

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newSession, setNewSession] = useState({ name: '', description: '' });
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleCreate = async () => {
    if (!newSession.name.trim()) {
      toast({ title: 'Error', description: 'Name is required', variant: 'destructive' });
      return;
    }
    try {
      await createSession.mutateAsync({
        name: newSession.name.trim(),
        description: newSession.description.trim() || undefined,
      });
      setShowCreateDialog(false);
      setNewSession({ name: '', description: '' });
      toast({ title: 'Session Created', description: `"${newSession.name}" created` });
    } catch {
      toast({ title: 'Error', description: 'Failed to create session', variant: 'destructive' });
    }
  };

  const handleSwitch = async (session: Session) => {
    try {
      await switchSession.mutateAsync(session.id);
      toast({ title: 'Session Switched', description: `Active session: ${session.name || session.id}` });
    } catch {
      toast({ title: 'Error', description: 'Failed to switch session', variant: 'destructive' });
    }
  };

  const handleExport = async (session: Session) => {
    try {
      await exportSession(session.id, session.name || undefined);
      toast({ title: 'Session Exported', description: `Downloaded "${session.name || session.id}"` });
    } catch {
      toast({ title: 'Error', description: 'Failed to export session', variant: 'destructive' });
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteSession.mutateAsync(id);
      setConfirmDeleteId(null);
      toast({ title: 'Session Deleted' });
    } catch {
      toast({ title: 'Error', description: 'Failed to delete session', variant: 'destructive' });
    }
  };

  const handleImportClick = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const data = JSON.parse(text);
      await importSession.mutateAsync(data);
      toast({ title: 'Session Imported', description: file.name });
    } catch {
      toast({ title: 'Error', description: 'Invalid session file', variant: 'destructive' });
    } finally {
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const formatDate = (iso: string) => {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  };

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading sessions…
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="space-y-3 border-b border-border p-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Sessions</h2>
            <p className="text-2xs text-muted-foreground">
              {sessions?.length || 0} session{sessions?.length === 1 ? '' : 's'}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <input
              ref={fileInputRef}
              type="file"
              accept="application/json,.json"
              className="hidden"
              onChange={handleFileChange}
            />
            <Button variant="outline" size="sm" onClick={handleImportClick} disabled={importSession.isPending}>
              <Upload className="h-4 w-4" />
              Import
            </Button>
            <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
              <DialogTrigger asChild>
                <Button size="sm">
                  <Plus className="h-4 w-4" />
                  Create
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Create Session</DialogTitle>
                  <DialogDescription>
                    Create a new debugging session to group captured traffic
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Name</label>
                    <Input
                      placeholder="My Session"
                      value={newSession.name}
                      onChange={(e) => setNewSession({ ...newSession, name: e.target.value })}
                    />
                  </div>
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Description (optional)</label>
                    <Input
                      placeholder="What this session is for"
                      value={newSession.description}
                      onChange={(e) => setNewSession({ ...newSession, description: e.target.value })}
                    />
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
                    Cancel
                  </Button>
                  <Button onClick={handleCreate} disabled={createSession.isPending}>
                    {createSession.isPending && <Loader2 className="mr-1 h-4 w-4 animate-spin" />}
                    Create
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        </div>
      </div>

      {/* List */}
      <ScrollArea className="flex-1">
        <div className="space-y-2 p-4">
          {(!sessions || sessions.length === 0) && (
            <div className="flex flex-col items-center justify-center py-16 text-center text-muted-foreground">
              <FolderTree className="mb-2 h-8 w-8 opacity-50" />
              <p className="text-xs">No sessions yet. Create or import one to get started.</p>
            </div>
          )}

          {sessions?.map((session) => (
            <div
              key={session.id}
              className="flex items-center justify-between rounded-md border border-border p-3 transition-colors hover:bg-muted/30"
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="truncate text-xs font-medium">
                    {session.name || 'Untitled Session'}
                  </span>
                  {session.description && (
                    <span className="truncate text-2xs text-muted-foreground">
                      {session.description}
                    </span>
                  )}
                </div>
                <div className="mt-1 flex items-center gap-3 text-2xs text-muted-foreground">
                  <span>{formatDate(session.created_at)}</span>
                  <span>
                    {session.traffic_count ?? session.request_count ?? 0} entries
                  </span>
                  <span className="truncate font-mono">{session.id}</span>
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-2xs"
                  onClick={() => handleSwitch(session)}
                  disabled={switchSession.isPending}
                >
                  <ArrowRightLeft className="h-3.5 w-3.5" />
                  Switch
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-2xs"
                  onClick={() => handleExport(session)}
                >
                  <Download className="h-3.5 w-3.5" />
                  Export
                </Button>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="icon-sm">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => handleSwitch(session)}>
                      <ArrowRightLeft className="mr-2 h-4 w-4" />
                      Switch to
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => handleExport(session)}>
                      <Download className="mr-2 h-4 w-4" />
                      Export
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      className="text-destructive"
                      onClick={() => setConfirmDeleteId(session.id)}
                    >
                      <Trash2 className="mr-2 h-4 w-4" />
                      Delete
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          ))}
        </div>
      </ScrollArea>

      {/* Delete confirmation */}
      <Dialog open={confirmDeleteId !== null} onOpenChange={(o) => !o && setConfirmDeleteId(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Session?</DialogTitle>
            <DialogDescription>
              This will permanently delete the session and all its captured traffic. This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmDeleteId(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => confirmDeleteId && handleDelete(confirmDeleteId)}
              disabled={deleteSession.isPending}
            >
              {deleteSession.isPending && <Loader2 className="mr-1 h-4 w-4 animate-spin" />}
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
