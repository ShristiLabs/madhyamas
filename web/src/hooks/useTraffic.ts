import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import type { TrafficEntry, TrafficFilter } from "@/types/traffic";
import { useTrafficWebSocket } from "./useTrafficWebSocket";
import type { TrafficEntrySnapshot, WsConnectionInfo } from "@/types/websocket";
import { apiGet, apiPost, apiPostVoid } from "@/lib/api/client";

// Storage key for WebSocket mode preference
const WS_MODE_STORAGE_KEY = "madhyamas-use-websocket";

async function fetchTraffic(filter?: TrafficFilter): Promise<TrafficEntry[]> {
  const params = new URLSearchParams();
  if (filter?.url) params.set("url", filter.url);
  if (filter?.method) params.set("method", filter.method);
  if (filter?.limit) params.set("limit", filter.limit.toString());
  if (filter?.offset) params.set("offset", filter.offset.toString());
  if (filter?.search) params.set("search", filter.search);

  return apiGet<TrafficEntry[]>(`/traffic?${params}`);
}

async function fetchTrafficEntry(id: string): Promise<TrafficEntry> {
  return apiGet<TrafficEntry>(`/traffic/${id}`);
}

async function clearTraffic(): Promise<void> {
  await apiPostVoid("/traffic/clear");
}

async function fetchTrafficCount(): Promise<number> {
  const data = await apiGet<{ count: number }>("/traffic/count");
  return data.count;
}

// Convert WebSocket snapshot to full TrafficEntry format for compatibility
function snapshotToTrafficEntry(snapshot: TrafficEntrySnapshot): TrafficEntry {
  return {
    id: snapshot.id,
    session_id: snapshot.session_id,
    request: {
      method: snapshot.method as TrafficEntry["request"]["method"],
      url: snapshot.url,
      host: snapshot.host,
      path: snapshot.path,
      headers: {},
      body: snapshot.has_request_body ? undefined : undefined,
      content_type: snapshot.content_type ?? undefined,
      http_version: snapshot.http_version ?? undefined,
    },
    response: snapshot.status_code
      ? {
          status_code: snapshot.status_code,
          status_message: snapshot.status_message ?? undefined,
          headers: {},
          body: snapshot.has_response_body ? undefined : undefined,
          content_type: snapshot.response_content_type ?? undefined,
          duration_ms: snapshot.duration_ms ?? 0,
          http_version: snapshot.http_version ?? undefined,
        }
      : undefined,
    timestamp: snapshot.timestamp,
    modified: snapshot.modified,
    notes: undefined,
    request_size: snapshot.request_size,
    response_size: snapshot.response_size ?? undefined,
    is_passthrough: snapshot.is_passthrough ?? false,
    script_intercepted: snapshot.script_intercepted ?? false,
  };
}

interface UseTrafficOptions {
  filter?: TrafficFilter;
  useWebSocket?: boolean;
  pollingInterval?: number;
}

interface UseTrafficReturn {
  data: TrafficEntry[] | undefined;
  isLoading: boolean;
  isError: boolean;
  error: Error | null;
  refetch: () => void;
  // WebSocket-specific
  connectionInfo: WsConnectionInfo | null;
  isWebSocketMode: boolean;
  setWebSocketMode: (enabled: boolean) => void;
}

