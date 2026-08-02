import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPost, apiPostVoid, apiPut, apiDeleteVoid } from './client';

// ==================== gRPC Types ====================

export interface GrpcConnection {
  id: string;
  host: string;
  port: number;
  state: 'idle' | 'active' | 'closed';
  created_at: string;
  last_updated_at: string;
}

export interface GrpcStream {
  id: string;
  connection_id: string;
  service: string;
  method: string;
  state: 'idle' | 'open' | 'closed';
  direction: 'unary' | 'server_streaming' | 'client_streaming' | 'bidi_streaming';
  message_count: number;
}

export interface GrpcFrame {
  id: string;
  stream_id: string;
  direction: 'request' | 'response';
  message_type: 'data' | 'trailers';
  compressed: boolean;
  sequence: number;
  payload?: string;
  timestamp: string;
  service?: string;
  method?: string;
  status_code?: number;
}

export interface GrpcStats {
  total_connections: number;
  active_connections: number;
  total_streams: number;
  active_streams: number;
  total_frames: number;
  frames_sent: number;
  frames_received: number;
}

export interface GrpcFilter {
  service?: string;
  method?: string;
  path?: string;
  direction?: string;
  search?: string;
  limit?: number;
  offset?: number;
  status_code?: number;
}

// ==================== Script Types ====================

/** Declarative match filter — script only fires on matching requests. */
export interface ScriptMatch {
  url_pattern?: string;
  host_pattern?: string;
  path_pattern?: string;
  method?: string;
}

/** Per-script error policy: what happens to the script chain when this
 *  script returns an error. */
export type ScriptErrorPolicy = 'continue' | 'stop_chain';

export interface Script {
  id: string;
  name: string;
  source: string;
  description?: string;
  enabled: boolean;
  hooks: string[];
  created_at: string;
  updated_at: string;
  match_filter?: ScriptMatch | null;
  /** Execution priority (lower runs first).  Defaults to 100. */
  priority?: number;
  /** Per-script error policy.  Defaults to `stop_chain`. */
  on_error?: ScriptErrorPolicy;
}

export interface ScriptTemplate {
  name: string;
  description: string;
  source: string;
  hooks: string[];
}

export interface ScriptConfig {
  runtime: string;
  timeout_ms: number;
  max_memory_mb: number;
  enable_console: boolean;
}

export interface ScriptExecution {
  script_id: string;
  duration_ms: number;
  success: boolean;
  error?: string;
  console: string[];
  timestamp: string;
  /** Traffic entry ID this execution was associated with, if any. */
  traffic_entry_id?: string;
  /** Which hook triggered this execution (e.g. "on_request"). */
  hook?: string;
}

/** Script execution entry enriched with the script name, returned by
 * `GET /api/scripts/history` (the global history endpoint). */
export interface ScriptHistoryEntry {
  script_id: string;
  script_name: string | null;
  duration_ms: number;
  success: boolean;
  error?: string;
  console: string[];
  timestamp: string;
  traffic_entry_id?: string;
  hook?: string;
}

/** Script execution trace enriched with script name, returned by
 * `GET /api/traffic/{id}/script-traces`. */
export interface ScriptTrace {
  script_id: string;
  script_name: string | null;
  duration_ms: number;
  success: boolean;
  error?: string;
  console: string[];
  timestamp: string;
  traffic_entry_id?: string;
  hook?: string;
}

/** Match-preview item returned by `POST /api/scripts/match-preview`. */
export interface MatchPreviewItem {
  id: string;
  name: string;
  priority: number;
  enabled: boolean;
  hooks: string[];
  match_filter?: ScriptMatch | null;
}

export interface ScriptTestResult {
  modified: boolean;
  continue_: boolean;
  response?: { statusCode: number; headers: Record<string, string>; body: string };
  error?: string;
  console: string[];
  duration_ms: number;
  modified_request?: Record<string, unknown>;
  modified_response?: Record<string, unknown>;
}

export interface ScriptValidateResult {
  valid: boolean;
  error?: string;
}

// ==================== Plugin Types ====================

export interface Plugin {
  id: string;
  manifest: PluginManifest;
  state: 'disabled' | 'enabled' | 'error';
  enabled: boolean;
  error?: string;
  stats?: PluginStats;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  homepage?: string;
  repository?: string;
  main: string;
  hooks: string[];
  capabilities: string[];
  dependencies?: Record<string, string>;
}

