import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TrafficView } from "./components/TrafficView";
import { CertificateHelper } from "./components/CertificateHelper";
import { ConfigDialog } from "./components/ConfigDialog";
import { Toaster } from "./components/ui/toaster";
import { Button } from "./components/ui/button";
import {
  Moon,
  Sun,
  Settings,
  SlidersHorizontal,
  CircleDot,
  CircleSlash,
} from "lucide-react";
import { useState, useEffect, useCallback } from "react";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      staleTime: 1000,
    },
  },
});

function App() {
  const [isDark, setIsDark] = useState(() => {
    if (typeof window !== "undefined") {
      return document.documentElement.classList.contains("dark");
    }
    return false;
  });
  const [proxyAddress, setProxyAddress] = useState("localhost:8888");
  const [captureEnabled, setCaptureEnabled] = useState(true);
  const [captureLoading, setCaptureLoading] = useState(false);

  useEffect(() => {
    if (isDark) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }, [isDark]);

  useEffect(() => {
    const handler = (e: Event) => {
      const theme = (e as CustomEvent<string>).detail;
      if (theme === "dark") setIsDark(true);
      else if (theme === "light") setIsDark(false);
      else {
        setIsDark(window.matchMedia("(prefers-color-scheme: dark)").matches);
      }
    };
    window.addEventListener("madhyamas-theme-change", handler);
    return () => window.removeEventListener("madhyamas-theme-change", handler);
  }, []);

  useEffect(() => {
    fetch("/api/capture")
      .then((r) => r.json())
      .then((d) => setCaptureEnabled(d.capture_enabled ?? true))
      .catch(() => {});
  }, []);

  const handleToggleCapture = useCallback(async () => {
    setCaptureLoading(true);
    try {
      const res = await fetch("/api/capture/toggle", { method: "POST" });
      const data = await res.json();
      setCaptureEnabled(data.capture_enabled);
    } catch {
      // ignore
    } finally {
      setCaptureLoading(false);
    }
  }, []);

  useEffect(() => {
    const fetchProxyConfig = async () => {
      try {
        const response = await fetch("/api/config");
        if (response.ok) {
          const config = await response.json();
          const host = config.host || "localhost";
          const port = config.proxy_port || 8888;
          setProxyAddress(`${host}:${port}`);
        }
      } catch (e) {
        console.log("Failed to fetch proxy config:", e);
      }
    };
    fetchProxyConfig();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <div className="h-screen flex flex-col bg-background">
        <header className="border-b px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-primary flex items-center justify-center">
              <span className="text-primary-foreground font-bold text-sm">
                M
              </span>
            </div>
            <h1 className="text-xl font-semibold">Madhyamas</h1>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm font-mono px-2.5 py-1 rounded-md bg-blue-50 dark:bg-blue-950 text-blue-700 dark:text-blue-300 border border-blue-200 dark:border-blue-800">
              Proxy: {proxyAddress}
            </span>
            <button
              onClick={handleToggleCapture}
              disabled={captureLoading}
              title={
                captureEnabled
                  ? "Recording — click to enable passthrough"
                  : "Passthrough — click to resume recording"
              }
              className={[
                "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium border transition-colors select-none",
                captureEnabled
                  ? "bg-green-50 dark:bg-green-950 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800 hover:bg-green-100 dark:hover:bg-green-900"
                  : "bg-amber-50 dark:bg-amber-950 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-800 hover:bg-amber-100 dark:hover:bg-amber-900",
                captureLoading
                  ? "opacity-50 cursor-not-allowed"
                  : "cursor-pointer",
              ].join(" ")}
            >
              {captureEnabled ? (
                <CircleDot className="h-3.5 w-3.5" />
              ) : (
                <CircleSlash className="h-3.5 w-3.5" />
              )}
              {captureEnabled ? "Recording" : "Passthrough"}
            </button>
            <CertificateHelper
              trigger={
                <Button variant="ghost" size="sm">
                  <Settings className="h-4 w-4 mr-1" />
                  Setup
                </Button>
              }
            />
            <ConfigDialog
              trigger={
                <Button variant="ghost" size="sm">
                  <SlidersHorizontal className="h-4 w-4 mr-1" />
                  Config
                </Button>
              }
            />
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setIsDark(!isDark)}
            >
              {isDark ? (
                <Sun className="h-4 w-4" />
              ) : (
                <Moon className="h-4 w-4" />
              )}
            </Button>
          </div>
        </header>
        <main className="flex-1 overflow-hidden">
          <TrafficView />
        </main>
      </div>
      <Toaster />
    </QueryClientProvider>
  );
}

export default App;
