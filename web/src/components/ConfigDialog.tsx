import { useState, useEffect, useCallback } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Switch } from "./ui/switch";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Slider } from "./ui/slider";
import { useToast } from "./ui/use-toast";
import { Save, RotateCcw, Server, Network, Camera, Palette } from "lucide-react";

// ─── Types ────────────────────────────────────────────────────────────────────

interface RuntimeConfig {
  proxy_port: number;
  api_port: number;
  host: string;
  public_ip: string;
  intercept_https: boolean;
  max_requests: number;
  verbose: boolean;
}

interface UpstreamConfig {
  enabled: boolean;
  protocol: "http" | "https" | "socks5";
  host: string;
  port: string;
  auth_enabled: boolean;
  username: string;
  password: string;
  no_proxy: string;
}

interface CaptureConfig {
  capture_request_bodies: boolean;
  capture_response_bodies: boolean;
  max_body_size_kb: number;
  ignored_domains: string;
}

interface AppearanceConfig {
  theme: "light" | "dark" | "system";
  auto_refresh_interval: string;
}

const LS_UPSTREAM = "proxyforge-upstream-config";
const LS_CAPTURE = "proxyforge-capture-config";
const LS_APPEARANCE = "proxyforge-appearance-config";

// ─── Default values ────────────────────────────────────────────────────────────

const DEFAULT_UPSTREAM: UpstreamConfig = {
  enabled: false,
  protocol: "http",
  host: "",
  port: "",
  auth_enabled: false,
  username: "",
  password: "",
  no_proxy: "",
};

const DEFAULT_CAPTURE: CaptureConfig = {
  capture_request_bodies: true,
  capture_response_bodies: true,
  max_body_size_kb: 512,
  ignored_domains: "",
};

const DEFAULT_APPEARANCE: AppearanceConfig = {
  theme: "system",
  auto_refresh_interval: "2000",
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function loadLS<T>(key: string, defaults: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw) return { ...defaults, ...JSON.parse(raw) };
  } catch { /* ignore */ }
  return defaults;
}

// ─── Section wrapper ──────────────────────────────────────────────────────────

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-4">
      <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide border-b pb-1">
        {title}
      </h3>
      {children}
    </div>
  );
}