export interface PluginStats {
  requests_processed: number;
  responses_modified: number;
  errors: number;
  avg_duration_ms: number;
  last_executed_at?: string;
}

// ==================== gRPC API ====================

export function useGrpcConnections() {
  return useQuery({
    queryKey: ['grpc-connections'],
    queryFn: async (): Promise<GrpcConnection[]> => {
      return apiGet<GrpcConnection[]>('/grpc/connections');
    },
  });
}

export function useGrpcStreams() {
  return useQuery({
    queryKey: ['grpc-streams'],
    queryFn: async (): Promise<GrpcStream[]> => {
      return apiGet<GrpcStream[]>('/grpc/streams');
    },
  });
}

export function useGrpcFrames(filter?: GrpcFilter) {
  return useQuery({
    queryKey: ['grpc-frames', filter],
    queryFn: async (): Promise<GrpcFrame[]> => {
      const params = new URLSearchParams();
      if (filter?.service) params.append('service', filter.service);
      if (filter?.method) params.append('method', filter.method);
      if (filter?.path) params.append('path', filter.path);
      if (filter?.direction) params.append('direction', filter.direction);
      if (filter?.search) params.append('search', filter.search);
      if (filter?.limit) params.append('limit', filter.limit.toString());
      if (filter?.offset) params.append('offset', filter.offset.toString());
      if (filter?.status_code) params.append('status_code', filter.status_code.toString());

      return apiGet<GrpcFrame[]>(`/grpc/frames?${params}`);
    },
  });
}

export function useGrpcStats() {
  return useQuery({
    queryKey: ['grpc-stats'],
    queryFn: async (): Promise<GrpcStats> => {
      return apiGet<GrpcStats>('/grpc/stats');
    },
  });
}

export function useClearGrpcFrames() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<void> => {
      return apiPostVoid('/grpc/clear');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['grpc-frames', 'grpc-stats'] });
    },
  });
}

// ==================== Scripts API ====================

export function useScripts() {
  return useQuery({
    queryKey: ['scripts'],
    queryFn: async (): Promise<Script[]> => {
      return apiGet<Script[]>('/scripts');
    },
  });
}

export function useScriptTemplates() {
  return useQuery({
    queryKey: ['script-templates'],
    queryFn: async (): Promise<ScriptTemplate[]> => {
      return apiGet<ScriptTemplate[]>('/scripts/templates');
    },
  });
}

export function useCreateScript() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (script: Omit<Script, 'id' | 'created_at' | 'updated_at'>): Promise<{ id: string; script: Script }> => {
      return apiPost<{ id: string; script: Script }>('/scripts', script);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scripts'] });
    },
  });
}

export function useUpdateScript() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (params: {
      id: string;
      source?: string;
      name?: string;
      description?: string;
      hooks?: string[];
      match_filter?: ScriptMatch | null;
      priority?: number;
      on_error?: ScriptErrorPolicy;
    }): Promise<void> => {
      const body: Record<string, unknown> = {};
      if (params.source !== undefined) body.source = params.source;
      if (params.name !== undefined) body.name = params.name;
      if (params.description !== undefined) body.description = params.description;
      if (params.hooks !== undefined) body.hooks = params.hooks;
      if (params.match_filter !== undefined) body.match_filter = params.match_filter;
      if (params.priority !== undefined) body.priority = params.priority;
      if (params.on_error !== undefined) body.on_error = params.on_error;
      return apiPut<void>(`/scripts/${params.id}`, body);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scripts'] });
    },
  });
}

/** Reorder a script up (run earlier) or down (run later) by renumbering
 * priorities so the new order is stable. */
export function useReorderScript() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, direction }: { id: string; direction: 'up' | 'down' }): Promise<void> => {
      return apiPostVoid(`/scripts/${id}/reorder`, { direction });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scripts'] });
    },
  });
}

/** Preview which scripts would match a given request without executing
 * them.  Returns the scripts in execution order (by priority). */
export function useMatchPreview() {
  return useMutation({
    mutationFn: async (req: {
      method: string;
      url: string;
      host: string;
      path: string;
      hook?: string;
    }): Promise<MatchPreviewItem[]> => {
      return apiPost<MatchPreviewItem[]>('/scripts/match-preview', req);
    },
  });
}

