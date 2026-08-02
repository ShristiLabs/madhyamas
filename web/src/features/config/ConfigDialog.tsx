import { useState, useEffect, useCallback } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { useToast } from "@/components/ui/use-toast";
import {
  Save,
  RotateCcw,
  Server,
  Network,
  Camera,
  Palette,
  ShieldOff,
  Plus,
  X,
} from "lucide-react";
import { apiGet, apiPatch, ApiError } from "@/lib/api/client";

// ─── Types ────────────────────────────────────────────────────────────────────

interface RuntimeConfig {
  proxy_port: number;
  api_port: number;
  host: string;
  public_ip: string;
  intercept_https: boolean;
  max_requests: number;
  verbose: boolean;
  passthrough_domains?: string[];
  enable_h2_downstream?: boolean;
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

// Shape returned by GET /api/config → upstream_proxy
interface UpstreamProxyApiConfig {
  enabled: boolean;
  protocol: string;
  host: string;
  port: number;
  auth_enabled: boolean;
  auth_username: string | null;
  no_proxy_hosts: string[];
}

interface CaptureConfig {
  capture_request_bodies: boolean;
  capture_response_bodies: boolean;
  max_body_size_kb: number;
  ignored_domains: string;
  max_total_size_mb: number | null;
}

interface AppearanceConfig {
  theme: "light" | "dark" | "system";
  auto_refresh_interval: string;
  use_websocket: string;
}

const LS_UPSTREAM = "madhyamas-upstream-config";
const LS_APPEARANCE = "madhyamas-appearance-config";

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
  max_body_size_kb: 20480,
  ignored_domains: "",
  max_total_size_mb: null,
};

const DEFAULT_APPEARANCE: AppearanceConfig = {
  theme: "system",
  auto_refresh_interval: "2000",
  use_websocket: "true",
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function loadLS<T>(key: string, defaults: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw) return { ...defaults, ...JSON.parse(raw) };
  } catch {
    /* ignore */
  }
  return defaults;
}

