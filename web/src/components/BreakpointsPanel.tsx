import { useState, useMemo } from 'react';
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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  useBreakpoints,
  usePausedTraffic,
  useCreateBreakpoint,
  useDeleteBreakpoint,
  useResumePaused,
  type PausedTraffic,
  type MatchCondition,
  type BreakpointDecision,
} from '@/lib/api/intercept';
import { useToast } from '@/components/ui/use-toast';
import { Search, Plus, MoreVertical, Layers, Trash2, AlertTriangle } from 'lucide-react';

export function BreakpointsPanel() {
  const { data: breakpoints, isLoading } = useBreakpoints();
  const { data: pausedTraffic } = usePausedTraffic();
  const createBreakpoint = useCreateBreakpoint();
  const deleteBreakpoint = useDeleteBreakpoint();
  const resumePaused = useResumePaused();
  const { toast } = useToast();

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const [newBreakpoint, setNewBreakpoint] = useState({
    name: '',
    urlPattern: '',
    direction: 'request' as 'request' | 'response' | 'both',
    once: false,
  });

  // Filter breakpoints based on search term
  const filteredBreakpoints = useMemo(() => {
    if (!breakpoints) return [];
    if (!searchTerm) return breakpoints;

    const term = searchTerm.toLowerCase();
    return breakpoints.filter(
      (bp) =>
        bp.name.toLowerCase().includes(term) ||
        bp.condition.pattern?.toLowerCase().includes(term) ||
        bp.direction.toLowerCase().includes(term)
    );
  }, [breakpoints, searchTerm]);

  const handleCreate = async () => {
    if (!newBreakpoint.name) {
      toast({
        title: 'Error',
        description: 'Name is required',
        variant: 'destructive',
      });
      return;
    }

    const condition: MatchCondition = newBreakpoint.urlPattern
      ? { type: 'url_pattern', pattern: newBreakpoint.urlPattern }
      : { type: 'all' };

    await createBreakpoint.mutateAsync({
      name: newBreakpoint.name,
      condition,
      direction: newBreakpoint.direction,
      enabled: true,
    });

    setShowCreateDialog(false);
    setNewBreakpoint({ name: '', urlPattern: '', direction: 'request', once: false });

    toast({
      title: 'Breakpoint Created',
      description: `Breakpoint "${newBreakpoint.name}" has been created`,
    });
  };

  const handleDelete = async (id: string, name: string) => {
    await deleteBreakpoint.mutateAsync(id);
    toast({
      title: 'Breakpoint Deleted',
      description: `Breakpoint "${name}" has been deleted`,
    });
  };

  const handleDeleteAll = async () => {
    if (!breakpoints || breakpoints.length === 0) return;
    for (const bp of breakpoints) {
      await deleteBreakpoint.mutateAsync(bp.id);
    }
    toast({ description: 'All breakpoints deleted' });
  };

  const handleResume = async (id: string, action: BreakpointDecision) => {
    await resumePaused.mutateAsync({ id, action });
    toast({
      title: 'Traffic Resumed',
      description: 'The paused traffic has been processed',
    });
  };

  const handleResumeAll = async (action: 'continue' | 'abort') => {
    if (!pausedTraffic || pausedTraffic.length === 0) return;
    for (const paused of pausedTraffic) {
      await resumePaused.mutateAsync({ id: paused.id, action: { action } });
    }
    toast({ description: `All paused requests ${action === 'continue' ? 'continued' : 'aborted'}` });
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading breakpoints...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Paused Traffic Alert */}
      {pausedTraffic && pausedTraffic.length > 0 && (
        <div className="p-4 bg-yellow-50 dark:bg-yellow-900/20 border-b border-yellow-200 dark:border-yellow-800">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
              <span className="text-yellow-600 dark:text-yellow-400 font-medium">
                {pausedTraffic.length} Paused Request{pausedTraffic.length > 1 ? 's' : ''}
              </span>
            </div>
            <div className="flex gap-1">
              <Button
                variant="ghost"
                size="sm"
                className="h-7 text-xs text-green-600"
                onClick={() => handleResumeAll('continue')}
              >
                Continue All
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 text-xs text-red-600"
                onClick={() => handleResumeAll('abort')}
              >
                Abort All
              </Button>
            </div>
          </div>
          <div className="space-y-2">
            {pausedTraffic.map((paused) => (
              <PausedTrafficItem
                key={paused.id}
                paused={paused}
                onResume={handleResume}
              />
            ))}
          </div>
        </div>
      )}

      {/* Header */}
      <div className="p-4 border-b space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Breakpoints</h2>
            <p className="text-xs text-muted-foreground">
              {breakpoints?.length || 0} configured • {pausedTraffic?.length || 0} paused
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
                <DropdownMenuItem
                  className="text-destructive"
                  onClick={handleDeleteAll}
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Delete All
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Create Button */}
            <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
              <DialogTrigger asChild>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-1" />
                  Create
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Create Breakpoint</DialogTitle>
                  <DialogDescription>
                    Create a breakpoint to pause traffic matching a pattern
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Name</label>
                    <Input
                      placeholder="My Breakpoint"
                      value={newBreakpoint.name}
                      onChange={(e) => setNewBreakpoint({ ...newBreakpoint, name: e.target.value })}
                    />
                  </div>
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">URL Pattern (Regex, optional)</label>
                    <Input
                      placeholder=".*api/users.*"
                      value={newBreakpoint.urlPattern}
                      onChange={(e) => setNewBreakpoint({ ...newBreakpoint, urlPattern: e.target.value })}
                    />
                  </div>
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Direction</label>
                    <Select
                      value={newBreakpoint.direction}
                      onValueChange={(v: 'request' | 'response' | 'both') => setNewBreakpoint({ ...newBreakpoint, direction: v })}
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="request">Request</SelectItem>
                        <SelectItem value="response">Response</SelectItem>
                        <SelectItem value="both">Both</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
                    Cancel
                  </Button>
                  <Button onClick={handleCreate} disabled={createBreakpoint.isPending}>
                    Create
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
            placeholder="Search breakpoints..."
            className="pl-9"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>
      </div>

      {/* List */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-3">
          {filteredBreakpoints.length === 0 && (
            <div className="text-center text-muted-foreground py-8">
              {searchTerm ? 'No breakpoints match your search' : 'No breakpoints configured. Create one to pause and inspect traffic.'}
            </div>
          )}

          {filteredBreakpoints.map((bp) => (
            <div
              key={bp.id}
              className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50 transition-colors"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium truncate">{bp.name}</span>
                  <span className="text-xs px-2 py-0.5 rounded bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-300 flex-shrink-0">
                    {bp.direction}
                  </span>
                </div>
                <div className="text-sm text-muted-foreground font-mono truncate">
                  {bp.condition.pattern || 'All URLs'}
                </div>
              </div>
              <div className="flex items-center gap-2 flex-shrink-0">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      className="text-destructive"
                      onClick={() => handleDelete(bp.id, bp.name)}
                    >
                      Delete
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}

function PausedTrafficItem({
  paused,
  onResume,
}: {
  paused: PausedTraffic;
  onResume: (id: string, action: BreakpointDecision) => void;
}) {
  const [showDetails, setShowDetails] = useState(false);

  return (
    <div className="p-2 bg-white dark:bg-gray-800 rounded border text-sm">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-mono text-xs px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-700 flex-shrink-0">
            {paused.request.method}
          </span>
          <span className="font-mono truncate">{paused.request.url}</span>
        </div>
        <div className="flex items-center gap-1 flex-shrink-0">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 text-xs"
            onClick={() => setShowDetails(!showDetails)}
          >
            {showDetails ? 'Hide' : 'Show'}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 text-xs text-green-600"
            onClick={() => onResume(paused.id, { action: 'continue' })}
          >
            Continue
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 text-xs text-red-600"
            onClick={() => onResume(paused.id, { action: 'abort' })}
          >
            Abort
          </Button>
        </div>
      </div>
      {showDetails && (
        <div className="mt-2 p-2 bg-gray-50 dark:bg-gray-900 rounded text-xs font-mono overflow-auto max-h-40">
          <div>
            <strong>Headers:</strong>
            <pre>{JSON.stringify(paused.request.headers, null, 2)}</pre>
          </div>
          {paused.request.body && (
            <div className="mt-2">
              <strong>Body:</strong>
              <pre>{paused.request.body}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