export function useTraffic(
  filterOrOptions?: TrafficFilter | UseTrafficOptions
): UseTrafficReturn {
  // Parse options
  const options: UseTrafficOptions = useMemo(() => {
    if (!filterOrOptions) return {};
    if ("useWebSocket" in filterOrOptions || "pollingInterval" in filterOrOptions) {
      return filterOrOptions as UseTrafficOptions;
    }
    return { filter: filterOrOptions as TrafficFilter };
  }, [filterOrOptions]);

  const { filter, pollingInterval = 1000 } = options;

  const queryClient = useQueryClient();

  // WebSocket mode state (persisted to localStorage)
  const [useWsMode, setUseWsMode] = useState<boolean>(() => {
    if (typeof window === "undefined") return true;
    const stored = localStorage.getItem(WS_MODE_STORAGE_KEY);
    return stored !== null ? stored === "true" : true; // Default to WebSocket mode
  });

  const setWebSocketMode = useCallback((enabled: boolean) => {
    setUseWsMode(enabled);
    localStorage.setItem(WS_MODE_STORAGE_KEY, String(enabled));
  }, []);

  // WebSocket hook
  const {
    traffic: wsTraffic,
    isLoading: wsLoading,
    connectionInfo,
    isConnected,
  } = useTrafficWebSocket({
    enabled: useWsMode,
  });

  // Invalidate script execution history when new traffic arrives over
  // WebSocket so the Scripts panel History tab reflects fresh executions
  // without waiting for the polling interval. Debounced to avoid
  // excessive refetches when traffic is flowing rapidly; TanStack Query
  // only refetches actively-observed queries (History tab open).
  const historyInvalidateTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (!wsTraffic || wsTraffic.length === 0) return;
    if (historyInvalidateTimer.current) clearTimeout(historyInvalidateTimer.current);
    historyInvalidateTimer.current = setTimeout(() => {
      queryClient.invalidateQueries({ queryKey: ["script-history"] });
    }, 500);
    return () => {
      if (historyInvalidateTimer.current) clearTimeout(historyInvalidateTimer.current);
    };
  }, [wsTraffic, queryClient]);

  // REST polling hook (fallback)
  const {
    data: restData,
    isLoading: restLoading,
    isError: restError,
    error: restErrorObj,
    refetch: restRefetch,
  } = useQuery({
    queryKey: ["traffic", filter],
    queryFn: () => fetchTraffic(filter),
    refetchInterval: useWsMode ? false : pollingInterval, // Disable polling when using WebSocket
    enabled: !useWsMode, // Only fetch when not using WebSocket
  });

  // Convert WebSocket snapshots to TrafficEntry format
  const wsData = useMemo(() => {
    if (!wsTraffic) return undefined;
    return wsTraffic.map(snapshotToTrafficEntry);
  }, [wsTraffic]);

  // Apply client-side filtering for WebSocket data
  const filteredWsData = useMemo(() => {
    if (!wsData || !filter) return wsData;

    return wsData.filter((entry) => {
      if (filter.search) {
        const searchLower = filter.search.toLowerCase();
        const matchesUrl = entry.request.url.toLowerCase().includes(searchLower);
        const matchesPath = entry.request.path.toLowerCase().includes(searchLower);
        if (!matchesUrl && !matchesPath) return false;
      }
      if (filter.method && entry.request.method !== filter.method) {
        return false;
      }
      return true;
    });
  }, [wsData, filter]);

  // Determine which data source to use
  const isWebSocketActive = useWsMode && isConnected;
  const data = isWebSocketActive ? filteredWsData : restData;
  const isLoading = isWebSocketActive ? wsLoading : restLoading;

  // Refetch function
  const refetch = useCallback(() => {
    if (!useWsMode) {
      restRefetch();
    }
    // For WebSocket mode, data is automatically updated
  }, [useWsMode, restRefetch]);

  return {
    data,
    isLoading,
    isError: !useWsMode && restError,
    error: !useWsMode ? (restErrorObj as Error | null) : null,
    refetch,
    connectionInfo: useWsMode ? connectionInfo : null,
    isWebSocketMode: useWsMode,
    setWebSocketMode,
  };
}

export function useTrafficEntry(id: string | null) {
  return useQuery({
    queryKey: ["traffic", id],
    queryFn: () => fetchTrafficEntry(id!),
    enabled: !!id,
  });
}

export function useTrafficCount() {
  return useQuery({
    queryKey: ["traffic-count"],
    queryFn: fetchTrafficCount,
    refetchInterval: 1000,
  });
}

export function useClearTraffic() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: clearTraffic,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["traffic"] });
      queryClient.invalidateQueries({ queryKey: ["traffic-count"] });
    },
  });
}

/** Result returned by the HAR import endpoint. */
export interface HarImportResult {
  session_id: string;
  imported_count: number;
  skipped_count: number;
  errors: string[];
}

interface ImportHarParams {
  har: unknown;
  sessionName?: string;
  switchSession?: boolean;
}

async function importHar(params: ImportHarParams): Promise<HarImportResult> {
  return apiPost<HarImportResult>("/traffic/import/har", {
    har: params.har,
    session_name: params.sessionName,
    switch_session: params.switchSession ?? false,
  });
}

/** Import traffic from a HAR JSON document into a new session. */
export function useImportHar() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: importHar,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["traffic"] });
      queryClient.invalidateQueries({ queryKey: ["traffic-count"] });
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
  });
}
