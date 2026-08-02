import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  useSavedRequests,
  useSaveRequest,
  useDeleteSavedRequest,
  useReplayRequest,
  useReplayRequestBatch,
  useReplayHistory,
  type SavedRequest,
  type ReplayResult,
  type ReplayBatchConfig,
  type ReplayBatchResult,
  type RequestModifications,
} from '@/lib/api/intercept';
import { useToast } from '@/components/ui/use-toast';
import type { TrafficEntry } from '@/lib/api';
import { RequestEditor } from '@/features/traffic/RequestEditor';
import { Pencil } from 'lucide-react';

interface ReplayPanelProps {
  selectedEntry?: TrafficEntry | null;
}

export function ReplayPanel({ selectedEntry }: ReplayPanelProps) {
  const { data: savedRequests, isLoading } = useSavedRequests();
  const { data: replayHistory } = useReplayHistory();
  const saveRequest = useSaveRequest();
  const deleteSavedRequest = useDeleteSavedRequest();
  const replayRequest = useReplayRequest();
  const replayRequestBatch = useReplayRequestBatch();
  const { toast } = useToast();

  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [showReplayDialog, setShowReplayDialog] = useState(false);
  const [showEditor, setShowEditor] = useState(false);
  const [selectedSaved, setSelectedSaved] = useState<SavedRequest | null>(null);
  const [editingSaved, setEditingSaved] = useState<SavedRequest | null>(null);
  const [saveName, setSaveName] = useState('');
  const [replayResult, setReplayResult] = useState<ReplayResult | null>(null);

  // Advanced (batch) replay state
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [advancedSaved, setAdvancedSaved] = useState<SavedRequest | null>(null);
  const [iterations, setIterations] = useState(3);
  const [concurrency, setConcurrency] = useState(1);
  const [delayMs, setDelayMs] = useState('');
  const [useDelay, setUseDelay] = useState(false);
  const [batchResult, setBatchResult] = useState<ReplayBatchResult | null>(null);

  const handleSave = async () => {
    if (!selectedEntry || !saveName) return;

    await saveRequest.mutateAsync({
      entry_id: selectedEntry.id,
      request: selectedEntry.request,
      name: saveName,
    });

    setShowSaveDialog(false);
    setSaveName('');
    toast({
      title: 'Request Saved',
      description: `Saved as "${saveName}"`,
    });
  };

  const handleReplay = async (saved: SavedRequest) => {
    setSelectedSaved(saved);
    setShowReplayDialog(true);

    const result = await replayRequest.mutateAsync({ id: saved.id });
    setReplayResult(result);
  };

  const handleEditReplay = (saved: SavedRequest) => {
    setEditingSaved(saved);
    setShowEditor(true);
  };

  const handleEditorSubmit = async (modifications: RequestModifications) => {
    if (!editingSaved) return;
    setShowEditor(false);
    setSelectedSaved(editingSaved);
    setShowReplayDialog(true);

    const result = await replayRequest.mutateAsync({
      id: editingSaved.id,
      modifications,
    });
    setReplayResult(result);
    setEditingSaved(null);
  };

  const handleDelete = async (id: string, name: string) => {
    await deleteSavedRequest.mutateAsync(id);
    toast({
      title: 'Request Deleted',
      description: `"${name}" has been removed`,
    });
  };

  const handleAdvancedReplay = async (saved: SavedRequest) => {
    setAdvancedSaved(saved);
    setShowAdvanced(true);
    setBatchResult(null);
  };

  const handleRunBatch = async () => {
    if (!advancedSaved) return;
    const config: ReplayBatchConfig = {
      iterations,
      concurrency,
      delay_ms: useDelay && delayMs ? Number(delayMs) : undefined,
    };
    try {
      const result = await replayRequestBatch.mutateAsync({
        id: advancedSaved.id,
        config,
      });
      setBatchResult(result);
      toast({
        title: 'Batch Replay Complete',
        description: `${result.succeeded}/${result.total} succeeded`,
      });
    } catch (e) {
      toast({
        title: 'Batch Replay Failed',
        description: String(e),
        variant: 'destructive',
      });
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading saved requests...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-4 border-b">
        <h2 className="text-lg font-semibold">Request Replay</h2>
        {selectedEntry && (
          <Button size="sm" onClick={() => setShowSaveDialog(true)}>
            Save Current
          </Button>
        )}
      </div>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-6">
          {/* Saved Requests */}
          <div>
            <h3 className="text-sm font-medium mb-3">Saved Requests</h3>
            <div className="space-y-2">
              {savedRequests?.length === 0 && (
                <div className="text-center text-muted-foreground py-4 text-sm">
                  No saved requests. Click "Save Current" to save a request from traffic.
                </div>
              )}

              {savedRequests?.map((saved) => (
                <div
                  key={saved.id}
                  className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50"
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-700">
                        {saved.request.method}
                      </span>
                      <span className="font-medium truncate">{saved.name || 'Unnamed'}</span>
                    </div>
                    <div className="text-sm text-muted-foreground font-mono truncate">
                      {saved.request.url}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleReplay(saved)}
                      disabled={replayRequest.isPending}
                    >
                      Replay
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleEditReplay(saved)}
                      disabled={replayRequest.isPending}
                    >
                      <Pencil className="h-3.5 w-3.5" />
                      Edit & Replay
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleAdvancedReplay(saved)}
                      disabled={replayRequestBatch.isPending}
                    >
                      Advanced
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-destructive"
                      onClick={() => handleDelete(saved.id, saved.name || 'Unnamed')}
                    >
                      Delete
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Replay History */}
          {replayHistory && replayHistory.length > 0 && (
            <div>
              <h3 className="text-sm font-medium mb-3">Recent Replays</h3>
              <div className="space-y-2">
                {replayHistory.slice(0, 10).map((result) => (
                  <div
                    key={result.id}
                    className="flex items-center justify-between p-2 border rounded text-sm"
                  >
                    <div className="flex items-center gap-2">
                      {result.error ? (
                        <span className="text-red-500">Error</span>
                      ) : (
                        <span className="font-mono text-xs px-1 py-0.5 rounded bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300">
                          {result.response?.status_code}
                        </span>
                      )}
                      <span className="text-muted-foreground">
                        {result.duration_ms}ms
                      </span>
                    </div>
                    <span className="text-xs text-muted-foreground">
                      {new Date(result.executed_at).toLocaleTimeString()}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Save Dialog */}
      <Dialog open={showSaveDialog} onOpenChange={setShowSaveDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Save Request</DialogTitle>
            <DialogDescription>
              Save this request for later replay
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <label className="text-sm font-medium">Name</label>
              <Input
                placeholder="My Request"
                value={saveName}
                onChange={(e) => setSaveName(e.target.value)}
              />
            </div>
            {selectedEntry && (
              <div className="p-2 bg-muted rounded text-sm font-mono">
                <div className="flex items-center gap-2">
                  <span>{selectedEntry.request.method}</span>
                  <span className="truncate">{selectedEntry.request.url}</span>
                </div>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowSaveDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleSave} disabled={!saveName || saveRequest.isPending}>
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Replay Dialog */}
      <Dialog open={showReplayDialog} onOpenChange={setShowReplayDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Replay Result</DialogTitle>
            <DialogDescription>
              {selectedSaved?.name || 'Unnamed request'}
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            {replayRequest.isPending ? (
              <div className="text-center py-8">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
                <p className="mt-2 text-muted-foreground">Executing request...</p>
              </div>
            ) : replayResult ? (
              <div className="space-y-4">
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">Status:</span>
                    {replayResult.error ? (
                      <span className="text-red-500">Error</span>
                    ) : (
                      <span className={`px-2 py-1 rounded text-sm ${
                        replayResult.response?.status_code && replayResult.response.status_code < 400
                          ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300'
                          : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300'
                      }`}>
                        {replayResult.response?.status_code || 'N/A'}
                      </span>
                    )}
                  </div>
                  <div>
                    <span className="text-sm font-medium">Duration:</span>
                    <span className="ml-2">{replayResult.duration_ms}ms</span>
                  </div>
                </div>

                {replayResult.error && (
                  <div className="p-2 bg-red-50 dark:bg-red-900/20 rounded text-red-600 dark:text-red-400 text-sm">
                    {replayResult.error}
                  </div>
                )}

                {replayResult.response && (
                  <>
                    <div>
                      <h4 className="text-sm font-medium mb-2">Response Headers</h4>
                      <div className="p-2 bg-muted rounded text-xs font-mono max-h-32 overflow-auto">
                        {Object.entries(replayResult.response.headers).map(([key, value]) => (
                          <div key={key}>
                            <span className="text-blue-600 dark:text-blue-400">{key}:</span> {value}
                          </div>
                        ))}
                      </div>
                    </div>

                    {replayResult.response.body && (
                      <div>
                        <h4 className="text-sm font-medium mb-2">Response Body</h4>
                        <pre className="p-2 bg-muted rounded text-xs font-mono max-h-48 overflow-auto whitespace-pre-wrap">
                          {replayResult.response.body}
                        </pre>
                      </div>
                    )}
                  </>
                )}
              </div>
            ) : null}
          </div>
          <DialogFooter>
            <Button onClick={() => setShowReplayDialog(false)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit & Replay Dialog */}
      {editingSaved && (
        <RequestEditor
          open={showEditor}
          onOpenChange={(open) => {
            setShowEditor(open);
            if (!open) setEditingSaved(null);
          }}
          initialRequest={editingSaved.request}
          onSubmit={handleEditorSubmit}
        />
      )}

      {/* Advanced (Batch) Replay Dialog */}
      <Dialog open={showAdvanced} onOpenChange={setShowAdvanced}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Advanced Replay (Repeat)</DialogTitle>
            <DialogDescription>
              {advancedSaved?.name || 'Unnamed request'} &mdash; batch replay
              with concurrency, iterations, and delay.
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-5">
            {/* Iterations */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="iterations">Iterations</Label>
                <span className="text-sm font-mono text-muted-foreground">
                  {iterations}
                </span>
              </div>
              <Slider
                id="iterations"
                min={1}
                max={1000}
                step={1}
                value={[iterations]}
                onValueChange={(v) => setIterations(v[0])}
              />
              <p className="text-xs text-muted-foreground">
                Total number of requests to send (max 10,000).
              </p>
            </div>

            {/* Concurrency */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="concurrency">Concurrency</Label>
                <span className="text-sm font-mono text-muted-foreground">
                  {concurrency}
                </span>
              </div>
              <Slider
                id="concurrency"
                min={1}
                max={100}
                step={1}
                value={[concurrency]}
                onValueChange={(v) => setConcurrency(v[0])}
              />
              <p className="text-xs text-muted-foreground">
                Number of simultaneous in-flight requests (max 100).
              </p>
            </div>

            {/* Delay */}
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <Switch
                  checked={useDelay}
                  onCheckedChange={setUseDelay}
                />
                <Label>Delay between requests</Label>
              </div>
              {useDelay && (
                <div className="flex items-center gap-2">
                  <Input
                    type="number"
                    min={0}
                    placeholder="Delay (ms)"
                    value={delayMs}
                    onChange={(e) => setDelayMs(e.target.value)}
                    className="w-40"
                  />
                  <span className="text-sm text-muted-foreground">ms</span>
                </div>
              )}
            </div>

            {/* Results summary */}
            {replayRequestBatch.isPending ? (
              <div className="text-center py-6">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
                <p className="mt-2 text-muted-foreground">
                  Running batch replay...
                </p>
              </div>
            ) : batchResult ? (
              <div className="space-y-3">
                <div className="grid grid-cols-3 gap-3">
                  <div className="p-3 border rounded-lg text-center">
                    <div className="text-2xl font-bold">{batchResult.total}</div>
                    <div className="text-xs text-muted-foreground">Total</div>
                  </div>
                  <div className="p-3 border rounded-lg text-center">
                    <div className="text-2xl font-bold text-green-600 dark:text-green-400">
                      {batchResult.succeeded}
                    </div>
                    <div className="text-xs text-muted-foreground">Succeeded</div>
                  </div>
                  <div className="p-3 border rounded-lg text-center">
                    <div className="text-2xl font-bold text-red-600 dark:text-red-400">
                      {batchResult.failed}
                    </div>
                    <div className="text-xs text-muted-foreground">Failed</div>
                  </div>
                </div>
                <div className="border rounded-lg">
                  <div className="px-3 py-2 border-b text-sm font-medium">
                    Latency Statistics (ms)
                  </div>
                  <div className="grid grid-cols-4 divide-x">
                    <div className="p-3 text-center">
                      <div className="text-lg font-mono">{batchResult.min_ms}</div>
                      <div className="text-xs text-muted-foreground">min</div>
                    </div>
                    <div className="p-3 text-center">
                      <div className="text-lg font-mono">{batchResult.avg_ms}</div>
                      <div className="text-xs text-muted-foreground">avg</div>
                    </div>
                    <div className="p-3 text-center">
                      <div className="text-lg font-mono">{batchResult.max_ms}</div>
                      <div className="text-xs text-muted-foreground">max</div>
                    </div>
                    <div className="p-3 text-center">
                      <div className="text-lg font-mono">{batchResult.p95_ms}</div>
                      <div className="text-xs text-muted-foreground">p95</div>
                    </div>
                  </div>
                </div>
              </div>
            ) : null}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAdvanced(false)}>
              Close
            </Button>
            <Button
              onClick={handleRunBatch}
              disabled={replayRequestBatch.isPending || iterations < 1}
            >
              Run Batch
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