/** Get script execution traces for a specific traffic entry. */
export function useTrafficScriptTraces(trafficId: string | null) {
  return useQuery({
    queryKey: ['traffic-script-traces', trafficId],
    queryFn: async (): Promise<ScriptTrace[]> => {
      return apiGet<ScriptTrace[]>(`/traffic/${trafficId}/script-traces`);
    },
    enabled: !!trafficId,
    refetchInterval: trafficId ? 3000 : false,
  });
}

/** Get the global script runtime config (timeout, error policy, etc.). */
export function useScriptConfig() {
  return useQuery({
    queryKey: ['script-config'],
    queryFn: async (): Promise<ScriptConfig> => {
      return apiGet<ScriptConfig>('/scripts/config');
    },
  });
}

/** Update the global script runtime config. */
export function useUpdateScriptConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (config: Partial<ScriptConfig>): Promise<void> => {
      return apiPut<void>('/scripts/config', config);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['script-config'] });
    },
  });
}

export function useDeleteScript() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      return apiDeleteVoid(`/scripts/${id}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scripts'] });
    },
  });
}

export function useToggleScript() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }): Promise<void> => {
      return apiPostVoid(`/scripts/${id}/toggle`, { enabled });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scripts'] });
    },
  });
}

export function useScriptHistory(id: string | null, limit = 50) {
  return useQuery({
    queryKey: ['script-history', id, limit],
    queryFn: async (): Promise<ScriptExecution[]> => {
      return apiGet<ScriptExecution[]>(`/scripts/${id}/history?limit=${limit}`);
    },
    enabled: !!id,
    // Poll every 2s while a script is selected so the history pane
    // reflects new executions as matching traffic flows through the proxy.
    // TanStack Query only refetches while the query is actively observed
    // (i.e. the History tab is open), so this is idle when the tab is closed.
    refetchInterval: id ? 2000 : false,
  });
}

/** Get recent executions across **all** scripts (global history view).
 *  Entries are enriched with the script name.  Polls every 3s so the
 *  History tab stays live as traffic flows through the proxy. */
export function useAllScriptHistory(limit = 100) {
  return useQuery({
    queryKey: ['script-history-all', limit],
    queryFn: async (): Promise<ScriptHistoryEntry[]> => {
      return apiGet<ScriptHistoryEntry[]>(`/scripts/history?limit=${limit}`);
    },
    // Poll every 3s so the global history view reflects new executions.
    // TanStack Query only refetches while the query is actively observed
    // (i.e. the History tab is open), so this is idle when the tab is closed.
    refetchInterval: 3000,
  });
}

export function useTestScript() {
  return useMutation({
    mutationFn: async ({ source, hook }: { source: string; hook: string }): Promise<ScriptTestResult> => {
      return apiPost<ScriptTestResult>('/scripts/test', { source, hook });
    },
  });
}

export function useValidateScript() {
  return useMutation({
    mutationFn: async (source: string): Promise<ScriptValidateResult> => {
      return apiPost<ScriptValidateResult>('/scripts/validate', { source });
    },
  });
}

// ==================== Plugins API ====================

export function usePlugins() {
  return useQuery({
    queryKey: ['plugins'],
    queryFn: async (): Promise<Plugin[]> => {
      return apiGet<Plugin[]>('/plugins');
    },
  });
}

export function usePlugin(id: string) {
  return useQuery({
    queryKey: ['plugin', id],
    queryFn: async (): Promise<Plugin> => {
      return apiGet<Plugin>(`/plugins/${id}`);
    },
    enabled: !!id,
  });
}

export function usePluginStats(id: string) {
  return useQuery({
    queryKey: ['plugin-stats', id],
    queryFn: async (): Promise<PluginStats> => {
      return apiGet<PluginStats>(`/plugins/${id}/stats`);
    },
    enabled: !!id,
  });
}

export function useEnablePlugin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      return apiPostVoid(`/plugins/${id}/enable`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['plugins'] });
    },
  });
}

export function useDisablePlugin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      return apiPostVoid(`/plugins/${id}/disable`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['plugins'] });
    },
  });
}

export function useReloadPlugins() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<void> => {
      return apiPostVoid('/plugins/reload');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['plugins'] });
    },
  });
}