// ─── Section wrapper ──────────────────────────────────────────────────────────

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
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
    api_port: 3001,
    host: "127.0.0.1",
    public_ip: "",
    intercept_https: true,
    max_requests: 10000,
    verbose: false,
    passthrough_domains: [],
    enable_h2_downstream: false,
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    apiGet<RuntimeConfig>("/config")
      .then((data) => {
        setConfig({
          proxy_port: data.proxy_port ?? 8888,
          api_port: data.api_port ?? 3001,
          host: data.host ?? "127.0.0.1",
          public_ip: data.public_ip ?? "",
          intercept_https: data.intercept_https ?? true,
          max_requests: data.max_requests ?? 10000,
          verbose: data.verbose ?? false,
          passthrough_domains: data.passthrough_domains ?? [],
          enable_h2_downstream: data.enable_h2_downstream ?? false,
        });
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      await apiPatch("/config", {
        intercept_https: config.intercept_https,
        max_requests: config.max_requests,
        verbose: config.verbose,
        public_ip: config.public_ip || null,
        passthrough_domains: config.passthrough_domains ?? [],
        enable_h2_downstream: config.enable_h2_downstream,
      });
      toast({ description: "Configuration saved successfully." });
    } catch (err) {
      const msg =
        err instanceof ApiError
          ? `Failed to save configuration (HTTP ${err.status}): ${err.body.slice(0, 200)}`
          : err instanceof Error
            ? `Failed to save configuration: ${err.message}`
            : "Failed to save configuration.";
      toast({ description: msg, variant: "destructive" });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48 text-muted-foreground">
        Loading…
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Section title="Proxy Address">
        <Row
          label="Proxy Port"
          description="Port clients connect to. Requires restart to change."
        >
          <Input
            value={config.proxy_port}
            disabled
            className="w-24 text-right opacity-70"
          />
        </Row>
        <Row
          label="API Port"
          description="Port for the web UI and API. Requires restart to change."
        >
          <Input
            value={config.api_port}
            disabled
            className="w-24 text-right opacity-70"
          />
        </Row>
        <Row
          label="Bind Host"
          description="Network interface the proxy is bound to. Requires restart."
        >
          <Input value={config.host} disabled className="w-36 opacity-70" />
        </Row>
        <Row
          label="Public IP Override"
          description="Display this IP to users instead of auto-detected. Useful for remote/NAT scenarios."
        >
          <Input
            value={config.public_ip}
            onChange={(e) =>
              setConfig((p) => ({ ...p, public_ip: e.target.value }))
            }
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
            onCheckedChange={(v) =>
              setConfig((p) => ({ ...p, intercept_https: v }))
            }
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
              setConfig((p) => ({
                ...p,
                max_requests: parseInt(e.target.value) || 1000,
              }))
            }
            min={100}
            max={100000}
            step={100}
            className="w-28 text-right"
          />
        </Row>
      </Section>

      <Section title="HTTP/2">
        <Row
          label="Enable HTTP/2 Downstream"
          description="Allow clients to negotiate HTTP/2 via ALPN. Enables HTTP/2 frame parsing on the client-facing side, required for gRPC interception. HTTP/1.1 clients continue to work via ALPN fallback. Restart required to take effect."
        >
          <Switch
            checked={config.enable_h2_downstream ?? false}
            onCheckedChange={(v) =>
              setConfig((p) => ({ ...p, enable_h2_downstream: v }))
            }
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
  const [cfg, setCfg] = useState<UpstreamConfig>(DEFAULT_UPSTREAM);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  // Load upstream proxy config from the backend API on mount.
  useEffect(() => {
    apiGet<RuntimeConfig & { upstream_proxy?: UpstreamProxyApiConfig }>(
      "/config",
    )
      .then((data) => {
        const up = data.upstream_proxy;
        if (up) {
          setCfg({
            enabled: up.enabled,
            protocol: (up.protocol as "http" | "https" | "socks5") ?? "http",
            host: up.host ?? "",
            port: up.port ? String(up.port) : "",
            auth_enabled: up.auth_enabled ?? false,
            username: up.auth_username ?? "",
            password: "", // password is write-only; never returned by API
            no_proxy: (up.no_proxy_hosts ?? []).join(", "),
          });
        }
      })
      .catch(() => {
        // Fall back to localStorage if the API is unavailable (e.g. older
        // server version that doesn't support upstream_proxy in the config).
        const ls = loadLS(LS_UPSTREAM, DEFAULT_UPSTREAM);
        setCfg(ls);
      })
      .finally(() => setLoading(false));
  }, []);

  const save = useCallback(async () => {
    setSaving(true);
    try {
      const noProxyHosts = cfg.no_proxy
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);

      await apiPatch("/config", {
        upstream_proxy: {
          enabled: cfg.enabled,
          protocol: cfg.protocol,
          host: cfg.host,
          port: cfg.port ? parseInt(cfg.port) : 0,
          auth_username: cfg.auth_enabled
            ? cfg.username || null
            : null,
          auth_password: cfg.auth_enabled ? cfg.password || null : null,
          no_proxy_hosts: noProxyHosts,
        },
      });
      toast({ description: "Upstream proxy settings saved." });
      // Clear the password field after save (it's write-only).
      setCfg((p) => ({ ...p, password: "" }));
    } catch (err) {
      const msg =
        err instanceof ApiError
          ? `Failed to save upstream proxy (HTTP ${err.status}): ${err.body.slice(0, 200)}`
          : err instanceof Error
            ? `Failed to save upstream proxy: ${err.message}`
            : "Failed to save upstream proxy settings.";
      toast({ description: msg, variant: "destructive" });
    } finally {
      setSaving(false);
    }
  }, [cfg, toast]);

  const reset = useCallback(() => {
    setCfg(DEFAULT_UPSTREAM);
    localStorage.removeItem(LS_UPSTREAM);
    toast({ description: "Upstream proxy settings reset." });
  }, [toast]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48 text-muted-foreground">
        Loading…
      </div>
    );
  }

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
                onChange={(e) =>
                  setCfg((p) => ({ ...p, host: e.target.value }))
                }
                placeholder="proxy.example.com"
                className="w-48"
              />
            </Row>
            <Row label="Port">
              <Input
                type="number"
                value={cfg.port}
                onChange={(e) =>
                  setCfg((p) => ({ ...p, port: e.target.value }))
                }
                placeholder="8080"
                className="w-24 text-right"
              />
            </Row>
          </Section>

          <Section title="Authentication">
            <Row label="Require Authentication">
              <Switch
                checked={cfg.auth_enabled}
                onCheckedChange={(v) =>
                  setCfg((p) => ({ ...p, auth_enabled: v }))
                }
              />
            </Row>
            {cfg.auth_enabled && (
              <>
                <Row label="Username">
                  <Input
                    value={cfg.username}
                    onChange={(e) =>
                      setCfg((p) => ({ ...p, username: e.target.value }))
                    }
                    className="w-40"
                  />
                </Row>
                <Row label="Password">
                  <Input
                    type="password"
                    value={cfg.password}
                    onChange={(e) =>
                      setCfg((p) => ({ ...p, password: e.target.value }))
                    }
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
                Comma-separated list of hosts to bypass the upstream proxy (e.g.
                localhost, 192.168.0.0/16).
              </p>
              <textarea
                value={cfg.no_proxy}
                onChange={(e) =>
                  setCfg((p) => ({ ...p, no_proxy: e.target.value }))
                }
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
        <Button onClick={save} disabled={saving}>
          <Save className="h-4 w-4 mr-2" />
          {saving ? "Saving…" : "Save Changes"}
        </Button>
      </div>
    </div>
  );
}

