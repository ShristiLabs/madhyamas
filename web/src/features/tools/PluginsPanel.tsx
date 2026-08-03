import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Input } from '@/components/ui/input';
import {
  usePlugins,
  usePluginStats,
  useEnablePlugin,
  useDisablePlugin,
  useReloadPlugins,
  useUninstallPlugin,
} from '@/lib/api/tools';
import type { Plugin } from '@/lib/api/tools';
import { Power, RefreshCw, Package, Info, Plus, Trash2, Settings, ScrollText, LayoutGrid } from 'lucide-react';
import { PluginInstallDialog } from './PluginInstallDialog';
import { PluginSettingsForm } from './PluginSettingsForm';
import { PluginLogs } from './PluginLogs';
import { PluginRegistryBrowser } from './PluginRegistryBrowser';

type Tab = 'installed' | 'registry';

export function PluginsPanel() {
  const [tab, setTab] = useState<Tab>('installed');
  const [search, setSearch] = useState('');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [installOpen, setInstallOpen] = useState(false);

  const { data: plugins = [], isLoading } = usePlugins();
  const enablePlugin = useEnablePlugin();
  const disablePlugin = useDisablePlugin();
  const reloadPlugins = useReloadPlugins();
  const uninstallPlugin = useUninstallPlugin();

  const filteredPlugins = plugins.filter((p) => {
    if (!search) return true;
    return (
      p.manifest.name.toLowerCase().includes(search.toLowerCase()) ||
      p.manifest.id.toLowerCase().includes(search.toLowerCase())
    );
  });

  const handleToggle = (id: string, enabled: boolean) => {
    if (enabled) {
      disablePlugin.mutate(id);
    } else {
      enablePlugin.mutate(id);
    }
  };

  const handleReload = () => {
    reloadPlugins.mutate();
  };

  const handleUninstall = (id: string) => {
    if (confirm(`Uninstall plugin "${id}"? This cannot be undone.`)) {
      uninstallPlugin.mutate(id);
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Tab selector */}
      <div className="flex items-center gap-1 p-1 border-b">
        <Button
          variant={tab === 'installed' ? 'default' : 'ghost'}
          size="sm"
          className="h-6 text-[10px] flex-1"
          onClick={() => setTab('installed')}
        >
          <Package className="w-3 h-3 mr-1" /> Installed ({plugins.length})
        </Button>
        <Button
          variant={tab === 'registry' ? 'default' : 'ghost'}
          size="sm"
          className="h-6 text-[10px] flex-1"
          onClick={() => setTab('registry')}
        >
          <LayoutGrid className="w-3 h-3 mr-1" /> Registry
        </Button>
      </div>

      {tab === 'installed' ? (
        <>
          <div className="p-2 border-b space-y-2">
            <div className="flex items-center gap-2">
              <Input
                placeholder="Search plugins..."
                className="h-7 text-xs flex-1"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs"
                onClick={() => setInstallOpen(true)}
              >
                <Plus className="w-3 h-3 mr-1" /> Install
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs"
                onClick={handleReload}
                disabled={reloadPlugins.isPending}
              >
                <RefreshCw
                  className={`w-3 h-3 mr-1 ${reloadPlugins.isPending ? 'animate-spin' : ''}`}
                />
                Reload
              </Button>
            </div>
          </div>

          <ScrollArea className="flex-1">
            {isLoading ? (
              <div className="p-4 text-center text-xs text-muted-foreground">Loading...</div>
            ) : filteredPlugins.length === 0 ? (
              <div className="p-4 text-center text-xs text-muted-foreground">
                No plugins found. Use the Registry tab or Install button to add plugins.
              </div>
            ) : (
              <div className="p-2 space-y-1">
                {filteredPlugins.map((plugin) => (
                  <PluginItem
                    key={plugin.manifest.id}
                    plugin={plugin}
                    isExpanded={expandedId === plugin.manifest.id}
                    onToggleExpand={() =>
                      setExpandedId(
                        expandedId === plugin.manifest.id ? null : plugin.manifest.id
                      )
                    }
                    onToggle={() => handleToggle(plugin.manifest.id, plugin.state === 'enabled')}
                    onUninstall={() => handleUninstall(plugin.manifest.id)}
                  />
                ))}
              </div>
            )}
          </ScrollArea>
        </>
      ) : (
        <PluginRegistryBrowser />
      )}

      <PluginInstallDialog open={installOpen} onOpenChange={setInstallOpen} />
    </div>
  );
}

interface PluginItemProps {
  plugin: Plugin;
  isExpanded: boolean;
  onToggleExpand: () => void;
  onToggle: () => void;
  onUninstall: () => void;
}

