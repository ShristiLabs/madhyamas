import { useState, useMemo, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import {
  useMocks,
  useCreateMock,
  useDeleteMock,
  useToggleMock,
  useDuplicateMock,
  useMockRecordingStatus,
  useSetMockRecording,
  useRecordedMocks,
  usePromoteRecordedMocks,
  useClearRecordedMocks,
  useImportMocks,
  useMockCollections,
  useCreateMockCollection,
  useDeleteMockCollection,
  useToggleMockCollection,
  type MockRule,
  type MockResponse,
  type MatchCondition,
} from "@/lib/api/intercept";
import { MockEditDialog } from "@/components/MockEditDialog";
import { useToast } from "@/components/ui/use-toast";

import {
  Search,
  Plus,
  MoreVertical,
  FileDown,
  FileUp,
  Check,
  X,
  Layers,
  Circle,
  Copy,
  Zap,
  List,
  Shuffle,
  GitBranch,
  Folder,
  Trash2,
  Edit,
} from "lucide-react";

interface MocksPanelProps {
  onEditMock?: (mock: MockRule) => void;
}

// Mock templates for quick creation
const MOCK_TEMPLATES = [
  {
    name: "JSON Success",
    statusCode: 200,
    contentType: "application/json",
    body: '{"success": true, "data": {}}',
  },
  {
    name: "JSON Error",
    statusCode: 500,
    contentType: "application/json",
    body: '{"success": false, "error": "Internal Server Error"}',
  },
  {
    name: "404 Not Found",
    statusCode: 404,
    contentType: "application/json",
    body: '{"error": "Not Found"}',
  },
  {
    name: "CORS Preflight",
    statusCode: 204,
    contentType: "text/plain",
    body: "",
    headers: {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
      "Access-Control-Allow-Headers": "*",
    },
  },
  {
    name: "Rate Limited",
    statusCode: 429,
    contentType: "application/json",
    body: '{"error": "Too Many Requests"}',
  },
];

// Helper to get display pattern from a condition
function getConditionPattern(condition: MatchCondition): string {
  return condition.pattern || "";
}

export function MocksPanel({ onEditMock }: MocksPanelProps) {
  const { data: mocks, isLoading } = useMocks();
  const createMock = useCreateMock();
  const deleteMock = useDeleteMock();
  const toggleMock = useToggleMock();
  const duplicateMock = useDuplicateMock();
  const { toast } = useToast();

  // Recording hooks
  const { data: recordingStatus } = useMockRecordingStatus();
  const setRecording = useSetMockRecording();
  const { data: recordedMocks } = useRecordedMocks();
  const promoteRecorded = usePromoteRecordedMocks();
  const clearRecorded = useClearRecordedMocks();

  // Collections hooks
  const { data: collections } = useMockCollections();
  const createCollection = useCreateMockCollection();
  const deleteCollection = useDeleteMockCollection();
  const toggleCollection = useToggleMockCollection();

  // Import hook
  const importMocks = useImportMocks();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showAdvancedCreate, setShowAdvancedCreate] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedCollection, setSelectedCollection] = useState<string | null>(
    null,
  );
  const [selectedTemplate, setSelectedTemplate] = useState<number | null>(null);
  const [editingMock, setEditingMock] = useState<MockRule | null>(null);
  const [newMock, setNewMock] = useState({
    name: "",
    urlPattern: "",
    statusCode: 200,
    body: "",
    contentType: "application/json",
    delayMs: 0,
    delayVariance: 0,
  });

  // Helper to get response from response_config
  const getResponse = (mock: MockRule): MockResponse => {
    const config = mock.response_config;
    if (config.type === "single" && config.response) {
      return config.response;
    }
    if (config.type === "sequence" && config.responses?.length) {
      return config.responses[0];
    }
    if (config.type === "conditional" && config.default_response) {
      return config.default_response;
    }
    if (
      config.type === "probabilistic" &&
      config.probabilistic_responses?.length
    ) {
      return config.probabilistic_responses[0].response;
    }
    return { status_code: 200 };
  };

  // Filter mocks based on search term
  const filteredMocks = useMemo(() => {
    if (!mocks) return [];
    if (!searchTerm) return mocks;

    const term = searchTerm.toLowerCase();
    return mocks.filter(
      (mock) =>
        mock.name.toLowerCase().includes(term) ||
        getConditionPattern(mock.condition).toLowerCase().includes(term) ||
        getResponse(mock).status_code.toString().includes(term),
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
        title: "Error",
        description: "Name and URL pattern are required",
        variant: "destructive",
      });
      return;
    }

    const condition: MatchCondition = {
      type: "url_pattern",
      pattern: newMock.urlPattern,
    };

    const response: MockResponse = {
      status_code: newMock.statusCode,
      headers: {
        "Content-Type": newMock.contentType,
      },
      body: newMock.body,
    };

    const response_config = {
      type: "single" as const,
      response,
    };

    await createMock.mutateAsync({
      name: newMock.name,
      condition,
      response_config,
      enabled: true,
      priority: 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      version: 1,
      version_history: [],
    });

    setShowCreateDialog(false);
    setNewMock({
      name: "",
      urlPattern: "",
      statusCode: 200,
      body: "",
      contentType: "application/json",
      delayMs: 0,
      delayVariance: 0,
    });
    setSelectedTemplate(null);

    toast({
      title: "Mock Created",
      description: `Mock "${newMock.name}" has been created`,
    });
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    await toggleMock.mutateAsync({ id, enabled });
    toast({
      title: enabled ? "Mock Enabled" : "Mock Disabled",
      description: `Mock has been ${enabled ? "enabled" : "disabled"}`,
    });
  };

  const handleDelete = async (id: string, name: string) => {
    await deleteMock.mutateAsync(id);
    toast({
      title: "Mock Deleted",
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
      delayMs: 0,
      delayVariance: 0,
    });
  };

  const handleEnableAll = async () => {
    if (!mocks) return;
    for (const mock of mocks) {
      if (!mock.enabled) {
        await toggleMock.mutateAsync({ id: mock.id, enabled: true });
      }
    }
    toast({ description: "All mocks enabled" });
  };

  const handleDisableAll = async () => {
    if (!mocks) return;
    for (const mock of mocks) {
      if (mock.enabled) {
        await toggleMock.mutateAsync({ id: mock.id, enabled: false });
      }
    }
    toast({ description: "All mocks disabled" });
  };

  const handleExport = () => {
    if (!mocks || mocks.length === 0) {
      toast({ title: "No mocks to export", variant: "destructive" });
      return;
    }
    const data = JSON.stringify(mocks, null, 2);
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "madhyamas-mocks.json";
    a.click();
    URL.revokeObjectURL(url);
    toast({ description: "Mocks exported" });
  };

  const handleDuplicate = async (id: string, name: string) => {
    try {
      await duplicateMock.mutateAsync({ id, newName: `${name} (Copy)` });
      toast({ description: "Mock duplicated" });
    } catch {
      toast({ title: "Failed to duplicate mock", variant: "destructive" });
    }
  };

  const handleToggleRecording = async () => {
    const newState = !recordingStatus?.recording;
    await setRecording.mutateAsync(newState);
    toast({
      description: newState
        ? "Recording started - traffic will be captured as mocks"
        : "Recording stopped",
    });
  };

  const handlePromoteRecorded = async () => {
    const result = await promoteRecorded.mutateAsync();
    toast({
      description: `Promoted ${result.promoted} recorded mocks to active rules`,
    });
  };

  const handleClearRecorded = async () => {
    await clearRecorded.mutateAsync();
    toast({ description: "Cleared recorded mocks" });
  };

  const handleImportFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const text = await file.text();
    let format: "har" | "openapi" | "postman" = "har";

    // Detect format from file content
    try {
      const json = JSON.parse(text);
      if (json.log?.entries) format = "har";
      else if (json.openapi || json.swagger) format = "openapi";
      else if (json.info?.schema?.includes("postman")) format = "postman";
    } catch {
      toast({ title: "Invalid JSON file", variant: "destructive" });
      return;
    }

    try {
      const result = await importMocks.mutateAsync({ format, data: text });
      toast({
        description: `Imported ${result.imported} mocks from ${format.toUpperCase()}`,
      });
    } catch {
      toast({ title: "Failed to import mocks", variant: "destructive" });
    }

    // Reset file input
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  // Get response type badge
  const getResponseTypeBadge = (mock: MockRule) => {
    const type = mock.response_config.type;
    switch (type) {
      case "sequence":
        return (
          <span className="text-xs px-1.5 py-0.5 rounded bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300">
            <List className="h-3 w-3 inline mr-0.5" />
            Seq
          </span>
        );
      case "conditional":
        return (
          <span className="text-xs px-1.5 py-0.5 rounded bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-300">
            <GitBranch className="h-3 w-3 inline mr-0.5" />
            Cond
          </span>
        );
      case "probabilistic":
        return (
          <span className="text-xs px-1.5 py-0.5 rounded bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300">
            <Shuffle className="h-3 w-3 inline mr-0.5" />
            Prob
          </span>
        );
      default:
        return null;
    }
  };

  // Suppress unused variable warnings - these are used for future collection management UI
  void createCollection;
  void deleteCollection;
  void toggleCollection;

  // Filter mocks by collection - must be before early return
  const filteredByCollection = useMemo(() => {
    if (!filteredMocks) return [];
    if (!selectedCollection) return filteredMocks;
    return filteredMocks.filter((m) => m.collection_id === selectedCollection);
  }, [filteredMocks, selectedCollection]);

  // Handle edit mock
  const handleEditMock = (mock: MockRule) => {
    setEditingMock(mock);
    onEditMock?.(mock);
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
      {/* Mock Edit Dialog - for editing existing mocks */}
      <MockEditDialog
        mock={editingMock}
        open={!!editingMock}
        onOpenChange={(open) => !open && setEditingMock(null)}
        onSave={() => setEditingMock(null)}
      />

      {/* Mock Edit Dialog - for creating new advanced mocks */}
      <MockEditDialog
        mock={null}
        open={showAdvancedCreate}
        onOpenChange={setShowAdvancedCreate}
        onSave={() => setShowAdvancedCreate(false)}
      />

      {/* Hidden file input for import */}
      <input
        type="file"
        ref={fileInputRef}
        className="hidden"
        accept=".json,.har"
        onChange={handleImportFile}
      />

      {/* Header */}
      <div className="p-4 border-b space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Response Mocks</h2>
            <p className="text-xs text-muted-foreground">
              {stats.enabled}/{stats.total} enabled • {stats.hits} total hits
              {recordedMocks && recordedMocks.length > 0 && (
                <span className="ml-2 text-orange-500">
                  • {recordedMocks.length} recorded
                </span>
              )}
              {collections && collections.length > 0 && (
                <span className="ml-2">• {collections.length} collections</span>
              )}
            </p>
          </div>
          <div className="flex items-center gap-2">
            {/* Recording Toggle */}
            <Button
              variant={recordingStatus?.recording ? "destructive" : "outline"}
              size="sm"
              onClick={handleToggleRecording}
              disabled={setRecording.isPending}
            >
              <Circle
                className={`h-3 w-3 mr-1 ${recordingStatus?.recording ? "fill-current animate-pulse" : ""}`}
              />
              {recordingStatus?.recording ? "Recording..." : "Record"}
            </Button>

            {/* Recorded Mocks Actions */}
            {recordedMocks && recordedMocks.length > 0 && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="sm">
                    <Zap className="h-4 w-4 mr-1" />
                    Recorded ({recordedMocks.length})
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={handlePromoteRecorded}>
                    <Check className="h-4 w-4 mr-2" />
                    Promote to Active
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={handleClearRecorded}
                    className="text-destructive"
                  >
                    <X className="h-4 w-4 mr-2" />
                    Clear Recorded
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            )}

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
                  Export JSON
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => fileInputRef.current?.click()}>
                  <FileUp className="h-4 w-4 mr-2" />
                  Import (HAR/OpenAPI/Postman)
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Create Button */}
            <Button
              size="sm"
              variant="outline"
              onClick={() => setShowAdvancedCreate(true)}
            >
              <Plus className="h-4 w-4 mr-1" />
              Create New Mock
            </Button>
            <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
              <DialogTrigger asChild>
                <Button size="sm">
                  <Plus className="h-4 w-4 mr-1" />
                  Quick Create
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
                    <label className="text-sm font-medium">
                      Quick Templates
                    </label>
                    <div className="flex flex-wrap gap-1">
                      {MOCK_TEMPLATES.map((template, index) => (
                        <Button
                          key={template.name}
                          variant={
                            selectedTemplate === index ? "default" : "outline"
                          }
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
                    <label className="text-sm font-medium">
                      URL Pattern (Regex)
                    </label>
                    <Input
                      placeholder=".*api/users.*"
                      value={newMock.urlPattern}
                      onChange={(e) =>
                        setNewMock({ ...newMock, urlPattern: e.target.value })
                      }
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">Status Code</label>
                      <Input
                        type="number"
                        value={newMock.statusCode}
                        onChange={(e) =>
                          setNewMock({
                            ...newMock,
                            statusCode: parseInt(e.target.value) || 200,
                          })
                        }
                      />
                    </div>
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">
                        Content Type
                      </label>
                      <Select
                        value={newMock.contentType}
                        onValueChange={(v) =>
                          setNewMock({ ...newMock, contentType: v })
                        }
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
                      onChange={(e) =>
                        setNewMock({ ...newMock, body: e.target.value })
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
                    disabled={createMock.isPending}
                  >
                    Create
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        </div>

        {/* Search and Filter */}
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search mocks..."
              className="pl-9"
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
            />
          </div>
          {collections && collections.length > 0 && (
            <Select
              value={selectedCollection || "all"}
              onValueChange={(v) =>
                setSelectedCollection(v === "all" ? null : v)
              }
            >
              <SelectTrigger className="w-[180px]">
                <Folder className="h-4 w-4 mr-2" />
                <SelectValue placeholder="All Collections" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Collections</SelectItem>
                {collections.map((c) => (
                  <SelectItem key={c.id} value={c.id}>
                    {c.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </div>
      </div>

      {/* List */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-3">
          {/* Recorded Mocks Section */}
          {recordedMocks && recordedMocks.length > 0 && (
            <div className="mb-4">
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-semibold text-orange-600 dark:text-orange-400">
                  <Circle className="h-3 w-3 inline mr-1 fill-current animate-pulse" />
                  Recorded Mocks ({recordedMocks.length})
                </h3>
                <div className="flex gap-1">
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={handlePromoteRecorded}
                  >
                    <Check className="h-3 w-3 mr-1" />
                    Promote All
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={handleClearRecorded}
                  >
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              </div>
              <div className="space-y-2">
                {recordedMocks.map((mock) => (
                  <div
                    key={mock.id}
                    className="flex items-center justify-between p-2 border border-orange-200 dark:border-orange-800 rounded-lg bg-orange-50 dark:bg-orange-950/30"
                  >
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium truncate">
                          {mock.name}
                        </span>
                        <span className="text-xs px-1.5 py-0.5 rounded bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-300">
                          {getResponse(mock).status_code}
                        </span>
                      </div>
                      <div className="text-xs text-muted-foreground font-mono truncate">
                        {getConditionPattern(mock.condition) || "All URLs"}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {filteredByCollection.length === 0 &&
            (!recordedMocks || recordedMocks.length === 0) && (
              <div className="text-center text-muted-foreground py-8">
                {searchTerm || selectedCollection
                  ? "No mocks match your filters"
                  : "No mocks configured. Create one to intercept and mock responses."}
              </div>
            )}

          {filteredByCollection.map((mock) => (
            <div
              key={mock.id}
              className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50 transition-colors"
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="font-medium truncate">{mock.name}</span>
                  <span className="text-xs px-2 py-0.5 rounded bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300 flex-shrink-0">
                    {getResponse(mock).status_code}
                  </span>
                  {getResponseTypeBadge(mock)}
                  {mock.hit_count > 0 && (
                    <span className="text-xs text-muted-foreground flex-shrink-0">
                      ({mock.hit_count} hits)
                    </span>
                  )}
                  {mock.tags && mock.tags.length > 0 && (
                    <span className="text-xs text-muted-foreground">
                      {mock.tags.slice(0, 2).join(", ")}
                    </span>
                  )}
                </div>
                <div className="text-sm text-muted-foreground font-mono truncate">
                  {getConditionPattern(mock.condition) || "All URLs"}
                </div>
                {mock.description && (
                  <div className="text-xs text-muted-foreground truncate mt-0.5">
                    {mock.description}
                  </div>
                )}
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
                    <DropdownMenuItem onClick={() => handleEditMock(mock)}>
                      <Edit className="h-4 w-4 mr-2" />
                      Edit
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={() => handleDuplicate(mock.id, mock.name)}
                    >
                      <Copy className="h-4 w-4 mr-2" />
                      Duplicate
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      className="text-destructive"
                      onClick={() => handleDelete(mock.id, mock.name)}
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
    </div>
  );
}
