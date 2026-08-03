import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { useInstallPlugin } from '@/lib/api/tools';
import { Download, Loader2, Link as LinkIcon, Package } from 'lucide-react';

interface PluginInstallDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function PluginInstallDialog({ open, onOpenChange }: PluginInstallDialogProps) {
  const [mode, setMode] = useState<'url' | 'registry'>('url');
  const [url, setUrl] = useState('');
  const [registryId, setRegistryId] = useState('');
  const [checksum, setChecksum] = useState('');

  const installPlugin = useInstallPlugin();

  const handleInstall = () => {
    if (mode === 'url' && !url) return;
    if (mode === 'registry' && !registryId) return;

    installPlugin.mutate(
      {
        source: mode,
        target: mode === 'url' ? url : registryId,
        checksum: mode === 'url' && checksum ? checksum : undefined,
      },
      {
        onSuccess: () => {
          onOpenChange(false);
          setUrl('');
          setRegistryId('');
          setChecksum('');
        },
      }
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="text-sm">Install Plugin</DialogTitle>
        </DialogHeader>

        <div className="space-y-3">
          {/* Mode selector */}
          <div className="flex gap-2">
            <Button
              variant={mode === 'url' ? 'default' : 'outline'}
              size="sm"
              className="h-7 text-xs flex-1"
              onClick={() => setMode('url')}
            >
              <LinkIcon className="w-3 h-3 mr-1" /> From URL
            </Button>
            <Button
              variant={mode === 'registry' ? 'default' : 'outline'}
              size="sm"
              className="h-7 text-xs flex-1"
              onClick={() => setMode('registry')}
            >
              <Package className="w-3 h-3 mr-1" /> From Registry
            </Button>
          </div>

          {mode === 'url' ? (
            <>
              <div className="space-y-1">
                <label className="text-xs font-medium">Plugin URL (.zip)</label>
                <Input
                  placeholder="https://example.com/plugin.zip"
                  className="h-8 text-xs"
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                />
              </div>
              <div className="space-y-1">
                <label className="text-xs font-medium">SHA-256 Checksum (optional)</label>
                <Input
                  placeholder="abc123..."
                  className="h-8 text-xs font-mono"
                  value={checksum}
                  onChange={(e) => setChecksum(e.target.value)}
                />
                <p className="text-[10px] text-muted-foreground">
                  Recommended for untrusted sources.
                </p>
              </div>
            </>
          ) : (
            <div className="space-y-1">
              <label className="text-xs font-medium">Registry Plugin ID</label>
              <Input
                placeholder="com.example.my-plugin"
                className="h-8 text-xs font-mono"
                value={registryId}
                onChange={(e) => setRegistryId(e.target.value)}
              />
              <p className="text-[10px] text-muted-foreground">
                Use the Registry tab to browse available plugins.
              </p>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            size="sm"
            onClick={handleInstall}
            disabled={installPlugin.isPending || (mode === 'url' ? !url : !registryId)}
          >
            {installPlugin.isPending ? (
              <Loader2 className="w-3 h-3 mr-1 animate-spin" />
            ) : (
              <Download className="w-3 h-3 mr-1" />
            )}
            Install
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