function CaptureTab() {
  const { toast } = useToast();
  const [cfg, setCfg] = useState<CaptureConfig>(DEFAULT_CAPTURE);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    apiGet<RuntimeConfig & { max_body_size?: number; capture_request_bodies?: boolean; capture_response_bodies?: boolean; ignored_domains?: string[]; max_total_size_mb?: number | null }>("/config")
      .then((data) => {
        setCfg({
          capture_request_bodies: data.capture_request_bodies ?? true,
          capture_response_bodies: data.capture_response_bodies ?? true,
          max_body_size_kb: data.max_body_size
            ? Math.round(data.max_body_size / 1024)
            : 20480,
          ignored_domains: (data.ignored_domains ?? []).join("\n"),
          max_total_size_mb: data.max_total_size_mb ?? null,
        });
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const save = useCallback(async () => {
    setSaving(true);
    try {
      const ignoredLines = cfg.ignored_domains
        .split("\n")
        .map((d) => d.trim())
        .filter((d) => d.length > 0);
      await apiPatch("/config", {
        capture_request_bodies: cfg.capture_request_bodies,
        capture_response_bodies: cfg.capture_response_bodies,
        max_body_size: cfg.max_body_size_kb * 1024,
        ignored_domains: ignoredLines,
        max_total_size_mb: cfg.max_total_size_mb,
      });
      toast({ description: "Capture settings saved." });
    } catch (err) {
      const msg =
        err instanceof ApiError
          ? `Failed to save capture settings (HTTP ${err.status}): ${err.body.slice(0, 200)}`
          : err instanceof Error
            ? `Failed to save capture settings: ${err.message}`
            : "Failed to save capture settings.";
      toast({ description: msg, variant: "destructive" });
    } finally {
      setSaving(false);
    }
  }, [cfg, toast]);

  const reset = useCallback(() => {
    setCfg(DEFAULT_CAPTURE);
    toast({ description: "Capture settings reset (click Save to apply)." });
  }, [toast]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48 text-muted-foreground">
        Loading…
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Section title="Body Recording">
        <Row
          label="Capture Request Bodies"
          description="Store request body content for inspection. Disable to reduce storage usage."
        >
          <Switch
            checked={cfg.capture_request_bodies}
            onCheckedChange={(v) =>
              setCfg((p) => ({ ...p, capture_request_bodies: v }))
            }
          />
        </Row>
        <Row
          label="Capture Response Bodies"
          description="Store response body content for inspection."
        >
          <Switch
            checked={cfg.capture_response_bodies}
            onCheckedChange={(v) =>
              setCfg((p) => ({ ...p, capture_response_bodies: v }))
            }
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
            onValueChange={([v]) =>
              setCfg((p) => ({ ...p, max_body_size_kb: v }))
            }
            className="w-full"
          />
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>16 KB</span>
            <span>4096 KB</span>
          </div>
        </div>
      </Section>

      <Section title="Recording Limits">
        <Row
          label="Max Total Recording Size"
          description="Total size of all stored bodies. When exceeded, oldest entries are pruned. Set to empty for unlimited."
        >
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={0}
              value={cfg.max_total_size_mb ?? ""}
              onChange={(e) => {
                const v = e.target.value;
                setCfg((p) => ({
                  ...p,
                  max_total_size_mb: v === "" ? null : parseInt(v, 10) || null,
                }));
              }}
              placeholder="Unlimited"
              className="w-28"
            />
            <span className="text-sm text-muted-foreground">MB</span>
          </div>
        </Row>
      </Section>

      <Section title="Domain Filtering">
        <div className="space-y-2">
          <Label className="text-sm font-medium">Ignored Domains</Label>
          <p className="text-xs text-muted-foreground">
            Traffic from these domains will not be recorded. One pattern per
            line. Supports wildcards (e.g. *.googleapis.com).
          </p>
          <textarea
            value={cfg.ignored_domains}
            onChange={(e) =>
              setCfg((p) => ({ ...p, ignored_domains: e.target.value }))
            }
            rows={5}
            placeholder={
              "*.google-analytics.com\n*.doubleclick.net\ntelemetry.example.com"
            }
            className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono resize-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          />
        </div>
      </Section>

      <div className="flex justify-between pt-2">
        <Button variant="ghost" onClick={reset}>
          <RotateCcw className="h-4 w-4 mr-2" />
          Reset
        </Button>
        <Button onClick={save} disabled={saving}>
          <Save className="h-4 w-4 mr-2" />
          {saving ? "Saving…" : "Save Changes"}
        </Button>
      </div>
    </div>
  );
}

function AppearanceTab() {
  const { toast } = useToast();
  const [cfg, setCfg] = useState<AppearanceConfig>(() =>
    loadLS(LS_APPEARANCE, DEFAULT_APPEARANCE),
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
      const prefersDark = window.matchMedia(
        "(prefers-color-scheme: dark)",
      ).matches;
      void (prefersDark
        ? root.classList.add("dark")
        : root.classList.remove("dark"));
    }

    // Emit custom event so App.tsx can sync its isDark state
    window.dispatchEvent(
      new CustomEvent("madhyamas-theme-change", { detail: cfg.theme }),
    );

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
        <Row label="Color Theme" description="Choose how Madhyamas looks.">
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
          label="Real-time Updates"
          description="Use WebSocket for instant traffic updates instead of polling."
        >
          <Select
            value={cfg.use_websocket ?? "true"}
            onValueChange={(v) => setCfg((p) => ({ ...p, use_websocket: v }))}
          >
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="true">WebSocket (Live)</SelectItem>
              <SelectItem value="false">Polling</SelectItem>
            </SelectContent>
          </Select>
        </Row>
        <Row
          label="Polling Interval"
          description="How often to refresh when using polling mode (fallback)."
        >
          <Select
            value={cfg.auto_refresh_interval}
            onValueChange={(v) =>
              setCfg((p) => ({ ...p, auto_refresh_interval: v }))
            }
            disabled={cfg.use_websocket === "true"}
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

// ─── SSL Passthrough Tab ──────────────────────────────────────────────────────

function SslPassthroughTab() {
  const { toast } = useToast();
  const [domains, setDomains] = useState<string[]>([]);
  const [newDomain, setNewDomain] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    apiGet<RuntimeConfig>("/config")
      .then((data) => {
        setDomains(data.passthrough_domains ?? []);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const handleAdd = () => {
    const trimmed = newDomain.trim().toLowerCase();
    if (!trimmed) return;
    if (domains.includes(trimmed)) {
      toast({ description: "Domain already in the list." });
      return;
    }
    setDomains([...domains, trimmed].sort());
    setNewDomain("");
  };

  const handleRemove = (domain: string) => {
    setDomains(domains.filter((d) => d !== domain));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await apiPatch("/config", {
        passthrough_domains: domains,
      });
      toast({ description: "SSL passthrough domains saved." });
    } catch (err) {
      const msg =
        err instanceof ApiError
          ? `Failed to save passthrough domains (HTTP ${err.status}): ${err.body.slice(0, 200)}`
          : err instanceof Error
            ? `Failed to save passthrough domains: ${err.message}`
            : "Failed to save passthrough domains.";
      toast({ description: msg, variant: "destructive" });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48 text-muted-foreground">
        Loading…
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <Section title="SSL Passthrough Domains">
        <p className="text-sm text-muted-foreground">
          Domains listed here will bypass TLS interception. The proxy tunnels
          the connection directly to the upstream server without decrypting
          traffic. The connections are still visible in the traffic list but
          flagged as passthrough (request/response bodies are not captured).
        </p>
        <p className="text-xs text-muted-foreground">
          Suffix matching is used: <code className="text-xs">example.com</code>{" "}
          matches <code className="text-xs">api.example.com</code> and{" "}
          <code className="text-xs">www.example.com</code>.
        </p>

        {/* Add domain input */}
        <div className="flex items-center gap-2">
          <Input
            placeholder="example.com"
            value={newDomain}
            onChange={(e) => setNewDomain(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleAdd();
              }
            }}
            className="flex-1"
          />
          <Button onClick={handleAdd} size="sm" disabled={!newDomain.trim()}>
            <Plus className="h-4 w-4 mr-1" />
            Add
          </Button>
        </div>

        {/* Domain list */}
        <div className="space-y-1.5">
          {domains.length === 0 ? (
            <div className="text-sm text-muted-foreground italic py-4 text-center border border-dashed rounded-md">
              No passthrough domains configured.
              <br />
              All HTTPS traffic will be intercepted.
            </div>
          ) : (
            domains.map((domain) => (
              <div
                key={domain}
                className="flex items-center justify-between gap-2 px-3 py-2 rounded-md border bg-muted/30"
              >
                <div className="flex items-center gap-2 min-w-0">
                  <ShieldOff className="h-3.5 w-3.5 text-amber-500 shrink-0" />
                  <span className="font-mono text-sm truncate">{domain}</span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => handleRemove(domain)}
                  className="h-7 px-2 text-muted-foreground hover:text-destructive"
                >
                  <X className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
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

// ─── Main dialog ───────────────────────────────────────────────────────────────

interface ConfigDialogProps {
  trigger?: React.ReactNode;
}

export function ConfigDialog({ trigger }: ConfigDialogProps) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        {trigger ?? (
          <Button variant="ghost" size="sm">
            Config
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="max-w-2xl max-h-[85vh] flex flex-col p-0">
        <DialogHeader className="px-6 pt-6 pb-0">
          <DialogTitle className="text-lg">Proxy Configuration</DialogTitle>
        </DialogHeader>
        <Tabs
          defaultValue="general"
          className="flex-1 flex flex-col min-h-0 mt-4"
        >
          <TabsList className="mx-6 justify-start shrink-0">
            <TabsTrigger value="general" className="flex items-center gap-1.5">
              <Server className="h-3.5 w-3.5" />
              General
            </TabsTrigger>
            <TabsTrigger value="upstream" className="flex items-center gap-1.5">
              <Network className="h-3.5 w-3.5" />
              Upstream Proxy
            </TabsTrigger>
            <TabsTrigger value="ssl" className="flex items-center gap-1.5">
              <ShieldOff className="h-3.5 w-3.5" />
              SSL Passthrough
            </TabsTrigger>
            <TabsTrigger value="capture" className="flex items-center gap-1.5">
              <Camera className="h-3.5 w-3.5" />
              Capture
            </TabsTrigger>
            <TabsTrigger
              value="appearance"
              className="flex items-center gap-1.5"
            >
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
            <TabsContent value="ssl" className="mt-0">
              <SslPassthroughTab />
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
