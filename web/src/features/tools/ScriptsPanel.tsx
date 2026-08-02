import { useState } from 'react';
import Editor from 'react-simple-code-editor';
import Prism from 'prismjs';
import 'prismjs/components/prism-javascript';
import 'prismjs/themes/prism-tomorrow.css';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import {
  useScripts,
  useScriptTemplates,
  useCreateScript,
  useUpdateScript,
  useDeleteScript,
  useToggleScript,
  useTestScript,
  useValidateScript,
  useScriptHistory,
  useAllScriptHistory,
  useReorderScript,
  useMatchPreview,
} from '@/lib/api/tools';
import type {
  Script,
  ScriptMatch,
  ScriptTemplate,
  ScriptTestResult,
  ScriptErrorPolicy,
  MatchPreviewItem,
  ScriptHistoryEntry,
} from '@/lib/api/tools';
import {
  Code, Trash2, Plus, Copy, FileCode, Play, CheckCircle, XCircle,
  History, Terminal, BookOpen, Filter, ChevronDown, ChevronUp,
  ArrowUp, ArrowDown, Zap, Search,
} from 'lucide-react';
import { ScriptGuide } from './ScriptGuide';

const HOOK_OPTIONS = [
  { value: 'on_request', label: 'on_request' },
  { value: 'on_response', label: 'on_response' },
  { value: 'on_websocket_message', label: 'on_websocket_message' },
  { value: 'on_grpc_message', label: 'on_grpc_message' },
  { value: 'on_traffic_store', label: 'on_traffic_store' },
  { value: 'on_session_start', label: 'on_session_start' },
  { value: 'on_session_end', label: 'on_session_end' },
];

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];

/** Starter skeleton for a new blank script. */
const BLANK_SCRIPT_SOURCE = `// Define a hook function (onRequest, onResponse, etc.)
function onRequest(request, context) {
    // Your code here
    console.log(request.method + ' ' + request.url);
    return { continue: true };
}
`;

/** Build a human-readable summary of a match filter. */
function matchSummary(match?: ScriptMatch | null): string | null {
  if (!match) return null;
  const parts: string[] = [];
  if (match.method) parts.push(match.method);
  if (match.host_pattern) parts.push(`host:${match.host_pattern}`);
  if (match.path_pattern) parts.push(`path:${match.path_pattern}`);
  if (match.url_pattern) parts.push(`url:${match.url_pattern}`);
  return parts.length > 0 ? parts.join(' • ') : null;
}

