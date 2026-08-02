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
  useRewrites,
  useCreateRewrite,
  useUpdateRewrite,
  useDeleteRewrite,
  useToggleRewrite,
  type RewriteRule,
  type RewriteAction,
  type MatchCondition,
} from '@/lib/api/intercept';
import { useToast } from '@/components/ui/use-toast';
import { Search, Plus, MoreVertical, Layers, Check, X, FileDown, GripVertical } from 'lucide-react';

interface RewritesPanelProps {
  onEditRewrite?: (rewrite: RewriteRule) => void;
}

// Rewrite templates
const REWRITE_TEMPLATES: {
  name: string;
  description?: string;
  actionType: 'set_header' | 'remove_header' | 'url_rewrite' | 'body_rewrite';
  headerName: string;
  headerValue?: string;
  direction: 'request' | 'response' | 'both';
  // Optional multi-action preset. When present, `handleCreate` uses these
  // actions directly instead of the single-action form fields.
  actions?: RewriteAction[];
}[] = [
  {
    name: 'Add CORS Headers',
    actionType: 'set_header',
    headerName: 'Access-Control-Allow-Origin',
    headerValue: '*',
    direction: 'response',
  },
  {
    name: 'Add Authorization Header',
    actionType: 'set_header',
    headerName: 'Authorization',
    headerValue: 'Bearer YOUR_TOKEN',
    direction: 'request',
  },
  {
    name: 'Remove Security Headers',
    actionType: 'remove_header',
    headerName: 'X-Frame-Options',
    direction: 'response',
  },
  {
    name: 'Redirect to Localhost',
    actionType: 'url_rewrite',
    headerName: 'https://api.example.com',
    headerValue: 'http://localhost:3000',
    direction: 'request',
  },
  {
    name: 'No Caching',
    description:
      'Strip cache-related headers and add no-cache directives so every request reaches the server.',
    actionType: 'remove_header',
    headerName: 'If-Modified-Since',
    direction: 'both',
    actions: [
      { type: 'remove_header', name: 'If-Modified-Since' },
      { type: 'remove_header', name: 'If-None-Match' },
      { type: 'remove_header', name: 'ETag' },
      { type: 'remove_header', name: 'Last-Modified' },
      { type: 'remove_header', name: 'Expires' },
      { type: 'set_header', name: 'Cache-Control', value: 'no-cache, no-store, must-revalidate' },
      { type: 'set_header', name: 'Pragma', value: 'no-cache' },
      { type: 'set_header', name: 'Expires', value: '0' },
    ],
  },
  {
    name: 'Block Cookies',
    description:
      'Strip Cookie and Set-Cookie headers from both requests and responses for anonymous-visitor testing.',
    actionType: 'remove_header',
    headerName: 'Cookie',
    direction: 'both',
    actions: [
      { type: 'remove_header', name: 'Cookie' },
      { type: 'remove_header', name: 'Set-Cookie' },
    ],
  },
];

