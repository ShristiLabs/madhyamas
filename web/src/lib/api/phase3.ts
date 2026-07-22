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

export interface Script {
  id: string;
  name: string;
  source: string;
  description?: string;
  enabled: boolean;
  hooks: string[];
  created_at: string;
  updated_at: string;
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
    mutationFn: async ({ id, source }: { id: string; source: string }): Promise<void> => {
      return apiPut<void>(`/scripts/${id}`, { source });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['scripts'] });
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
