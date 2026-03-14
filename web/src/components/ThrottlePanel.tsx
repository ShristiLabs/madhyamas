import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Slider } from '@/components/ui/slider';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  useThrottle,
  useSetThrottle,
  useThrottlePresets,
  type ThrottleProfile,
} from '@/lib/api/intercept';
import { useToast } from '@/components/ui/use-toast';

export function ThrottlePanel() {
  const { data: throttle, isLoading } = useThrottle();
  const { data: presets } = useThrottlePresets();
  const setThrottle = useSetThrottle();
  const { toast } = useToast();

  const [enabled, setEnabled] = useState(false);
  const [profile, setProfile] = useState<ThrottleProfile>({
    name: 'Custom',
    download_bps: 0,
    upload_bps: 0,
    latency_ms: 0,
    jitter_ms: 0,
    packet_loss_percent: 0,
  });

  useEffect(() => {
    if (throttle) {
      setEnabled(throttle.enabled);
      setProfile(throttle.profile);
    }
  }, [throttle]);

  const handlePresetChange = (presetName: string) => {
    const preset = presets?.find((p) => p.name === presetName);
    if (preset) {
      setProfile(preset);
    }
  };

  const handleSave = async () => {
    await setThrottle.mutateAsync({ profile, enabled });
    toast({
      title: 'Throttle Settings Saved',
      description: enabled
        ? `Throttling enabled with ${profile.name} profile`
        : 'Throttling disabled',
    });
  };

  const formatBps = (bps: number): string => {
    if (bps === 0) return 'Unlimited';
    if (bps >= 1000000) return `${(bps / 1000000).toFixed(1)} MB/s`;
    if (bps >= 1000) return `${(bps / 1000).toFixed(0)} KB/s`;
    return `${bps} B/s`;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading throttle settings...</div>
      </div>
    );
  }

  return (
    <div className="p-4 space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Network Throttling</h2>
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">Enable</span>
          <Switch checked={enabled} onCheckedChange={setEnabled} />
        </div>
      </div>

      <div className={`space-y-6 ${!enabled ? 'opacity-50' : ''}`}>
        <div className="grid gap-2">
          <label className="text-sm font-medium">Preset Profile</label>
          <Select value={profile.name} onValueChange={handlePresetChange} disabled={!enabled}>
            <SelectTrigger>
              <SelectValue placeholder="Select a profile" />
            </SelectTrigger>
            <SelectContent>
              {presets?.map((p) => (
                <SelectItem key={p.name} value={p.name}>
                  {p.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="grid gap-4">
          <div className="grid gap-2">
            <div className="flex justify-between">
              <label className="text-sm font-medium">Download Speed</label>
              <span className="text-sm text-muted-foreground">{formatBps(profile.download_bps)}</span>
            </div>
            <Slider
              value={[profile.download_bps]}
              onValueChange={([v]) => setProfile({ ...profile, download_bps: v, name: 'Custom' })}
              max={50000000}
              step={100000}
              disabled={!enabled}
            />
          </div>

          <div className="grid gap-2">
            <div className="flex justify-between">
              <label className="text-sm font-medium">Upload Speed</label>
              <span className="text-sm text-muted-foreground">{formatBps(profile.upload_bps)}</span>
            </div>
            <Slider
              value={[profile.upload_bps]}
              onValueChange={([v]) => setProfile({ ...profile, upload_bps: v, name: 'Custom' })}
              max={50000000}
              step={100000}
              disabled={!enabled}
            />
          </div>

          <div className="grid gap-2">
            <div className="flex justify-between">
              <label className="text-sm font-medium">Latency</label>
              <span className="text-sm text-muted-foreground">{profile.latency_ms} ms</span>
            </div>
            <Slider
              value={[profile.latency_ms]}
              onValueChange={([v]) => setProfile({ ...profile, latency_ms: v, name: 'Custom' })}
              max={2000}
              step={10}
              disabled={!enabled}
            />
          </div>

          <div className="grid gap-2">
            <div className="flex justify-between">
              <label className="text-sm font-medium">Jitter</label>
              <span className="text-sm text-muted-foreground">{profile.jitter_ms} ms</span>
            </div>
            <Slider
              value={[profile.jitter_ms]}
              onValueChange={([v]) => setProfile({ ...profile, jitter_ms: v, name: 'Custom' })}
              max={500}
              step={5}
              disabled={!enabled}
            />
          </div>

          <div className="grid gap-2">
            <div className="flex justify-between">
              <label className="text-sm font-medium">Packet Loss</label>
              <span className="text-sm text-muted-foreground">{profile.packet_loss_percent}%</span>
            </div>
            <Slider
              value={[profile.packet_loss_percent]}
              onValueChange={([v]) => setProfile({ ...profile, packet_loss_percent: v, name: 'Custom' })}
              max={100}
              step={1}
              disabled={!enabled}
            />
          </div>
        </div>

        <div className="p-4 bg-muted/50 rounded-lg">
          <h3 className="font-medium mb-2">Current Profile Summary</h3>
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div>Download: <span className="font-mono">{formatBps(profile.download_bps)}</span></div>
            <div>Upload: <span className="font-mono">{formatBps(profile.upload_bps)}</span></div>
            <div>Latency: <span className="font-mono">{profile.latency_ms} ms</span></div>
            <div>Jitter: <span className="font-mono">{profile.jitter_ms} ms</span></div>
            <div>Packet Loss: <span className="font-mono">{profile.packet_loss_percent}%</span></div>
          </div>
        </div>

        <Button onClick={handleSave} disabled={setThrottle.isPending} className="w-full">
          Save Settings
        </Button>
      </div>
    </div>
  );
}
