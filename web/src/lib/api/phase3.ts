import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

const API_BASE = '/api';

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
      const res = await fetch(`${API_BASE}/grpc/connections`);
      if (!res.ok) throw new Error('Failed to fetch gRPC connections');
      return res.json();
    },
  });
}

export function useGrpcStreams() {
  return useQuery({
    queryKey: ['grpc-streams'],
    queryFn: async (): Promise<GrpcStream[]> => {
      const res = await fetch(`${API_BASE}/grpc/streams`);
      if (!res.ok) throw new Error('Failed to fetch gRPC streams');
      return res.json();
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

      const res = await fetch(`${API_BASE}/grpc/frames?${params}`);
      if (!res.ok) throw new Error('Failed to fetch gRPC frames');
      return res.json();
    },
  });
}

export function useGrpcStats() {
  return useQuery({
    queryKey: ['grpc-stats'],
    queryFn: async (): Promise<GrpcStats> => {
      const res = await fetch(`${API_BASE}/grpc/stats`);
      if (!res.ok) throw new Error('Failed to fetch gRPC stats');
      return res.json();
    },
  });
}

export function useClearGrpcFrames() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<void> => {
      const res = await fetch(`${API_BASE}/grpc/clear`, { method: 'POST' });
      if (!res.ok) throw new Error('Failed to clear gRPC frames');
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
      const res = await fetch(`${API_BASE}/scripts`);
      if (!res.ok) throw new Error('Failed to fetch scripts');
      return res.json();
    },
  });
}

export function useScriptTemplates() {
  return useQuery({
    queryKey: ['script-templates'],
    queryFn: async (): Promise<ScriptTemplate[]> => {
      const res = await fetch(`${API_BASE}/scripts/templates`);
      if (!res.ok) throw new Error('Failed to fetch script templates');
      return res.json();
    },
  });
}

export function useCreateScript() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (script: Omit<Script, 'id' | 'created_at' | 'updated_at'>): Promise<{ id: string; script: Script }> => {
      const res = await fetch(`${API_BASE}/scripts`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(script),
      });
      if (!res.ok) throw new Error('Failed to create script');
      return res.json();
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
      const res = await fetch(`${API_BASE}/scripts/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ source }),
      });
      if (!res.ok) throw new Error('Failed to update script');
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
      const res = await fetch(`${API_BASE}/scripts/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error('Failed to delete script');
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
      const res = await fetch(`${API_BASE}/scripts/${id}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled }),
      });
      if (!res.ok) throw new Error('Failed to toggle script');
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
      const res = await fetch(`${API_BASE}/plugins`);
      if (!res.ok) throw new Error('Failed to fetch plugins');
      return res.json();
    },
  });
}

export function usePlugin(id: string) {
  return useQuery({
    queryKey: ['plugin', id],
    queryFn: async (): Promise<Plugin> => {
      const res = await fetch(`${API_BASE}/plugins/${id}`);
      if (!res.ok) throw new Error('Failed to fetch plugin');
      return res.json();
    },
    enabled: !!id,
  });
}

export function usePluginStats(id: string) {
  return useQuery({
    queryKey: ['plugin-stats', id],
    queryFn: async (): Promise<PluginStats> => {
      const res = await fetch(`${API_BASE}/plugins/${id}/stats`);
      if (!res.ok) throw new Error('Failed to fetch plugin stats');
      return res.json();
    },
    enabled: !!id,
  });
}

export function useEnablePlugin() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const res = await fetch(`${API_BASE}/plugins/${id}/enable`, { method: 'POST' });
      if (!res.ok) throw new Error('Failed to enable plugin');
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
      const res = await fetch(`${API_BASE}/plugins/${id}/disable`, { method: 'POST' });
      if (!res.ok) throw new Error('Failed to disable plugin');
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
      const res = await fetch(`${API_BASE}/plugins/reload`, { method: 'POST' });
      if (!res.ok) throw new Error('Failed to reload plugins');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['plugins'] });
    },
  });
}
