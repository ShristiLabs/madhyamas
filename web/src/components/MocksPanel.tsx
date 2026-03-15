import { useState, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
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
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import {
  useMocks,
  useCreateMock,
  useDeleteMock,
  useToggleMock,
  type MockRule,
  type MockResponse,
  type MatchCondition,
} from '@/lib/api/intercept';
import { useToast } from '@/components/ui/use-toast';
import { Search, Plus, MoreVertical, FileDown, Check, X, Layers } from 'lucide-react';

interface MocksPanelProps {
  onEditMock?: (mock: MockRule) => void;
}

// Mock templates for quick creation
const MOCK_TEMPLATES = [
  {
    name: 'JSON Success',
    statusCode: 200,
    contentType: 'application/json',
    body: '{"success": true, "data": {}}',
  },
  {
    name: 'JSON Error',
    statusCode: 500,
    contentType: 'application/json',
    body: '{"success": false, "error": "Internal Server Error"}',
  },
  {
    name: '404 Not Found',
    statusCode: 404,
    contentType: 'application/json',
    body: '{"error": "Not Found"}',
  },
  {
    name: 'CORS Preflight',
    statusCode: 204,
    contentType: 'text/plain',
    body: '',
    headers: {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
      'Access-Control-Allow-Headers': '*',
    },
  },
  {
    name: 'Rate Limited',
    statusCode: 429,
    contentType: 'application/json',
    body: '{"error": "Too Many Requests"}',
  },
];

export function MocksPanel({ onEditMock }: MocksPanelProps) {
  const { data: mocks, isLoading } = useMocks();
  const createMock = useCreateMock();
  const deleteMock = useDeleteMock();
  const toggleMock = useToggleMock();
  const { toast } = useToast();

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedTemplate, setSelectedTemplate] = useState<number | null>(null);
  const [newMock, setNewMock] = useState({
    name: '',
    urlPattern: '',
    statusCode: 200,
    body: '',
    contentType: 'application/json',
  });

  // Filter mocks based on search term
  const filteredMocks = useMemo(() => {
    if (!mocks) return [];
    if (!searchTerm) return mocks;

    const term = searchTerm.toLowerCase();
    return mocks.filter(
      (mock) =>
        mock.name.toLowerCase().includes(term) ||
        mock.condition.pattern?.toLowerCase().includes(term) ||
        mock.response.status_code.toString().includes(term)
    );
  }, [mocks, searchTerm]);

  // Stats
  const stats = useMemo(() => {
    if (!mocks) return { total: 0, enabled: 0, hits: 0 };
    return {
      total: mocks.length,
      enabled: mocks.filter((m) => m.enabled).length,
      hits: mocks.reduce((sum, m) => sum + m.hit_count, 0),
    };
  }, [mocks]);

  const handleCreate = async () => {
    if (!newMock.name || !newMock.urlPattern) {
      toast({
        title: 'Error',
        description: 'Name and URL pattern are required',
        variant: 'destructive',
      });
      return;
    }

    const condition: MatchCondition = {
      type: 'url_pattern',
      pattern: newMock.urlPattern,
    };

    const response: MockResponse = {
      status_code: newMock.statusCode,
      headers: {
        'Content-Type': newMock.contentType,
      },
      body: newMock.body,
    };

    await createMock.mutateAsync({
      name: newMock.name,
      condition,
      response,
      enabled: true,
    });

    setShowCreateDialog(false);
    setNewMock({
      name: '',
      urlPattern: '',
      statusCode: 200,
      body: '',
      contentType: 'application/json',
    });
    setSelectedTemplate(null);

    toast({
      title: 'Mock Created',
      description: `Mock "${newMock.name}" has been created`,
    });
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    await toggleMock.mutateAsync({ id, enabled });
    toast({
      title: enabled ? 'Mock Enabled' : 'Mock Disabled',
      description: `Mock has been ${enabled ? 'enabled' : 'disabled'}`,
    });
  };

  const handleDelete = async (id: string, name: string) => {
    await deleteMock.mutateAsync(id);
    toast({
      title: 'Mock Deleted',
      description: `Mock "${name}" has been deleted`,
    });
  };

  const handleTemplateSelect = (index: number) => {
    const template = MOCK_TEMPLATES[index];
    setSelectedTemplate(index);
    setNewMock({
      ...newMock,
      name: template.name,
      statusCode: template.statusCode,
      contentType: template.contentType,
      body: template.body,
    });
  };

  const handleEnableAll = async () => {
    if (!mocks) return;
    for (const mock of mocks) {
      if (!mock.enabled) {
        await toggleMock.mutateAsync({ id: mock.id, enabled: true });
      }
    }
    toast({ description: 'All mocks enabled' });
  };

  const handleDisableAll = async () => {
    if (!mocks) return;
    for (const mock of mocks) {
      if (mock.enabled) {
        await toggleMock.mutateAsync({ id: mock.id, enabled: false });
      }
    }
    toast({ description: 'All mocks disabled' });
  };

  const handleExport = () => {
    if (!mocks || mocks.length === 0) {
      toast({ title: 'No mocks to export', variant: 'destructive' });
      return;
    }
    const data = JSON.stringify(mocks, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'madhyamas-mocks.json';
    a.click();
    URL.revokeObjectURL(url);
    toast({ description: 'Mocks exported' });
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading mocks...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Response Mocks</h2>
            <p className="text-xs text-muted-foreground">
              {stats.enabled}/{stats.total} enabled • {stats.hits} total hits
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
                  <FileDown className="h-4 w-4 mr-2" />
                  Export
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Create Button */}
            <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
              <DialogTrigger asChild>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-1" />
                  Create Mock
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-lg">
                <DialogHeader>
                  <DialogTitle>Create Mock Response</DialogTitle>
                  <DialogDescription>
                    Create a mock response for matching requests
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  {/* Templates */}
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Quick Templates</label>
                    <div className="flex flex-wrap gap-1">
                      {MOCK_TEMPLATES.map((template, index) => (
                        <Button
                          key={template.name}
                          variant={selectedTemplate === index ? 'default' : 'outline'}
                          size="sm"
                          onClick={() => handleTemplateSelect(index)}
                          className="h-7 text-xs"
                        >
                          {template.name}
                        </Button>
                      ))}
                    </div>
                  </div>

                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Name</label>
                    <Input
                      placeholder="My Mock"
                      value={newMock.name}
                      onChange={(e) => {
                        setNewMock({ ...newMock, name: e.target.value });
                        setSelectedTemplate(null);
                      }}
                    />
                  </div>
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">URL Pattern (Regex)</label>
                    <Input
                      placeholder=".*api/users.*"
                      value={newMock.urlPattern}
                      onChange={(e) => setNewMock({ ...newMock, urlPattern: e.target.value })}
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">Status Code</label>
                      <Input
                        type="number"
                        value={newMock.statusCode}
                        onChange={(e) => setNewMock({ ...newMock, statusCode: parseInt(e.target.value) || 200 })}
                      />
                    </div>
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">Content Type</label>
                      <Select
                        value={newMock.contentType}
                        onValueChange={(v) => setNewMock({ ...newMock, contentType: v })}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="application/json">JSON</SelectItem>
                          <SelectItem value="text/html">HTML</SelectItem>
                          <SelectItem value="text/plain">Plain Text</SelectItem>
                          <SelectItem value="application/xml">XML</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Response Body</label>
                    <textarea
                      className="w-full h-32 p-2 border rounded-md font-mono text-sm bg-background"
                      placeholder='{"message": "Hello, World!"}'
                      value={newMock.body}
                      onChange={(e) => setNewMock({ ...newMock, body: e.target.value })}
                    />
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
                    Cancel
                  </Button>
                  <Button onClick={handleCreate} disabled={createMock.isPending}>
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
            placeholder="Search mocks..."
            className="pl-9"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>
      </div>

      {/* List */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-3">
          {filteredMocks.length === 0 && (
            <div className="text-center text-muted-foreground py-8">
              {searchTerm ? 'No mocks match your search' : 'No mocks configured. Create one to intercept and mock responses.'}
            </div>
          )}

          {filteredMocks.map((mock) => (
            <div
              key={mock.id}
              className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50 transition-colors"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium truncate">{mock.name}</span>
                  <span className="text-xs px-2 py-0.5 rounded bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300 flex-shrink-0">
                    {mock.response.status_code}
                  </span>
                  {mock.hit_count > 0 && (
                    <span className="text-xs text-muted-foreground flex-shrink-0">
                      ({mock.hit_count} hits)
                    </span>
                  )}
                </div>
                <div className="text-sm text-muted-foreground font-mono truncate">
                  {mock.condition.pattern || 'All URLs'}
                </div>
              </div>
              <div className="flex items-center gap-2 flex-shrink-0">
                <Switch
                  checked={mock.enabled}
                  onCheckedChange={(checked) => handleToggle(mock.id, checked)}
                  aria-label={`Toggle ${mock.name}`}
                />
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => onEditMock?.(mock)}>
                      Edit
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      className="text-destructive"
                      onClick={() => handleDelete(mock.id, mock.name)}
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
