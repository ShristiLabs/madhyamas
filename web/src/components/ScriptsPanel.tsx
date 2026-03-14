import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import {
  useScripts,
  useScriptTemplates,
  useCreateScript,
  useUpdateScript,
  useDeleteScript,
  useToggleScript
} from '@/lib/api/phase3';
import type { Script, ScriptTemplate } from '@/lib/api/phase3';
import { Code, Trash2, Plus, Copy, FileCode } from 'lucide-react';

export function ScriptsPanel() {
  const [activeTab, setActiveTab] = useState('scripts');
  const [search, setSearch] = useState('');
  const [selectedScript, setSelectedScript] = useState<Script | null>(null);
  const [editingSource, setEditingSource] = useState('');

  const { data: scripts = [] } = useScripts();
  const { data: templates = [] } = useScriptTemplates();
  const createScript = useCreateScript();
  const updateScript = useUpdateScript();
  const deleteScript = useDeleteScript();
  const toggleScript = useToggleScript();

  const filteredScripts = scripts.filter((s) => {
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

  const handleSave = () => {
    if (selectedScript) {
      updateScript.mutate({
        id: selectedScript.id,
        source: editingSource,
      }, {
        onSuccess: () => {
          setSelectedScript(null);
          setEditingSource('');
        },
      });
    }
  };

  const tabs = [
    { value: 'scripts', label: 'Scripts', icon: <FileCode className="w-4 h-4" /> },
    { value: 'templates', label: 'Templates', icon: <Copy className="w-4 h-4" /> },
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
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
        <TabsList className="grid grid-cols-2 h-9 p-0 m-2">
          {tabs.map((tab) => (
            <TabsTrigger key={tab.value} value={tab.value} className="text-xs py-1">
              {tab.icon}
            </TabsTrigger>
          ))}
        </TabsList>

        <ScrollArea className="flex-1">
          <TabsContent value="scripts" className="m-0 p-2">
            {selectedScript ? (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium">{selectedScript.name}</span>
                  <div className="flex gap-1">
                    <Button size="sm" className="h-6 text-xs" onClick={handleSave}>
                      Save
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-6 text-xs"
                      onClick={() => {
                        setSelectedScript(null);
                        setEditingSource('');
                      }}
                    >
                      Cancel
                    </Button>
                  </div>
                </div>
                <textarea
                  className="w-full h-48 p-2 text-xs font-mono bg-muted rounded border resize-none"
                  value={editingSource}
                  onChange={(e) => setEditingSource(e.target.value)}
                />
              </div>
            ) : filteredScripts.length === 0 ? (
              <div className="text-xs text-muted-foreground text-center py-4">
                No scripts. Use templates to create one.
              </div>
            ) : (
              <div className="space-y-1">
                {filteredScripts.map((script) => (
                  <ScriptItem
                    key={script.id}
                    script={script}
                    onToggle={(id, enabled) => toggleScript.mutate({ id, enabled })}
                    onDelete={(id) => deleteScript.mutate(id)}
                    onEdit={() => {
                      setSelectedScript(script);
                      setEditingSource(script.source);
                    }}
                  />
                ))}
              </div>
            )}
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
        </ScrollArea>
      </Tabs>
    </div>
  );
}

interface ScriptItemProps {
  script: Script;
  onToggle: (id: string, enabled: boolean) => void;
  onDelete: (id: string) => void;
  onEdit: () => void;
}

function ScriptItem({ script, onToggle, onDelete, onEdit }: ScriptItemProps) {
  return (
    <div className="border rounded p-2 text-xs">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Switch
            checked={script.enabled}
            onCheckedChange={(checked) => onToggle(script.id, checked)}
            className="scale-75"
          />
          <span className="font-medium">{script.name}</span>
        </div>
        <div className="flex gap-1">
          <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onEdit}>
            <Code className="w-3 h-3" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0 text-destructive"
            onClick={() => onDelete(script.id)}
          >
            <Trash2 className="w-3 h-3" />
          </Button>
        </div>
      </div>
      {script.description && (
        <p className="text-muted-foreground mt-1 text-[10px]">{script.description}</p>
      )}
      <div className="flex gap-1 mt-1 flex-wrap">
        {script.hooks.map((hook) => (
          <span key={hook} className="px-1 py-0.5 bg-muted rounded text-[10px]">
            {hook}
          </span>
        ))}
      </div>
    </div>
  );
}

interface TemplateItemProps {
  template: ScriptTemplate;
  onCreate: (template: ScriptTemplate) => void;
}

function TemplateItem({ template, onCreate }: TemplateItemProps) {
  return (
    <div className="border rounded p-2 text-xs">
      <div className="flex items-center justify-between">
        <span className="font-medium">{template.name}</span>
        <Button
          size="sm"
          className="h-6 text-xs"
          onClick={() => onCreate(template)}
        >
          <Plus className="w-3 h-3 mr-1" />
          Use
        </Button>
      </div>
      <p className="text-muted-foreground mt-1 text-[10px]">{template.description}</p>
      <div className="flex gap-1 mt-1 flex-wrap">
        {template.hooks.map((hook) => (
          <span key={hook} className="px-1 py-0.5 bg-muted rounded text-[10px]">
            {hook}
          </span>
        ))}
      </div>
    </div>
  );
}
