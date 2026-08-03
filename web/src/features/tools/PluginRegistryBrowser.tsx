import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  usePluginRegistry,
  useSearchPluginRegistry,
  useInstallPlugin,
  useRefreshRegistry,
} from '@/lib/api/tools';
import type { RegistryEntry } from '@/lib/api/tools';
import { Search, Download, Loader2, Star, Package, RefreshCw, HardDrive, Globe } from 'lucide-react';

export function PluginRegistryBrowser() {
  const [query, setQuery] = useState('');
  const [installing, setInstalling] = useState<string | null>(null);

  const { data: allEntries = [], isLoading: loadingAll } = usePluginRegistry();
  const { data: searchResults, isLoading: loadingSearch } = useSearchPluginRegistry(query);
  const installPlugin = useInstallPlugin();
  const refreshRegistry = useRefreshRegistry();

  const entries = query.length > 0 ? searchResults ?? [] : allEntries;
  const isLoading = query.length > 0 ? loadingSearch : loadingAll;

  const handleInstall = (entry: RegistryEntry) => {
    setInstalling(entry.manifest.id);
    installPlugin.mutate(
      { source: 'registry', target: entry.manifest.id },
      { onSettled: () => setInstalling(null) }
    );
  };

  return (
    <div className="h-full flex flex-col">
      <div className="p-2 border-b space-y-2">
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-3 h-3 text-muted-foreground" />
            <Input
              placeholder="Search registry..."
              className="h-7 text-xs pl-7"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <Button
            variant="outline"
            size="sm"
            className="h-7 text-xs"
            onClick={() => refreshRegistry.mutate()}
            disabled={refreshRegistry.isPending}
          >
            <RefreshCw className={`w-3 h-3 mr-1 ${refreshRegistry.isPending ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
        </div>
      </div>

      <ScrollArea className="flex-1">
        {isLoading ? (
          <div className="flex items-center justify-center p-4 text-xs text-muted-foreground">
            <Loader2 className="w-3 h-3 mr-1 animate-spin" /> Loading...
          </div>
        ) : entries.length === 0 ? (
          <div className="p-4 text-center text-xs text-muted-foreground">
            {query ? 'No plugins found.' : 'No plugins available. The registry repo may not have a registry.json yet.'}
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {entries.map((entry) => (
              <RegistryEntryItem
                key={entry.manifest.id}
                entry={entry}
                isInstalling={installing === entry.manifest.id}
                onInstall={() => handleInstall(entry)}
              />
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

interface RegistryEntryItemProps {
  entry: RegistryEntry;
  isInstalling: boolean;
  onInstall: () => void;
}

function RegistryEntryItem({ entry, isInstalling, onInstall }: RegistryEntryItemProps) {
  const isLocal = entry.source === 'local' || !entry.download_url;

  return (
    <div className="border rounded text-xs p-2 space-y-1">
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2 flex-1">
          <Package className="w-3 h-3 text-muted-foreground flex-shrink-0" />
          <div className="flex-1 min-w-0">
            <div className="font-medium truncate">{entry.manifest.name}</div>
            <div className="text-[10px] text-muted-foreground font-mono truncate">
              {entry.manifest.id}
            </div>
          </div>
        </div>
        {isLocal ? (
          <span className="flex items-center gap-0.5 px-1.5 py-0.5 bg-gray-100 text-gray-600 rounded text-[10px] flex-shrink-0">
            <HardDrive className="w-2.5 h-2.5" /> Installed
          </span>
        ) : (
          <Button
            variant="outline"
            size="sm"
            className="h-6 text-[10px] flex-shrink-0"
            onClick={onInstall}
            disabled={isInstalling}
          >
            {isInstalling ? (
              <Loader2 className="w-3 h-3 mr-1 animate-spin" />
            ) : (
              <Download className="w-3 h-3 mr-1" />
            )}
            Install
          </Button>
        )}
      </div>
      {entry.manifest.description && (
        <p className="text-muted-foreground text-[10px]">{entry.manifest.description}</p>
      )}
      <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
        <span>v{entry.manifest.version}</span>
        {!isLocal && entry.rating > 0 && (
          <span className="flex items-center gap-0.5">
            <Star className="w-2.5 h-2.5 fill-yellow-400 text-yellow-400" />
            {entry.rating.toFixed(1)} ({entry.rating_count})
          </span>
        )}
        {!isLocal && <span>{entry.downloads} downloads</span>}
        {!isLocal && (
          <span className="flex items-center gap-0.5">
            <Globe className="w-2.5 h-2.5" /> Remote
          </span>
        )}
      </div>
      {entry.tags.length > 0 && (
        <div className="flex gap-1 flex-wrap">
          {entry.tags.map((tag) => (
            <span
              key={tag}
              className="px-1.5 py-0.5 bg-blue-100 text-blue-700 rounded text-[10px]"
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