function PluginItem({ plugin, isExpanded, onToggleExpand, onToggle, onUninstall }: PluginItemProps) {
  const [detailTab, setDetailTab] = useState<'info' | 'settings' | 'logs'>('info');
  const { data: stats } = usePluginStats(isExpanded ? plugin.manifest.id : '');
  const isEnabled = plugin.state === 'enabled';

  return (
    <div className="border rounded text-xs">
      <div
        className="flex items-center justify-between p-2 cursor-pointer hover:bg-muted/50"
        onClick={onToggleExpand}
      >
        <div className="flex items-center gap-2">
          <Package className="w-3 h-3 text-muted-foreground" />
          <span className="font-medium">{plugin.manifest.name}</span>
          <span className="text-muted-foreground text-[10px]">v{plugin.manifest.version}</span>
          <span
            className={`px-1.5 py-0.5 rounded text-[10px] ${
              isEnabled
                ? 'bg-green-100 text-green-700'
                : plugin.state === 'error'
                  ? 'bg-red-100 text-red-700'
                  : 'bg-gray-100 text-gray-600'
            }`}
          >
            {plugin.state}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={(e) => {
              e.stopPropagation();
              onToggle();
            }}
          >
            <Power className={`w-3 h-3 ${isEnabled ? 'text-green-500' : 'text-gray-400'}`} />
          </Button>
        </div>
      </div>
      {isExpanded && (
        <div className="border-t">
          {/* Detail tabs */}
          <div className="flex gap-1 p-1 border-b">
            <Button
              variant={detailTab === 'info' ? 'default' : 'ghost'}
              size="sm"
              className="h-5 text-[10px] px-2"
              onClick={() => setDetailTab('info')}
            >
              <Info className="w-2.5 h-2.5 mr-1" /> Info
            </Button>
            <Button
              variant={detailTab === 'settings' ? 'default' : 'ghost'}
              size="sm"
              className="h-5 text-[10px] px-2"
              onClick={() => setDetailTab('settings')}
            >
              <Settings className="w-2.5 h-2.5 mr-1" /> Settings
            </Button>
            <Button
              variant={detailTab === 'logs' ? 'default' : 'ghost'}
              size="sm"
              className="h-5 text-[10px] px-2"
              onClick={() => setDetailTab('logs')}
            >
              <ScrollText className="w-2.5 h-2.5 mr-1" /> Logs
            </Button>
          </div>

          <div className="p-2">
            {detailTab === 'info' && (
              <div className="space-y-2">
                {plugin.error && (
                  <p className="text-red-500 text-[10px]">{plugin.error}</p>
                )}
                {plugin.manifest.description && (
                  <p className="text-muted-foreground">{plugin.manifest.description}</p>
                )}
                <div className="grid grid-cols-2 gap-2 text-[10px]">
                  <div>
                    <span className="text-muted-foreground">ID:</span>{' '}
                    <span className="font-mono">{plugin.manifest.id}</span>
                  </div>
                  {plugin.manifest.author && (
                    <div>
                      <span className="text-muted-foreground">Author:</span>{' '}
                      {plugin.manifest.author}
                    </div>
                  )}
                </div>
                <div className="flex gap-1 flex-wrap">
                  {plugin.manifest.hooks.map((hook) => (
                    <span
                      key={hook}
                      className="px-1.5 py-0.5 bg-blue-100 text-blue-700 rounded text-[10px]"
                    >
                      {hook}
                    </span>
                  ))}
                  {plugin.manifest.capabilities.map((cap) => (
                    <span
                      key={cap}
                      className="px-1.5 py-0.5 bg-purple-100 text-purple-700 rounded text-[10px]"
                    >
                      {cap}
                    </span>
                  ))}
                </div>
                {stats && (
                  <div className="border-t pt-2 mt-2">
                    <div className="text-muted-foreground mb-1 flex items-center gap-1">
                      <Info className="w-3 h-3" /> Stats
                    </div>
                    <div className="grid grid-cols-2 gap-1 text-[10px]">
                      <div>Invocations: {stats.requests_processed}</div>
                      <div>Errors: {stats.errors}</div>
                      <div>Avg Duration: {stats.avg_duration_ms}ms</div>
                      {stats.responses_modified != null && (
                        <div>Modified: {stats.responses_modified}</div>
                      )}
                    </div>
                  </div>
                )}
                <div className="border-t pt-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 text-[10px] text-red-500 hover:text-red-600"
                    onClick={onUninstall}
                  >
                    <Trash2 className="w-3 h-3 mr-1" /> Uninstall
                  </Button>
                </div>
              </div>
            )}
            {detailTab === 'settings' && <PluginSettingsForm pluginId={plugin.manifest.id} />}
            {detailTab === 'logs' && <PluginLogs pluginId={plugin.manifest.id} />}
          </div>
        </div>
      )}
    </div>
  );
}