export function ScriptsPanel() {
  const [activeTab, setActiveTab] = useState('guide');
  const [search, setSearch] = useState('');
  const [selectedScript, setSelectedScript] = useState<Script | null>(null);
  const [editingSource, setEditingSource] = useState('');
  const [editingName, setEditingName] = useState('');
  const [editingHooks, setEditingHooks] = useState<string[]>([]);
  const [editingMatch, setEditingMatch] = useState<ScriptMatch | null>(null);
  const [showTestDialog, setShowTestDialog] = useState(false);
  const [testResult, setTestResult] = useState<ScriptTestResult | null>(null);
  const [testHook, setTestHook] = useState('on_request');
  const [historyScriptId, setHistoryScriptId] = useState<string | null>(null);
  // Match-preview state
  const [previewMethod, setPreviewMethod] = useState('GET');
  const [previewHost, setPreviewHost] = useState('');
  const [previewPath, setPreviewPath] = useState('');
  const [previewUrl, setPreviewUrl] = useState('');
  const [previewHook, setPreviewHook] = useState('');
  const [previewResult, setPreviewResult] = useState<MatchPreviewItem[] | null>(null);

  const { data: scripts = [] } = useScripts();
  const { data: templates = [] } = useScriptTemplates();
  const createScript = useCreateScript();
  const updateScript = useUpdateScript();
  const deleteScript = useDeleteScript();
  const toggleScript = useToggleScript();
  const testScript = useTestScript();
  const validateScript = useValidateScript();
  const reorderScript = useReorderScript();
  const matchPreview = useMatchPreview();
  const { data: history = [] } = useScriptHistory(historyScriptId);

  // Sort scripts by priority (then created_at) for display so the list
  // reflects execution order.
  const sortedScripts = [...scripts].sort((a, b) => {
    const pa = a.priority ?? 100;
    const pb = b.priority ?? 100;
    if (pa !== pb) return pa - pb;
    return a.created_at.localeCompare(b.created_at);
  });

  const filteredScripts = sortedScripts.filter((s) => {
    if (!search) return true;
    return s.name.toLowerCase().includes(search.toLowerCase()) ||
           s.source.toLowerCase().includes(search.toLowerCase());
  });

  const handleCreateFromTemplate = (template: ScriptTemplate) => {
    createScript.mutate({
      name: template.name,
      source: template.source,
      description: template.description,
      enabled: true,
      hooks: template.hooks,
    });
  };

  const handleCreateBlank = () => {
    createScript.mutate(
      {
        name: 'New Script',
        source: BLANK_SCRIPT_SOURCE,
        description: 'Custom script — edit me',
        enabled: false,
        hooks: ['on_request'],
      },
      {
        onSuccess: () => {
          setActiveTab('scripts');
        },
      },
    );
  };

  const startEditing = (script: Script) => {
    setSelectedScript(script);
    setEditingSource(script.source);
    setEditingName(script.name);
    setEditingHooks(script.hooks);
    setEditingMatch(script.match_filter ?? null);
    setTestResult(null);
  };

  const cancelEditing = () => {
    setSelectedScript(null);
    setEditingSource('');
    setEditingName('');
    setEditingHooks([]);
    setEditingMatch(null);
    setTestResult(null);
  };

  const handleSave = () => {
    if (selectedScript) {
      const cleanMatch: ScriptMatch | null = editingMatch && Object.values(editingMatch).some(
        (v) => v && v.trim(),
      )
        ? Object.fromEntries(
            Object.entries(editingMatch).filter(([, v]) => v && v.trim()),
          ) as ScriptMatch
        : null;

      updateScript.mutate(
        {
          id: selectedScript.id,
          source: editingSource,
          name: editingName,
          hooks: editingHooks,
          match_filter: cleanMatch,
        },
        {
          onSuccess: () => {
            cancelEditing();
          },
        },
      );
    }
  };

  const handleValidate = () => {
    if (editingSource) {
      validateScript.mutate(editingSource, {
        onSuccess: (result) => {
          if (!result.valid) {
            setTestResult({
              modified: false,
              continue_: true,
              error: result.error,
              console: [],
              duration_ms: 0,
            });
          } else {
            setTestResult(null);
          }
        },
      });
    }
  };

  const handleTest = () => {
    if (editingSource) {
      testScript.mutate(
        { source: editingSource, hook: testHook },
        {
          onSuccess: (result) => setTestResult(result),
        },
      );
    }
  };

  const toggleHook = (hook: string) => {
    setEditingHooks((prev) =>
      prev.includes(hook) ? prev.filter((h) => h !== hook) : [...prev, hook],
    );
  };

  const handlePreview = () => {
    matchPreview.mutate(
      {
        method: previewMethod,
        url: previewUrl || `https://${previewHost}${previewPath}`,
        host: previewHost,
        path: previewPath,
        hook: previewHook || undefined,
      },
      {
        onSuccess: (items) => setPreviewResult(items),
      },
    );
  };

  const tabs = [
    { value: 'guide', label: 'Guide', icon: <BookOpen className="w-4 h-4" /> },
    { value: 'scripts', label: 'Scripts', icon: <FileCode className="w-4 h-4" /> },
    { value: 'preview', label: 'Preview', icon: <Zap className="w-4 h-4" /> },
    { value: 'templates', label: 'Templates', icon: <Copy className="w-4 h-4" /> },
    { value: 'history', label: 'History', icon: <History className="w-4 h-4" /> },
  ];

  return (
    <div className="h-full flex flex-col">
      <div className="p-2 border-b">
        <div className="flex items-center gap-2">
          <Input
            placeholder="Search scripts..."
            className="h-7 text-xs"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <Button
            size="sm"
            className="h-7 text-xs whitespace-nowrap"
            onClick={handleCreateBlank}
            disabled={createScript.isPending}
            title="Create a blank script from scratch"
          >
            <Plus className="w-3 h-3 mr-1" />
            New Script
          </Button>
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 min-h-0 flex flex-col">
        <TabsList className="grid grid-cols-5 h-9 p-0 m-2">
          {tabs.map((tab) => (
            <TabsTrigger key={tab.value} value={tab.value} className="text-xs py-1 gap-1">
              {tab.icon}
              <span>{tab.label}</span>
            </TabsTrigger>
          ))}
        </TabsList>

        <ScrollArea className="flex-1 min-h-0">
          <TabsContent value="guide" className="m-0 p-0">
            <ScriptGuide />
          </TabsContent>

          <TabsContent value="scripts" className="m-0 p-2">
            {selectedScript ? (
              <ScriptEditor
                name={editingName}
                source={editingSource}
                hooks={editingHooks}
                matchFilter={editingMatch}
                onNameChange={setEditingName}
                onSourceChange={setEditingSource}
                onHooksChange={toggleHook}
                onMatchChange={setEditingMatch}
                onSave={handleSave}
                onCancel={cancelEditing}
                onValidate={handleValidate}
                onTest={() => setShowTestDialog(true)}
                isSaving={updateScript.isPending}
                isValidating={validateScript.isPending}
                testResult={testResult}
              />
            ) : filteredScripts.length === 0 ? (
              <div className="text-xs text-muted-foreground text-center py-8 space-y-2">
                <p>No scripts yet.</p>
                <p>Click <strong>New Script</strong> above to start from scratch, or use a template.</p>
              </div>
            ) : (
              <div className="space-y-2">
                {filteredScripts.map((script, index) => (
                  <ScriptItem
                    key={script.id}
                    script={script}
                    canMoveUp={index > 0}
                    canMoveDown={index < filteredScripts.length - 1}
                    onToggle={(id, enabled) => toggleScript.mutate({ id, enabled })}
                    onDelete={(id) => deleteScript.mutate(id)}
                    onEdit={() => startEditing(script)}
                    onShowHistory={(id) => {
                      setHistoryScriptId(id);
                      setActiveTab('history');
                    }}
                    onReorder={(id, direction) => reorderScript.mutate({ id, direction })}
                    isReordering={reorderScript.isPending}
                    onErrorChange={(id, on_error) =>
                      updateScript.mutate({ id, on_error })
                    }
                  />
                ))}
              </div>
            )}
          </TabsContent>

          <TabsContent value="preview" className="m-0 p-2">
            <MatchPreviewPanel
              method={previewMethod}
              host={previewHost}
              path={previewPath}
              url={previewUrl}
              hook={previewHook}
              onMethodChange={setPreviewMethod}
              onHostChange={setPreviewHost}
              onPathChange={setPreviewPath}
              onUrlChange={setPreviewUrl}
              onHookChange={setPreviewHook}
              onRun={handlePreview}
              isRunning={matchPreview.isPending}
              result={previewResult}
            />
          </TabsContent>

          <TabsContent value="templates" className="m-0 p-2">
            <div className="space-y-2">
              {templates.length === 0 ? (
                <div className="text-xs text-muted-foreground text-center py-4">
                  No templates available
                </div>
              ) : (
                templates.map((template, index) => (
                  <TemplateItem
                    key={index}
                    template={template}
                    onCreate={handleCreateFromTemplate}
                  />
                ))
              )}
            </div>
          </TabsContent>

          <TabsContent value="history" className="m-0 p-2">
            <HistoryTab
              scripts={scripts}
              focusedScriptId={historyScriptId}
              perScriptHistory={history}
              onClearFocus={() => setHistoryScriptId(null)}
            />
          </TabsContent>
        </ScrollArea>
      </Tabs>

      {showTestDialog && (
        <TestDialog
          source={editingSource}
          hook={testHook}
          onHookChange={setTestHook}
          onRun={handleTest}
          onClose={() => setShowTestDialog(false)}
          result={testResult}
          isRunning={testScript.isPending}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Script list item
// ---------------------------------------------------------------------------

interface ScriptItemProps {
  script: Script;
  canMoveUp: boolean;
  canMoveDown: boolean;
  onToggle: (id: string, enabled: boolean) => void;
  onDelete: (id: string) => void;
  onEdit: () => void;
  onShowHistory: (id: string) => void;
  onReorder: (id: string, direction: 'up' | 'down') => void;
  isReordering: boolean;
  onErrorChange: (id: string, on_error: ScriptErrorPolicy) => void;
}

function ScriptItem({
  script, canMoveUp, canMoveDown, onToggle, onDelete, onEdit, onShowHistory, onReorder, isReordering, onErrorChange,
}: ScriptItemProps) {
  const matchStr = matchSummary(script.match_filter);
  const onError = script.on_error ?? 'stop_chain';
  return (
    <div className="border rounded-lg p-3 text-xs hover:shadow-sm transition-shadow bg-card">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          <Switch
            checked={script.enabled}
            onCheckedChange={(checked) => onToggle(script.id, checked)}
            className="scale-75"
          />
          <span className="font-medium truncate">{script.name}</span>
          {!script.enabled && (
            <span className="text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">
              disabled
            </span>
          )}
          <span className="text-[10px] text-muted-foreground font-mono">
            #{script.priority ?? 100}
          </span>
        </div>
        <div className="flex gap-0.5 shrink-0">
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => onReorder(script.id, 'up')}
            disabled={!canMoveUp || isReordering}
            title="Move up (run earlier)"
          >
            <ArrowUp className="w-3.5 h-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => onReorder(script.id, 'down')}
            disabled={!canMoveDown || isReordering}
            title="Move down (run later)"
          >
            <ArrowDown className="w-3.5 h-3.5" />
          </Button>
          <Button variant="ghost" size="sm" className="h-7 w-7 p-0" onClick={onEdit} title="Edit">
            <Code className="w-3.5 h-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => onShowHistory(script.id)}
            title="View history"
          >
            <History className="w-3.5 h-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0 text-destructive"
            onClick={() => onDelete(script.id)}
            title="Delete"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>
      {script.description && (
        <p className="text-muted-foreground mt-1.5 text-[11px]">{script.description}</p>
      )}
      <div className="flex gap-1 mt-2 flex-wrap items-center">
        {script.hooks.map((hook) => (
          <span key={hook} className="px-1.5 py-0.5 bg-primary/10 text-primary rounded text-[10px] font-mono">
            {hook}
          </span>
        ))}
        {matchStr && (
          <span className="flex items-center gap-1 px-1.5 py-0.5 bg-blue-500/10 text-blue-600 dark:text-blue-400 rounded text-[10px] font-mono">
            <Filter className="w-2.5 h-2.5" />
            {matchStr}
          </span>
        )}
        {/* Per-script error policy selector */}
        <span className="flex items-center gap-1 ml-auto">
          <span className="text-[10px] text-muted-foreground">On error:</span>
          <select
            value={onError}
            onChange={(e) => onErrorChange(script.id, e.target.value as ScriptErrorPolicy)}
            className="h-5 text-[10px] border rounded px-1 bg-background"
            title="What happens to subsequent scripts when this script fails"
          >
            <option value="stop_chain">Stop chain</option>
            <option value="continue">Continue</option>
          </select>
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Script editor
// ---------------------------------------------------------------------------

interface ScriptEditorProps {
  name: string;
  source: string;
  hooks: string[];
  matchFilter: ScriptMatch | null;
  onNameChange: (name: string) => void;
  onSourceChange: (source: string) => void;
  onHooksChange: (hook: string) => void;
  onMatchChange: (match: ScriptMatch | null) => void;
  onSave: () => void;
  onCancel: () => void;
  onValidate: () => void;
  onTest: () => void;
  isSaving: boolean;
  isValidating: boolean;
  testResult: ScriptTestResult | null;
}

function ScriptEditor({
  name, source, hooks, matchFilter,
  onNameChange, onSourceChange, onHooksChange, onMatchChange,
  onSave, onCancel, onValidate, onTest,
  isSaving, isValidating, testResult,
}: ScriptEditorProps) {
  const [showMatchFilter, setShowMatchFilter] = useState(
    matchFilter !== null && matchFilter !== undefined,
  );

  const updateMatchField = (field: keyof ScriptMatch, value: string) => {
    const current = matchFilter ?? {};
    onMatchChange({ ...current, [field]: value || undefined });
  };

  return (
    <div className="space-y-3">
      {/* Toolbar */}
      <div className="flex items-center justify-between sticky top-0 bg-background z-10 pb-2 border-b">
        <Input
          value={name}
          onChange={(e) => onNameChange(e.target.value)}
          className="h-7 text-xs font-medium max-w-[200px]"
        />
        <div className="flex gap-1">
          <Button size="sm" className="h-7 text-xs" variant="outline" onClick={onValidate} disabled={isValidating}>
            <CheckCircle className="w-3 h-3 mr-1" />
            {isValidating ? 'Checking...' : 'Validate'}
          </Button>
          <Button size="sm" className="h-7 text-xs" variant="secondary" onClick={onTest}>
            <Play className="w-3 h-3 mr-1" />
            Test
          </Button>
          <Button size="sm" className="h-7 text-xs" onClick={onSave} disabled={isSaving}>
            {isSaving ? 'Saving...' : 'Save'}
          </Button>
          <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={onCancel}>
            Cancel
          </Button>
        </div>
      </div>

      {/* Hooks selector */}
      <div className="space-y-1.5">
        <Label className="text-[11px] text-muted-foreground">Hooks</Label>
        <div className="flex gap-1 flex-wrap">
          {HOOK_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              onClick={() => onHooksChange(opt.value)}
              className={`px-2 py-1 rounded text-[10px] font-mono transition-colors border ${
                hooks.includes(opt.value)
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'bg-background text-muted-foreground border-border hover:bg-muted'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      {/* Match filter */}
      <div className="border rounded-lg overflow-hidden">
        <button
          onClick={() => {
            if (!showMatchFilter) onMatchChange({});
            setShowMatchFilter(!showMatchFilter);
          }}
          className="w-full flex items-center justify-between p-2 hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2 text-xs">
            <Filter className="w-3.5 h-3.5 text-blue-500" />
            <span className="font-medium">Match Filter</span>
            {matchFilter && matchSummary(matchFilter) && (
              <span className="text-[10px] text-muted-foreground font-mono">
                {matchSummary(matchFilter)}
              </span>
            )}
            {!matchFilter && (
              <span className="text-[10px] text-muted-foreground">applies to all requests</span>
            )}
          </div>
          {showMatchFilter ? (
            <ChevronUp className="w-3.5 h-3.5 text-muted-foreground" />
          ) : (
            <ChevronDown className="w-3.5 h-3.5 text-muted-foreground" />
          )}
        </button>
        {showMatchFilter && (
          <div className="p-2.5 space-y-2.5 border-t bg-muted/30">
            <p className="text-[10px] text-muted-foreground">
              Restrict this script to specific requests. Patterns support{' '}
              <code className="font-mono">*</code> (any sequence) and{' '}
              <code className="font-mono">?</code> (single char). Leave empty to match all.
            </p>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <Label className="text-[10px] text-muted-foreground">Method</Label>
                <select
                  value={matchFilter?.method ?? ''}
                  onChange={(e) => updateMatchField('method', e.target.value)}
                  className="w-full h-7 text-[11px] border rounded px-1.5 bg-background mt-0.5"
                >
                  <option value="">Any method</option>
                  {HTTP_METHODS.map((m) => (
                    <option key={m} value={m}>{m}</option>
                  ))}
                </select>
              </div>
              <div>
                <Label className="text-[10px] text-muted-foreground">Host pattern</Label>
                <Input
                  placeholder="*.example.com"
                  value={matchFilter?.host_pattern ?? ''}
                  onChange={(e) => updateMatchField('host_pattern', e.target.value)}
                  className="h-7 text-[11px] mt-0.5 font-mono"
                />
              </div>
              <div>
                <Label className="text-[10px] text-muted-foreground">Path pattern</Label>
                <Input
                  placeholder="/api/v2/*"
                  value={matchFilter?.path_pattern ?? ''}
                  onChange={(e) => updateMatchField('path_pattern', e.target.value)}
                  className="h-7 text-[11px] mt-0.5 font-mono"
                />
              </div>
              <div>
                <Label className="text-[10px] text-muted-foreground">URL pattern</Label>
                <Input
                  placeholder="*/api/users*"
                  value={matchFilter?.url_pattern ?? ''}
                  onChange={(e) => updateMatchField('url_pattern', e.target.value)}
                  className="h-7 text-[11px] mt-0.5 font-mono"
                />
              </div>
            </div>
            {matchFilter && matchSummary(matchFilter) && (
              <Button
                size="sm"
                variant="ghost"
                className="h-6 text-[10px] text-destructive"
                onClick={() => onMatchChange(null)}
              >
                <Trash2 className="w-2.5 h-2.5 mr-1" />
                Clear filter
              </Button>
            )}
          </div>
        )}
      </div>

      {/* Code editor */}
      <div className="space-y-1">
        <Label className="text-[11px] text-muted-foreground">Source code</Label>
        <div className="w-full h-56 rounded-lg border bg-muted overflow-auto">
          <Editor
            value={source}
            onValueChange={(code) => onSourceChange(code)}
            highlight={(code) =>
              Prism.highlight(code, Prism.languages.javascript, 'javascript')
            }
            padding={10}
            className="w-full min-h-full text-xs font-mono"
            textareaClassName="outline-none"
            style={{
              fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
              fontSize: 12,
              minHeight: '100%',
            }}
          />
        </div>
      </div>

      {testResult && <TestResultDisplay result={testResult} />}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Template item
// ---------------------------------------------------------------------------

interface TemplateItemProps {
  template: ScriptTemplate;
  onCreate: (template: ScriptTemplate) => void;
}

function TemplateItem({ template, onCreate }: TemplateItemProps) {
  return (
    <div className="border rounded-lg p-3 text-xs hover:shadow-sm transition-shadow bg-card">
      <div className="flex items-center justify-between">
        <span className="font-medium">{template.name}</span>
        <Button size="sm" className="h-7 text-xs" onClick={() => onCreate(template)}>
          <Plus className="w-3 h-3 mr-1" />
          Use
        </Button>
      </div>
      <p className="text-muted-foreground mt-1.5 text-[11px]">{template.description}</p>
      <div className="flex gap-1 mt-2 flex-wrap">
        {template.hooks.map((hook) => (
          <span key={hook} className="px-1.5 py-0.5 bg-primary/10 text-primary rounded text-[10px] font-mono">
            {hook}
          </span>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Test result display
// ---------------------------------------------------------------------------

interface TestResultDisplayProps {
  result: ScriptTestResult;
}

function TestResultDisplay({ result }: TestResultDisplayProps) {
  return (
    <div className="border rounded-lg p-2.5 text-xs space-y-1.5 bg-card">
      <div className="flex items-center gap-2">
        {result.error ? (
          <>
            <XCircle className="w-3.5 h-3.5 text-destructive" />
            <span className="text-destructive font-medium">Error</span>
            <span className="text-muted-foreground">{result.duration_ms}ms</span>
          </>
        ) : (
          <>
            <CheckCircle className="w-3.5 h-3.5 text-green-500" />
            <span className="text-green-500 font-medium">Success</span>
            <span className="text-muted-foreground">{result.duration_ms}ms</span>
            {result.modified && (
              <span className="px-1.5 py-0.5 bg-blue-500/20 text-blue-600 dark:text-blue-400 rounded text-[10px]">modified</span>
            )}
            {!result.continue_ && (
              <span className="px-1.5 py-0.5 bg-orange-500/20 text-orange-600 dark:text-orange-400 rounded text-[10px]">short-circuited</span>
            )}
          </>
        )}
      </div>
      {result.error && (
        <pre className="text-destructive text-[10px] whitespace-pre-wrap font-mono bg-destructive/10 p-2 rounded">
          {result.error}
        </pre>
      )}
      {result.console.length > 0 && (
        <div className="space-y-0.5">
          <div className="flex items-center gap-1 text-muted-foreground">
            <Terminal className="w-3 h-3" />
            <span className="text-[10px]">Console</span>
          </div>
          {result.console.map((line, i) => (
            <pre key={i} className="text-[10px] whitespace-pre-wrap font-mono bg-muted p-1.5 rounded">
              {line}
            </pre>
          ))}
        </div>
      )}
      {result.response && (
        <div className="space-y-0.5">
          <span className="text-muted-foreground text-[10px]">Response (short-circuit)</span>
          <pre className="text-[10px] whitespace-pre-wrap font-mono bg-muted p-1.5 rounded">
            {result.response.statusCode} {result.response.body.slice(0, 200)}
          </pre>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Test dialog
// ---------------------------------------------------------------------------

interface TestDialogProps {
  source: string;
  hook: string;
  onHookChange: (hook: string) => void;
  onRun: () => void;
  onClose: () => void;
  result: ScriptTestResult | null;
  isRunning: boolean;
}

function TestDialog({ hook, onHookChange, onRun, onClose, result, isRunning }: TestDialogProps) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background border rounded-lg p-4 w-[500px] max-h-[80vh] overflow-auto">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium">Test Script</h3>
          <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onClose}>
            ✕
          </Button>
        </div>
        <div className="space-y-3">
          <div>
            <label className="text-xs text-muted-foreground mb-1 block">Hook</label>
            <select
              value={hook}
              onChange={(e) => onHookChange(e.target.value)}
              className="w-full h-8 text-xs border rounded px-2 bg-background"
            >
              {HOOK_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>
          <div className="text-xs text-muted-foreground">
            The script will be executed against a sample request/response context.
            No live traffic will be affected and no history will be recorded.
          </div>
          <div className="flex gap-2">
            <Button size="sm" className="h-7 text-xs" onClick={onRun} disabled={isRunning}>
              <Play className="w-3 h-3 mr-1" />
              {isRunning ? 'Running...' : 'Run Test'}
            </Button>
            <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={onClose}>
              Close
            </Button>
          </div>
          {result && <TestResultDisplay result={result} />}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Match preview panel
// ---------------------------------------------------------------------------

interface MatchPreviewPanelProps {
  method: string;
  host: string;
  path: string;
  url: string;
  hook: string;
  onMethodChange: (v: string) => void;
  onHostChange: (v: string) => void;
  onPathChange: (v: string) => void;
  onUrlChange: (v: string) => void;
  onHookChange: (v: string) => void;
  onRun: () => void;
  isRunning: boolean;
  result: MatchPreviewItem[] | null;
}

function MatchPreviewPanel({
  method, host, path, url, hook,
  onMethodChange, onHostChange, onPathChange, onUrlChange, onHookChange,
  onRun, isRunning, result,
}: MatchPreviewPanelProps) {
  return (
    <div className="space-y-3">
      <div className="text-xs text-muted-foreground">
        Enter a sample request to see which scripts would fire (in execution
        order) without actually executing them.
      </div>
      <div className="grid grid-cols-4 gap-2">
        <div>
          <Label className="text-[10px] text-muted-foreground">Method</Label>
          <select
            value={method}
            onChange={(e) => onMethodChange(e.target.value)}
            className="w-full h-7 text-[11px] border rounded px-1.5 bg-background mt-0.5"
          >
            {HTTP_METHODS.map((m) => (
              <option key={m} value={m}>{m}</option>
            ))}
          </select>
        </div>
        <div>
          <Label className="text-[10px] text-muted-foreground">Host</Label>
          <Input
            placeholder="api.example.com"
            value={host}
            onChange={(e) => onHostChange(e.target.value)}
            className="h-7 text-[11px] mt-0.5 font-mono"
          />
        </div>
        <div>
          <Label className="text-[10px] text-muted-foreground">Path</Label>
          <Input
            placeholder="/api/v2/users"
            value={path}
            onChange={(e) => onPathChange(e.target.value)}
            className="h-7 text-[11px] mt-0.5 font-mono"
          />
        </div>
        <div>
          <Label className="text-[10px] text-muted-foreground">Hook filter</Label>
          <select
            value={hook}
            onChange={(e) => onHookChange(e.target.value)}
            className="w-full h-7 text-[11px] border rounded px-1.5 bg-background mt-0.5"
          >
            <option value="">Any hook</option>
            {HOOK_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>{opt.label}</option>
            ))}
          </select>
        </div>
      </div>
      <div>
        <Label className="text-[10px] text-muted-foreground">Full URL (optional — overrides host/path)</Label>
        <Input
          placeholder="https://api.example.com/api/v2/users?limit=10"
          value={url}
          onChange={(e) => onUrlChange(e.target.value)}
          className="h-7 text-[11px] mt-0.5 font-mono"
        />
      </div>
      <Button size="sm" className="h-7 text-xs" onClick={onRun} disabled={isRunning}>
        <Search className="w-3 h-3 mr-1" />
        {isRunning ? 'Checking...' : 'Check matches'}
      </Button>

      {result !== null && (
        <div className="space-y-2">
          {result.length === 0 ? (
            <div className="text-xs text-muted-foreground text-center py-4 border rounded-lg bg-muted/30">
              No scripts would match this request.
            </div>
          ) : (
            <>
              <div className="text-[11px] text-muted-foreground">
                {result.length} script{result.length !== 1 ? 's' : ''} would fire (in order):
              </div>
              {result.map((item, i) => {
                const matchStr = matchSummary(item.match_filter);
                return (
                  <div key={item.id} className="border rounded-lg p-2.5 text-xs bg-card">
                    <div className="flex items-center gap-2">
                      <span className="text-muted-foreground font-mono text-[10px]">
                        {i + 1}.
                      </span>
                      <span className="font-medium">{item.name}</span>
                      {!item.enabled && (
                        <span className="text-[10px] text-muted-foreground bg-muted px-1 py-0.5 rounded">
                          disabled
                        </span>
                      )}
                      <span className="text-[10px] text-muted-foreground font-mono ml-auto">
                        #{item.priority}
                      </span>
                    </div>
                    <div className="flex gap-1 mt-1.5 flex-wrap items-center">
                      {item.hooks.map((h) => (
                        <span key={h} className="px-1.5 py-0.5 bg-primary/10 text-primary rounded text-[10px] font-mono">
                          {h}
                        </span>
                      ))}
                      {matchStr && (
                        <span className="flex items-center gap-1 px-1.5 py-0.5 bg-blue-500/10 text-blue-600 dark:text-blue-400 rounded text-[10px] font-mono">
                          <Filter className="w-2.5 h-2.5" />
                          {matchStr}
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
            </>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// History tab — global execution history with optional per-script filter
// ---------------------------------------------------------------------------

interface HistoryTabProps {
  scripts: Script[];
  /** When set (by clicking the history icon on a script), the view is
   *  filtered to that script only.  `null` shows the global view. */
  focusedScriptId: string | null;
  /** Per-script history fetched via `useScriptHistory(focusedScriptId)`.
   *  Used only when `focusedScriptId` is set. */
  perScriptHistory: Array<{
    script_id: string;
    duration_ms: number;
    success: boolean;
    error?: string;
    console: string[];
    timestamp: string;
    traffic_entry_id?: string;
    hook?: string;
  }>;
  onClearFocus: () => void;
}

function HistoryTab({
  scripts,
  focusedScriptId,
  perScriptHistory,
  onClearFocus,
}: HistoryTabProps) {
  // Global history (all scripts).  Polls every 3s.
  const { data: allHistory = [], isLoading } = useAllScriptHistory(100);

  // Build a lookup table for script names so we can show a readable label
  // even for executions whose script has been deleted.
  const nameById = new Map<string, string>();
  for (const s of scripts) nameById.set(s.id, s.name);

  // Local filter state — lets the user narrow the global view by script
  // or by success/failure status without leaving the tab.
  const [filterScriptId, setFilterScriptId] = useState<string>('');
  const [filterStatus, setFilterStatus] = useState<'all' | 'success' | 'error'>('all');

  // If the user clicked the history icon on a specific script, we show
  // the per-script view (which reads from the dedicated per-script
  // endpoint and has its own polling).  Otherwise we show the global
  // view with optional client-side filters.
  if (focusedScriptId) {
    const focusedName = nameById.get(focusedScriptId) ?? focusedScriptId.slice(0, 8);
    return (
      <div className="space-y-2">
        <div className="flex items-center gap-2 text-xs">
          <span className="text-muted-foreground">
            Filtered to: <span className="font-medium text-foreground">{focusedName}</span>
          </span>
          <Button
            size="sm"
            variant="ghost"
            className="h-6 text-[11px] px-2"
            onClick={onClearFocus}
          >
            Show all
          </Button>
        </div>
        <HistoryList
          entries={perScriptHistory.map((e) => ({
            ...e,
            script_name: nameById.get(e.script_id) ?? null,
          }))}
          emptyMessage="No execution history for this script yet."
        />
      </div>
    );
  }

  if (isLoading && allHistory.length === 0) {
    return (
      <div className="text-xs text-muted-foreground text-center py-4">
        Loading execution history…
      </div>
    );
  }

  // Apply client-side filters to the global history.
  const filtered = allHistory.filter((e) => {
    if (filterScriptId && e.script_id !== filterScriptId) return false;
    if (filterStatus === 'success' && !e.success) return false;
    if (filterStatus === 'error' && e.success) return false;
    return true;
  });

  return (
    <div className="space-y-2">
      {/* Filter bar */}
      <div className="flex items-center gap-2 text-[11px] flex-wrap">
        <span className="text-muted-foreground">Filter:</span>
        <select
          value={filterScriptId}
          onChange={(e) => setFilterScriptId(e.target.value)}
          className="h-6 text-[11px] border rounded px-1.5 bg-background"
        >
          <option value="">All scripts</option>
          {scripts.map((s) => (
            <option key={s.id} value={s.id}>{s.name}</option>
          ))}
        </select>
        <select
          value={filterStatus}
          onChange={(e) => setFilterStatus(e.target.value as 'all' | 'success' | 'error')}
          className="h-6 text-[11px] border rounded px-1.5 bg-background"
        >
          <option value="all">All statuses</option>
          <option value="success">Success only</option>
          <option value="error">Errors only</option>
        </select>
        <span className="text-muted-foreground ml-auto">
          {filtered.length} execution{filtered.length !== 1 ? 's' : ''}
        </span>
      </div>

      <HistoryList
        entries={filtered}
        emptyMessage={
          allHistory.length === 0
            ? 'No script executions yet.  Enable a script and send traffic through the proxy to see executions here.'
            : 'No executions match the current filters.'
        }
      />
    </div>
  );
}

/** Render a list of execution entries. */
function HistoryList({
  entries,
  emptyMessage,
}: {
  entries: ScriptHistoryEntry[];
  emptyMessage: string;
}) {
  if (entries.length === 0) {
    return (
      <div className="text-xs text-muted-foreground text-center py-4">
        {emptyMessage}
      </div>
    );
  }
  return (
    <div className="space-y-1.5">
      {entries.map((exec, i) => (
        <div key={i} className="border rounded-lg p-2.5 text-xs bg-card">
          <div className="flex items-center gap-2 flex-wrap">
            {exec.success ? (
              <CheckCircle className="w-3.5 h-3.5 text-green-500 shrink-0" />
            ) : (
              <XCircle className="w-3.5 h-3.5 text-destructive shrink-0" />
            )}
            <span className="font-medium">
              {exec.script_name ?? exec.script_id.slice(0, 8)}
            </span>
            {exec.hook && (
              <span className="px-1.5 py-0.5 bg-primary/10 text-primary rounded text-[10px] font-mono">
                {exec.hook}
              </span>
            )}
            <span className="text-muted-foreground ml-auto">
              {new Date(exec.timestamp).toLocaleString()}
            </span>
            <span className="text-muted-foreground">{exec.duration_ms}ms</span>
          </div>
          {exec.error && (
            <pre className="text-destructive text-[10px] whitespace-pre-wrap font-mono bg-destructive/10 p-1.5 rounded mt-1.5">
              {exec.error}
            </pre>
          )}
          {exec.console.length > 0 && (
            <div className="mt-1.5 space-y-0.5">
              <div className="flex items-center gap-1 text-muted-foreground">
                <Terminal className="w-3 h-3" />
                <span className="text-[10px]">Console</span>
              </div>
              {exec.console.map((line, j) => (
                <pre key={j} className="text-[10px] whitespace-pre-wrap font-mono bg-muted p-1.5 rounded">
                  {line}
                </pre>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