export function RewritesPanel({ onEditRewrite }: RewritesPanelProps) {
  const { data: rewrites, isLoading } = useRewrites();
  const createRewrite = useCreateRewrite();
  const updateRewrite = useUpdateRewrite();
  const deleteRewrite = useDeleteRewrite();
  const toggleRewrite = useToggleRewrite();
  const { toast } = useToast();

  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [editingRewrite, setEditingRewrite] = useState<RewriteRule | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedTemplate, setSelectedTemplate] = useState<number | null>(null);
  const [newRewrite, setNewRewrite] = useState({
    name: '',
    urlPattern: '',
    direction: 'request' as 'request' | 'response' | 'both',
    actionType: 'set_header' as RewriteAction['type'],
    headerName: '',
    headerValue: '',
  });
  const [editRewrite, setEditRewrite] = useState({
    name: '',
    urlPattern: '',
    direction: 'request' as 'request' | 'response' | 'both',
    actionType: 'set_header' as RewriteAction['type'],
    headerName: '',
    headerValue: '',
    enabled: true,
  });

  // Filter rewrites based on search term
  const filteredRewrites = useMemo(() => {
    if (!rewrites) return [];
    if (!searchTerm) return rewrites;

    const term = searchTerm.toLowerCase();
    return rewrites.filter(
      (rewrite) =>
        rewrite.name.toLowerCase().includes(term) ||
        getActionDescription(rewrite).toLowerCase().includes(term) ||
        rewrite.direction.toLowerCase().includes(term)
    );
  }, [rewrites, searchTerm]);

  // Stats
  const stats = useMemo(() => {
    if (!rewrites) return { total: 0, enabled: 0, hits: 0 };
    return {
      total: rewrites.length,
      enabled: rewrites.filter((r) => r.enabled).length,
      hits: rewrites.reduce((sum, r) => sum + (r.hit_count || 0), 0),
    };
  }, [rewrites]);

  const handleCreate = async () => {
    if (!newRewrite.name) {
      toast({
        title: 'Error',
        description: 'Name is required',
        variant: 'destructive',
      });
      return;
    }

    const condition: MatchCondition = newRewrite.urlPattern
      ? { type: 'url_pattern', pattern: newRewrite.urlPattern }
      : { type: 'all' };

    let actions: RewriteAction[];
    const template =
      selectedTemplate !== null ? REWRITE_TEMPLATES[selectedTemplate] : null;
    if (template && template.actions && template.actions.length > 0) {
      // Multi-action template (e.g. No Caching, Block Cookies) — use the
      // preset actions directly.
      actions = template.actions;
    } else {
      // Single-action form: build one action from the form fields.
      let action: RewriteAction;
      switch (newRewrite.actionType) {
        case 'set_header':
          action = { type: 'set_header', name: newRewrite.headerName, value: newRewrite.headerValue };
          break;
        case 'remove_header':
          action = { type: 'remove_header', name: newRewrite.headerName };
          break;
        case 'url_rewrite':
          action = { type: 'url_rewrite', pattern: newRewrite.headerName, replacement: newRewrite.headerValue };
          break;
        case 'body_rewrite':
          action = { type: 'body_rewrite', pattern: newRewrite.headerName, replacement: newRewrite.headerValue };
          break;
        default:
          action = { type: 'set_header', name: '', value: '' };
      }
      actions = [action];
    }

    await createRewrite.mutateAsync({
      name: newRewrite.name,
      condition,
      direction: newRewrite.direction,
      rewrites: actions,
      enabled: true,
    });

    setShowCreateDialog(false);
    setNewRewrite({
      name: '',
      urlPattern: '',
      direction: 'request',
      actionType: 'set_header',
      headerName: '',
      headerValue: '',
    });
    setSelectedTemplate(null);

    toast({
      title: 'Rewrite Created',
      description: `Rewrite "${newRewrite.name}" has been created`,
    });
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    await toggleRewrite.mutateAsync({ id, enabled });
    toast({
      title: enabled ? 'Rewrite Enabled' : 'Rewrite Disabled',
      description: `Rewrite has been ${enabled ? 'enabled' : 'disabled'}`,
    });
  };

  const handleDelete = async (id: string, name: string) => {
    await deleteRewrite.mutateAsync(id);
    toast({
      title: 'Rewrite Deleted',
      description: `Rewrite "${name}" has been deleted`,
    });
  };

  // Derive single-action form state from an existing rule. Multi-action
  // rules are represented by their first action so the user can still edit
  // the name, URL pattern, direction, and the leading action.
  const deriveEditState = (rewrite: RewriteRule) => {
    const urlPattern =
      rewrite.condition.type === 'url_pattern' ? rewrite.condition.pattern ?? '' : '';
    const action = rewrite.rewrites[0];
    let actionType: RewriteAction['type'] = 'set_header';
    let headerName = '';
    let headerValue = '';
    if (action) {
      switch (action.type) {
        case 'set_header':
          actionType = 'set_header';
          headerName = action.name ?? '';
          headerValue = action.value ?? '';
          break;
        case 'remove_header':
          actionType = 'remove_header';
          headerName = action.name ?? '';
          break;
        case 'url_rewrite':
          actionType = 'url_rewrite';
          headerName = action.pattern ?? '';
          headerValue = action.replacement ?? '';
          break;
        case 'body_rewrite':
          actionType = 'body_rewrite';
          headerName = action.pattern ?? '';
          headerValue = action.replacement ?? '';
          break;
      }
    }
    return {
      name: rewrite.name,
      urlPattern,
      direction: rewrite.direction,
      actionType,
      headerName,
      headerValue,
      enabled: rewrite.enabled,
    };
  };

  const handleEdit = (rewrite: RewriteRule) => {
    // Allow an external listener (e.g. a dedicated editor screen) to take
    // over; otherwise fall back to the built-in edit dialog.
    if (onEditRewrite) {
      onEditRewrite(rewrite);
      return;
    }
    setEditingRewrite(rewrite);
    setEditRewrite(deriveEditState(rewrite));
    setShowEditDialog(true);
  };

  const handleEditSubmit = async () => {
    if (!editingRewrite) return;
    if (!editRewrite.name) {
      toast({
        title: 'Error',
        description: 'Name is required',
        variant: 'destructive',
      });
      return;
    }

    const condition: MatchCondition = editRewrite.urlPattern
      ? { type: 'url_pattern', pattern: editRewrite.urlPattern }
      : { type: 'all' };

    // Preserve any additional actions beyond the first one so multi-action
    // rules (e.g. No Caching) aren't truncated when editing the leading
    // action or metadata.
    const preservedActions = editingRewrite.rewrites.slice(1);
    let action: RewriteAction;
    switch (editRewrite.actionType) {
      case 'set_header':
        action = { type: 'set_header', name: editRewrite.headerName, value: editRewrite.headerValue };
        break;
      case 'remove_header':
        action = { type: 'remove_header', name: editRewrite.headerName };
        break;
      case 'url_rewrite':
        action = { type: 'url_rewrite', pattern: editRewrite.headerName, replacement: editRewrite.headerValue };
        break;
      case 'body_rewrite':
        action = { type: 'body_rewrite', pattern: editRewrite.headerName, replacement: editRewrite.headerValue };
        break;
      default:
        action = { type: 'set_header', name: '', value: '' };
    }
    const actions = [action, ...preservedActions];

    await updateRewrite.mutateAsync({
      id: editingRewrite.id,
      rewrite: {
        name: editRewrite.name,
        condition,
        direction: editRewrite.direction,
        rewrites: actions,
        enabled: editRewrite.enabled,
      },
    });

    setShowEditDialog(false);
    setEditingRewrite(null);
    toast({
      title: 'Rewrite Updated',
      description: `Rewrite "${editRewrite.name}" has been updated`,
    });
  };

  const handleTemplateSelect = (index: number) => {
    const template = REWRITE_TEMPLATES[index];
    setSelectedTemplate(index);
    setNewRewrite({
      ...newRewrite,
      name: template.name,
      actionType: template.actionType,
      headerName: template.headerName,
      headerValue: template.headerValue ?? '',
      direction: template.direction,
    });
  };

  const handleEnableAll = async () => {
    if (!rewrites) return;
    for (const rewrite of rewrites) {
      if (!rewrite.enabled) {
        await toggleRewrite.mutateAsync({ id: rewrite.id, enabled: true });
      }
    }
    toast({ description: 'All rewrites enabled' });
  };

  const handleDisableAll = async () => {
    if (!rewrites) return;
    for (const rewrite of rewrites) {
      if (rewrite.enabled) {
        await toggleRewrite.mutateAsync({ id: rewrite.id, enabled: false });
      }
    }
    toast({ description: 'All rewrites disabled' });
  };

  const handleExport = () => {
    if (!rewrites || rewrites.length === 0) {
      toast({ title: 'No rewrites to export', variant: 'destructive' });
      return;
    }
    const data = JSON.stringify(rewrites, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'madhyamas-rewrites.json';
    a.click();
    URL.revokeObjectURL(url);
    toast({ description: 'Rewrites exported' });
  };

  const getActionDescription = (rewrite: RewriteRule): string => {
    if (!rewrite.rewrites.length) return 'No actions';
    // For multi-action rules (e.g. No Caching, Block Cookies), summarize
    // the action count rather than only showing the first action.
    if (rewrite.rewrites.length > 1) {
      const names = rewrite.rewrites.map((a) => a.name ?? a.type);
      return `${rewrite.rewrites.length} actions: ${names.slice(0, 3).join(', ')}${names.length > 3 ? ', …' : ''}`;
    }
    const action = rewrite.rewrites[0];
    switch (action.type) {
      case 'set_header':
        return `Set ${action.name}: ${action.value}`;
      case 'remove_header':
        return `Remove ${action.name}`;
      case 'url_rewrite':
        return `URL: ${action.pattern} → ${action.replacement}`;
      case 'body_rewrite':
        return `Body: ${action.pattern} → ${action.replacement}`;
      default:
        return action.type;
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading rewrites...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Rewrite Rules</h2>
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
                  Create
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-lg">
                <DialogHeader>
                  <DialogTitle>Create Rewrite Rule</DialogTitle>
                  <DialogDescription>
                    Create a rule to modify requests or responses
                  </DialogDescription>
                </DialogHeader>
                <div className="grid gap-4 py-4">
                  {/* Templates */}
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Quick Templates</label>
                    <div className="flex flex-wrap gap-1">
                      {REWRITE_TEMPLATES.map((template, index) => (
                        <Button
                          key={template.name}
                          variant={selectedTemplate === index ? 'default' : 'outline'}
                          size="sm"
                          onClick={() => handleTemplateSelect(index)}
                          className="h-7 text-xs"
                          title={template.description}
                        >
                          {template.name}
                        </Button>
                      ))}
                    </div>
                  </div>

                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Name</label>
                    <Input
                      placeholder="My Rewrite"
                      value={newRewrite.name}
                      onChange={(e) => {
                        setNewRewrite({ ...newRewrite, name: e.target.value });
                        setSelectedTemplate(null);
                      }}
                    />
                  </div>
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">URL Pattern (Regex, optional)</label>
                    <Input
                      placeholder=".*api/.*"
                      value={newRewrite.urlPattern}
                      onChange={(e) => setNewRewrite({ ...newRewrite, urlPattern: e.target.value })}
                    />
                  </div>
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Direction</label>
                    <Select
                      value={newRewrite.direction}
                      onValueChange={(v: 'request' | 'response' | 'both') => setNewRewrite({ ...newRewrite, direction: v })}
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
                  <div className="grid gap-2">
                    <label className="text-sm font-medium">Action Type</label>
                    <Select
                      value={newRewrite.actionType}
                      onValueChange={(v: 'set_header' | 'remove_header' | 'url_rewrite' | 'body_rewrite') => setNewRewrite({ ...newRewrite, actionType: v })}
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="set_header">Set Header</SelectItem>
                        <SelectItem value="remove_header">Remove Header</SelectItem>
                        <SelectItem value="url_rewrite">URL Rewrite</SelectItem>
                        <SelectItem value="body_rewrite">Body Rewrite</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  {(newRewrite.actionType === 'set_header' || newRewrite.actionType === 'remove_header') && (
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">Header Name</label>
                      <Input
                        placeholder="Authorization"
                        value={newRewrite.headerName}
                        onChange={(e) => setNewRewrite({ ...newRewrite, headerName: e.target.value })}
                      />
                    </div>
                  )}
                  {(newRewrite.actionType === 'set_header' || newRewrite.actionType === 'url_rewrite' || newRewrite.actionType === 'body_rewrite') && (
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">
                        {newRewrite.actionType === 'set_header' ? 'Header Value' : 'Replacement'}
                      </label>
                      <Input
                        placeholder={newRewrite.actionType === 'set_header' ? 'Bearer token123' : 'replacement text'}
                        value={newRewrite.headerValue}
                        onChange={(e) => setNewRewrite({ ...newRewrite, headerValue: e.target.value })}
                      />
                    </div>
                  )}
                  {(newRewrite.actionType === 'url_rewrite' || newRewrite.actionType === 'body_rewrite') && (
                    <div className="grid gap-2">
                      <label className="text-sm font-medium">Pattern (Regex)</label>
                      <Input
                        placeholder="http://localhost"
                        value={newRewrite.headerName}
                        onChange={(e) => setNewRewrite({ ...newRewrite, headerName: e.target.value })}
                      />
                    </div>
                  )}
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
                    Cancel
                  </Button>
                  <Button onClick={handleCreate} disabled={createRewrite.isPending}>
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
            placeholder="Search rewrites..."
            className="pl-9"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>
      </div>

      {/* List */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-3">
          {filteredRewrites.length === 0 && (
            <div className="text-center text-muted-foreground py-8">
              {searchTerm ? 'No rewrites match your search' : 'No rewrite rules configured. Create one to modify traffic.'}
            </div>
          )}

          {filteredRewrites.map((rewrite, index) => (
            <div
              key={rewrite.id}
              className="flex items-center justify-between p-3 border rounded-lg hover:bg-muted/50 transition-colors"
            >
              <div className="flex items-center gap-2 flex-shrink-0 text-muted-foreground">
                <GripVertical className="h-4 w-4" />
                <span className="text-xs">{index + 1}</span>
              </div>
              <div className="flex-1 min-w-0 ml-2">
                <div className="flex items-center gap-2">
                  <span className="font-medium truncate">{rewrite.name}</span>
                  <span className="text-xs px-2 py-0.5 rounded bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300 flex-shrink-0">
                    {rewrite.direction}
                  </span>
                  {rewrite.hit_count > 0 && (
                    <span className="text-xs text-muted-foreground flex-shrink-0">
                      ({rewrite.hit_count} hits)
                    </span>
                  )}
                </div>
                <div className="text-sm text-muted-foreground font-mono truncate">
                  {getActionDescription(rewrite)}
                </div>
              </div>
              <div className="flex items-center gap-2 flex-shrink-0">
                <Switch
                  checked={rewrite.enabled}
                  onCheckedChange={(checked) => handleToggle(rewrite.id, checked)}
                  aria-label={`Toggle ${rewrite.name}`}
                />
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => handleEdit(rewrite)}>
                      Edit
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      className="text-destructive"
                      onClick={() => handleDelete(rewrite.id, rewrite.name)}
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

      {/* Edit Dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>Edit Rewrite Rule</DialogTitle>
            <DialogDescription>
              Modify the rule's match criteria and actions
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <label className="text-sm font-medium">Name</label>
              <Input
                placeholder="My Rewrite"
                value={editRewrite.name}
                onChange={(e) => setEditRewrite({ ...editRewrite, name: e.target.value })}
              />
            </div>
            <div className="grid gap-2">
              <label className="text-sm font-medium">URL Pattern (Regex, optional)</label>
              <Input
                placeholder=".*api/.*"
                value={editRewrite.urlPattern}
                onChange={(e) => setEditRewrite({ ...editRewrite, urlPattern: e.target.value })}
              />
            </div>
            <div className="grid gap-2">
              <label className="text-sm font-medium">Direction</label>
              <Select
                value={editRewrite.direction}
                onValueChange={(v: 'request' | 'response' | 'both') => setEditRewrite({ ...editRewrite, direction: v })}
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
            <div className="grid gap-2">
              <label className="text-sm font-medium">Action Type</label>
              <Select
                value={editRewrite.actionType}
                onValueChange={(v: 'set_header' | 'remove_header' | 'url_rewrite' | 'body_rewrite') => setEditRewrite({ ...editRewrite, actionType: v })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="set_header">Set Header</SelectItem>
                  <SelectItem value="remove_header">Remove Header</SelectItem>
                  <SelectItem value="url_rewrite">URL Rewrite</SelectItem>
                  <SelectItem value="body_rewrite">Body Rewrite</SelectItem>
                </SelectContent>
              </Select>
            </div>
            {(editRewrite.actionType === 'set_header' || editRewrite.actionType === 'remove_header') && (
              <div className="grid gap-2">
                <label className="text-sm font-medium">Header Name</label>
                <Input
                  placeholder="Authorization"
                  value={editRewrite.headerName}
                  onChange={(e) => setEditRewrite({ ...editRewrite, headerName: e.target.value })}
                />
              </div>
            )}
            {(editRewrite.actionType === 'set_header' || editRewrite.actionType === 'url_rewrite' || editRewrite.actionType === 'body_rewrite') && (
              <div className="grid gap-2">
                <label className="text-sm font-medium">
                  {editRewrite.actionType === 'set_header' ? 'Header Value' : 'Replacement'}
                </label>
                <Input
                  placeholder={editRewrite.actionType === 'set_header' ? 'Bearer token123' : 'replacement text'}
                  value={editRewrite.headerValue}
                  onChange={(e) => setEditRewrite({ ...editRewrite, headerValue: e.target.value })}
                />
              </div>
            )}
            {(editRewrite.actionType === 'url_rewrite' || editRewrite.actionType === 'body_rewrite') && (
              <div className="grid gap-2">
                <label className="text-sm font-medium">Pattern (Regex)</label>
                <Input
                  placeholder="http://localhost"
                  value={editRewrite.headerName}
                  onChange={(e) => setEditRewrite({ ...editRewrite, headerName: e.target.value })}
                />
              </div>
            )}
            <div className="flex items-center gap-2">
              <Switch
                checked={editRewrite.enabled}
                onCheckedChange={(checked) => setEditRewrite({ ...editRewrite, enabled: checked })}
                id="edit-enabled"
              />
              <label htmlFor="edit-enabled" className="text-sm font-medium">
                Enabled
              </label>
            </div>
            {editingRewrite && editingRewrite.rewrites.length > 1 && (
              <p className="text-xs text-muted-foreground">
                This rule has {editingRewrite.rewrites.length} actions. Editing the
                form above updates the first action and preserves the remaining{' '}
                {editingRewrite.rewrites.length - 1}.
              </p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowEditDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleEditSubmit} disabled={updateRewrite.isPending}>
              Save Changes
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