function Row({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex-1 min-w-0">
        <Label className="text-sm font-medium">{label}</Label>
        {description && (
          <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

// ─── Tabs ─────────────────────────────────────────────────────────────────────

function GeneralTab() {
  const { toast } = useToast();
  const [config, setConfig] = useState<RuntimeConfig>({
    proxy_port: 8888,
    api_port: 3000,
    host: "127.0.0.1",
    public_ip: "",
    intercept_https: true,
    max_requests: 10000,
    verbose: false,
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetch("/api/config")
      .then((r) => r.json())
      .then((data) => {
        setConfig({
          proxy_port: data.proxy_port ?? 8888,
          api_port: data.api_port ?? 3000,
          host: data.host ?? "127.0.0.1",
          public_ip: data.public_ip ?? "",
          intercept_https: data.intercept_https ?? true,
          max_requests: data.max_requests ?? 10000,
          verbose: data.verbose ?? false,
        });
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      const res = await fetch("/api/config", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          intercept_https: config.intercept_https,
          max_requests: config.max_requests,
          verbose: config.verbose,
          public_ip: config.public_ip || null,
        }),
      });
      if (res.ok) {
        toast({ description: "Configuration saved successfully." });
      } else {
        toast({ description: "Failed to save configuration.", variant: "destructive" });
      }
    } catch {
      toast({ description: "Failed to save configuration.", variant: "destructive" });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <div className="flex items-center justify-center h-48 text-muted-foreground">Loading…</div>;
  }

  return (
    <div className="space-y-6">
      <Section title="Proxy Address">
        <Row label="Proxy Port" description="Port clients connect to. Requires restart to change.">
          <Input value={config.proxy_port} disabled className="w-24 text-right opacity-70" />
        </Row>
        <Row label="API Port" description="Port for the web UI and API. Requires restart to change.">
          <Input value={config.api_port} disabled className="w-24 text-right opacity-70" />
        </Row>
        <Row label="Bind Host" description="Network interface the proxy is bound to. Requires restart.">
          <Input value={config.host} disabled className="w-36 opacity-70" />
        </Row>
        <Row
          label="Public IP Override"
          description="Display this IP to users instead of auto-detected. Useful for remote/NAT scenarios."
        >
          <Input
            value={config.public_ip}
            onChange={(e) => setConfig((p) => ({ ...p, public_ip: e.target.value }))}
            placeholder="e.g. 192.168.1.100"
            className="w-40"
          />
        </Row>
      </Section>

      <Section title="Traffic">
        <Row
          label="HTTPS Interception"
          description="Intercept and inspect HTTPS traffic. Requires CA certificate installed on clients."
        >
          <Switch
            checked={config.intercept_https}
            onCheckedChange={(v) => setConfig((p) => ({ ...p, intercept_https: v }))}
          />
        </Row>
        <Row
          label="Max Traffic Entries"
          description="Maximum number of requests to keep in the database."
        >
          <Input
            type="number"
            value={config.max_requests}
            onChange={(e) =>
              setConfig((p) => ({ ...p, max_requests: parseInt(e.target.value) || 1000 }))
            }
            min={100}
            max={100000}
            step={100}
            className="w-28 text-right"
          />
        </Row>
      </Section>

      <Section title="Logging">
        <Row
          label="Verbose Logging"
          description="Enable detailed proxy engine logging. May impact performance."
        >
          <Switch
            checked={config.verbose}
            onCheckedChange={(v) => setConfig((p) => ({ ...p, verbose: v }))}
          />
        </Row>
      </Section>

      <div className="flex justify-end pt-2">
        <Button onClick={handleSave} disabled={saving}>
          <Save className="h-4 w-4 mr-2" />
          {saving ? "Saving…" : "Save Changes"}
        </Button>
      </div>
    </div>
  );
}

function UpstreamProxyTab() {
  const { toast } = useToast();
  const [cfg, setCfg] = useState<UpstreamConfig>(() => loadLS(LS_UPSTREAM, DEFAULT_UPSTREAM));

  const save = useCallback(() => {
    localStorage.setItem(LS_UPSTREAM, JSON.stringify(cfg));
    toast({ description: "Upstream proxy settings saved." });
  }, [cfg, toast]);

  const reset = useCallback(() => {
    setCfg(DEFAULT_UPSTREAM);
    localStorage.removeItem(LS_UPSTREAM);
    toast({ description: "Upstream proxy settings reset." });
  }, [toast]);

  return (
    <div className="space-y-6">
      <Section title="Proxy Chaining">
        <Row
          label="Enable Upstream Proxy"
          description="Forward all traffic through an upstream proxy server."
        >
          <Switch
            checked={cfg.enabled}
            onCheckedChange={(v) => setCfg((p) => ({ ...p, enabled: v }))}
          />
        </Row>
      </Section>

      {cfg.enabled && (
        <>
          <Section title="Server">
            <Row label="Protocol">
              <Select
                value={cfg.protocol}
                onValueChange={(v: "http" | "https" | "socks5") =>
                  setCfg((p) => ({ ...p, protocol: v }))
                }
              >
                <SelectTrigger className="w-28">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="http">HTTP</SelectItem>
                  <SelectItem value="https">HTTPS</SelectItem>
                  <SelectItem value="socks5">SOCKS5</SelectItem>
                </SelectContent>
              </Select>
            </Row>
            <Row label="Host">
              <Input
                value={cfg.host}
                onChange={(e) => setCfg((p) => ({ ...p, host: e.target.value }))}
                placeholder="proxy.example.com"
                className="w-48"
              />
            </Row>
            <Row label="Port">
              <Input
                type="number"
                value={cfg.port}
                onChange={(e) => setCfg((p) => ({ ...p, port: e.target.value }))}
                placeholder="8080"
                className="w-24 text-right"
              />
            </Row>
          </Section>

          <Section title="Authentication">
            <Row label="Require Authentication">
              <Switch
                checked={cfg.auth_enabled}
                onCheckedChange={(v) => setCfg((p) => ({ ...p, auth_enabled: v }))}
              />
            </Row>
            {cfg.auth_enabled && (
              <>
                <Row label="Username">
                  <Input
                    value={cfg.username}
                    onChange={(e) => setCfg((p) => ({ ...p, username: e.target.value }))}
                    className="w-40"
                  />
                </Row>
                <Row label="Password">
                  <Input
                    type="password"
                    value={cfg.password}
                    onChange={(e) => setCfg((p) => ({ ...p, password: e.target.value }))}
                    className="w-40"
                  />
                </Row>
              </>
            )}
          </Section>

          <Section title="Bypass">
            <div className="space-y-2">
              <Label className="text-sm font-medium">No-Proxy Hosts</Label>
              <p className="text-xs text-muted-foreground">
                Comma-separated list of hosts to bypass the upstream proxy (e.g. localhost,
                192.168.0.0/16).
              </p>
              <textarea
                value={cfg.no_proxy}
                onChange={(e) => setCfg((p) => ({ ...p, no_proxy: e.target.value }))}
                rows={3}
                placeholder="localhost, 127.0.0.1, *.internal.corp"
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm resize-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
          </Section>
        </>
      )}

      <div className="flex justify-between pt-2">
        <Button variant="ghost" onClick={reset}>
          <RotateCcw className="h-4 w-4 mr-2" />
          Reset
        </Button>
        <Button onClick={save}>
          <Save className="h-4 w-4 mr-2" />
          Save Changes
        </Button>
      </div>
    </div>
  );
}

function CaptureTab() {
  const { toast } = useToast();
  const [cfg, setCfg] = useState<CaptureConfig>(() => loadLS(LS_CAPTURE, DEFAULT_CAPTURE));

  const save = useCallback(() => {
    localStorage.setItem(LS_CAPTURE, JSON.stringify(cfg));
    toast({ description: "Capture settings saved." });
  }, [cfg, toast]);

  const reset = useCallback(() => {
    setCfg(DEFAULT_CAPTURE);
    localStorage.removeItem(LS_CAPTURE);
    toast({ description: "Capture settings reset." });
  }, [toast]);

  return (
    <div className="space-y-6">
      <Section title="Body Recording">
        <Row
          label="Capture Request Bodies"
          description="Store request body content for inspection. Disable to reduce storage usage."
        >
          <Switch
            checked={cfg.capture_request_bodies}
            onCheckedChange={(v) => setCfg((p) => ({ ...p, capture_request_bodies: v }))}
          />
        </Row>
        <Row
          label="Capture Response Bodies"
          description="Store response body content for inspection."
        >
          <Switch
            checked={cfg.capture_response_bodies}
            onCheckedChange={(v) => setCfg((p) => ({ ...p, capture_response_bodies: v }))}
          />
        </Row>
        <div className="space-y-3">
          <Row
            label="Max Body Size"
            description={`Bodies larger than ${cfg.max_body_size_kb} KB will be truncated.`}
          >
            <span className="text-sm font-mono w-20 text-right inline-block">
              {cfg.max_body_size_kb} KB
            </span>
          </Row>
          <Slider
            min={16}
            max={4096}
            step={16}
            value={[cfg.max_body_size_kb]}
            onValueChange={([v]) => setCfg((p) => ({ ...p, max_body_size_kb: v }))}
            className="w-full"
          />
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>16 KB</span>
            <span>4096 KB</span>
          </div>
        </div>
      </Section>

      <Section title="Domain Filtering">
        <div className="space-y-2">
          <Label className="text-sm font-medium">Ignored Domains</Label>
          <p className="text-xs text-muted-foreground">
            Traffic from these domains will not be recorded. One pattern per line.
            Supports wildcards (e.g. *.googleapis.com).
          </p>
          <textarea
            value={cfg.ignored_domains}
            onChange={(e) => setCfg((p) => ({ ...p, ignored_domains: e.target.value }))}
            rows={5}
            placeholder={"*.google-analytics.com\n*.doubleclick.net\ntelemetry.example.com"}
            className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono resize-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
        </div>
      </Section>

      <div className="flex justify-between pt-2">
        <Button variant="ghost" onClick={reset}>
          <RotateCcw className="h-4 w-4 mr-2" />
          Reset
        </Button>
        <Button onClick={save}>
          <Save className="h-4 w-4 mr-2" />
          Save Changes
        </Button>
      </div>
    </div>
  );
}

function AppearanceTab() {
  const { toast } = useToast();
  const [cfg, setCfg] = useState<AppearanceConfig>(() =>
    loadLS(LS_APPEARANCE, DEFAULT_APPEARANCE)
  );

  const save = useCallback(() => {
    localStorage.setItem(LS_APPEARANCE, JSON.stringify(cfg));

    // Apply theme immediately
    const root = document.documentElement;
    if (cfg.theme === "dark") {
      root.classList.add("dark");
    } else if (cfg.theme === "light") {
      root.classList.remove("dark");
    } else {
      const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      prefersDark ? root.classList.add("dark") : root.classList.remove("dark");
    }

    // Emit custom event so App.tsx can sync its isDark state
    window.dispatchEvent(new CustomEvent("proxyforge-theme-change", { detail: cfg.theme }));

    toast({ description: "Appearance settings saved." });
  }, [cfg, toast]);

  const reset = useCallback(() => {
    setCfg(DEFAULT_APPEARANCE);
    localStorage.removeItem(LS_APPEARANCE);
    toast({ description: "Appearance settings reset." });
  }, [toast]);

  return (
    <div className="space-y-6">
      <Section title="Theme">
        <Row label="Color Theme" description="Choose how ProxyForge looks.">
          <Select
            value={cfg.theme}
            onValueChange={(v: "light" | "dark" | "system") =>
              setCfg((p) => ({ ...p, theme: v }))
            }
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="light">Light</SelectItem>
              <SelectItem value="dark">Dark</SelectItem>
              <SelectItem value="system">System</SelectItem>
            </SelectContent>
          </Select>
        </Row>
      </Section>

      <Section title="Traffic View">
        <Row
          label="Auto-Refresh Interval"
          description="How often the traffic list automatically refreshes."
        >
          <Select
            value={cfg.auto_refresh_interval}
            onValueChange={(v) => setCfg((p) => ({ ...p, auto_refresh_interval: v }))}
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="0">Off</SelectItem>
              <SelectItem value="1000">1 second</SelectItem>
              <SelectItem value="2000">2 seconds</SelectItem>
              <SelectItem value="5000">5 seconds</SelectItem>
              <SelectItem value="10000">10 seconds</SelectItem>
              <SelectItem value="30000">30 seconds</SelectItem>
            </SelectContent>
          </Select>
        </Row>
      </Section>

      <div className="flex justify-between pt-2">
        <Button variant="ghost" onClick={reset}>
          <RotateCcw className="h-4 w-4 mr-2" />
          Reset
        </Button>
        <Button onClick={save}>
          <Save className="h-4 w-4 mr-2" />
          Save Changes
        </Button>
      </div>
    </div>
  );
}

// ─── Main dialog ───────────────────────────────────────────────────────────────

interface ConfigDialogProps {
  trigger?: React.ReactNode;
}

export function ConfigDialog({ trigger }: ConfigDialogProps) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        {trigger ?? <Button variant="ghost" size="sm">Config</Button>}
      </DialogTrigger>
      <DialogContent className="max-w-2xl max-h-[85vh] flex flex-col p-0">
        <DialogHeader className="px-6 pt-6 pb-0">
          <DialogTitle className="text-lg">Proxy Configuration</DialogTitle>
        </DialogHeader>
        <Tabs defaultValue="general" className="flex-1 flex flex-col min-h-0 mt-4">
          <TabsList className="mx-6 justify-start shrink-0">
            <TabsTrigger value="general" className="flex items-center gap-1.5">
              <Server className="h-3.5 w-3.5" />
              General
            </TabsTrigger>
            <TabsTrigger value="upstream" className="flex items-center gap-1.5">
              <Network className="h-3.5 w-3.5" />
              Upstream Proxy
            </TabsTrigger>
            <TabsTrigger value="capture" className="flex items-center gap-1.5">
              <Camera className="h-3.5 w-3.5" />
              Capture
            </TabsTrigger>
            <TabsTrigger value="appearance" className="flex items-center gap-1.5">
              <Palette className="h-3.5 w-3.5" />
              Appearance
            </TabsTrigger>
          </TabsList>
          <div className="flex-1 overflow-y-auto px-6 py-5">
            <TabsContent value="general" className="mt-0">
              <GeneralTab />
            </TabsContent>
            <TabsContent value="upstream" className="mt-0">
              <UpstreamProxyTab />
            </TabsContent>
            <TabsContent value="capture" className="mt-0">
              <CaptureTab />
            </TabsContent>
            <TabsContent value="appearance" className="mt-0">
              <AppearanceTab />
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
