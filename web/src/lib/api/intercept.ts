import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

const API_BASE = '/api';

// ==================== Types ====================

export interface MatchCondition {
  type: 'url_pattern' | 'all' | 'host' | 'path';
  pattern?: string;
}

// Breakpoints
export interface BreakpointRule {
  id: string;
  name: string;
  condition: MatchCondition;
  direction: 'request' | 'response' | 'both';
  enabled: boolean;
  hit_count: number;
}

export interface PausedTraffic {
  id: string;
  request: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body?: string;
  };
  breakpoint_id: string;
  paused_at: string;
}

export interface BreakpointDecision {
  action: 'continue' | 'abort';
  modifications?: {
    headers?: Record<string, string>;
    body?: string;
  };
}

// Mocks
export interface MockResponse {
  status_code: number;
  headers?: Record<string, string>;
  body?: string;
  delay_ms?: number;
}

export interface MockRule {
  id: string;
  name: string;
  condition: MatchCondition;
  response: MockResponse;
  enabled: boolean;
  hit_count: number;
}

// Rewrites
export interface RewriteAction {
  type: 'set_header' | 'remove_header' | 'url_rewrite' | 'body_rewrite';
  name?: string;
  value?: string;
  pattern?: string;
  replacement?: string;
}

export interface RewriteRule {
  id: string;
  name: string;
  condition: MatchCondition;
  direction: 'request' | 'response' | 'both';
  rewrites: RewriteAction[];
  enabled: boolean;
  hit_count: number;
}

// Throttle
export interface ThrottleProfile {
  name: string;
  download_bps: number;
  upload_bps: number;
  latency_ms: number;
  jitter_ms: number;
  packet_loss_percent: number;
}

export interface ThrottleConfig {
  enabled: boolean;
  profile: ThrottleProfile;
}

// Replay
export interface SavedRequest {
  id: string;
  name: string;
  request: {
    method: string;
    url: string;
    headers: Record<string, string>;
    body?: string;
  };
  created_at: string;
}

export interface ReplayResult {
  id: string;
  saved_request_id: string;
  executed_at: string;
  error?: string;
  duration_ms?: number;
  response?: {
    status_code: number;
    headers: Record<string, string>;
    body?: string;
    duration_ms: number;
  };
}

// ==================== Breakpoints API ====================

export function useBreakpoints() {
  return useQuery({
    queryKey: ['breakpoints'],
    queryFn: async (): Promise<BreakpointRule[]> => {
      const res = await fetch(`${API_BASE}/breakpoints`);
      if (!res.ok) throw new Error('Failed to fetch breakpoints');
      return res.json();
    },
  });
}

export function usePausedTraffic() {
  return useQuery({
    queryKey: ['paused-traffic'],
    queryFn: async (): Promise<PausedTraffic[]> => {
      const res = await fetch(`${API_BASE}/breakpoints/paused`);
      if (!res.ok) throw new Error('Failed to fetch paused traffic');
      return res.json();
    },
  });
}

export function useCreateBreakpoint() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (breakpoint: Omit<BreakpointRule, 'id' | 'hit_count'>): Promise<BreakpointRule> => {
      const res = await fetch(`${API_BASE}/breakpoints`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(breakpoint),
      });
      if (!res.ok) throw new Error('Failed to create breakpoint');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['breakpoints'] });
    },
  });
}

export function useDeleteBreakpoint() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const res = await fetch(`${API_BASE}/breakpoints/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error('Failed to delete breakpoint');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['breakpoints'] });
    },
  });
}

export function useResumePaused() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, action }: { id: string; action: BreakpointDecision }): Promise<void> => {
      const res = await fetch(`${API_BASE}/breakpoints/paused/${id}/resume`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(action),
      });
      if (!res.ok) throw new Error('Failed to resume traffic');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['paused-traffic'] });
      queryClient.invalidateQueries({ queryKey: ['traffic'] });
    },
  });
}

// ==================== Mocks API ====================

export function useMocks() {
  return useQuery({
    queryKey: ['mocks'],
    queryFn: async (): Promise<MockRule[]> => {
      const res = await fetch(`${API_BASE}/mocks`);
      if (!res.ok) throw new Error('Failed to fetch mocks');
      return res.json();
    },
  });
}

export function useCreateMock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (mock: Omit<MockRule, 'id' | 'hit_count'>): Promise<MockRule> => {
      const res = await fetch(`${API_BASE}/mocks`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(mock),
      });
      if (!res.ok) throw new Error('Failed to create mock');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

export function useDeleteMock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error('Failed to delete mock');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

export function useToggleMock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/${id}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled }),
      });
      if (!res.ok) throw new Error('Failed to toggle mock');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

// ==================== Rewrites API ====================

export function useRewrites() {
  return useQuery({
    queryKey: ['rewrites'],
    queryFn: async (): Promise<RewriteRule[]> => {
      const res = await fetch(`${API_BASE}/rewrites`);
      if (!res.ok) throw new Error('Failed to fetch rewrites');
      return res.json();
    },
  });
}

export function useCreateRewrite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (rewrite: Omit<RewriteRule, 'id' | 'hit_count'>): Promise<RewriteRule> => {
      const res = await fetch(`${API_BASE}/rewrites`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(rewrite),
      });
      if (!res.ok) throw new Error('Failed to create rewrite');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rewrites'] });
    },
  });
}

export function useDeleteRewrite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const res = await fetch(`${API_BASE}/rewrites/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error('Failed to delete rewrite');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rewrites'] });
    },
  });
}

export function useToggleRewrite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }): Promise<void> => {
      const res = await fetch(`${API_BASE}/rewrites/${id}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled }),
      });
      if (!res.ok) throw new Error('Failed to toggle rewrite');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rewrites'] });
    },
  });
}

// ==================== Throttle API ====================

export function useThrottle() {
  return useQuery({
    queryKey: ['throttle'],
    queryFn: async (): Promise<ThrottleConfig> => {
      const res = await fetch(`${API_BASE}/throttle`);
      if (!res.ok) throw new Error('Failed to fetch throttle config');
      return res.json();
    },
  });
}

export function useThrottlePresets() {
  return useQuery({
    queryKey: ['throttle-presets'],
    queryFn: async (): Promise<ThrottleProfile[]> => {
      const res = await fetch(`${API_BASE}/throttle/presets`);
      if (!res.ok) throw new Error('Failed to fetch throttle presets');
      return res.json();
    },
  });
}

export function useSetThrottle() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (config: ThrottleConfig): Promise<void> => {
      const res = await fetch(`${API_BASE}/throttle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config),
      });
      if (!res.ok) throw new Error('Failed to set throttle config');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['throttle'] });
    },
  });
}

// ==================== Replay API ====================

export function useSavedRequests() {
  return useQuery({
    queryKey: ['saved-requests'],
    queryFn: async (): Promise<SavedRequest[]> => {
      const res = await fetch(`${API_BASE}/replay/saved`);
      if (!res.ok) throw new Error('Failed to fetch saved requests');
      return res.json();
    },
  });
}

export function useReplayHistory() {
  return useQuery({
    queryKey: ['replay-history'],
    queryFn: async (): Promise<ReplayResult[]> => {
      const res = await fetch(`${API_BASE}/replay/history`);
      if (!res.ok) throw new Error('Failed to fetch replay history');
      return res.json();
    },
  });
}

export function useSaveRequest() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (data: { entry_id?: string; request: SavedRequest['request']; name: string }): Promise<SavedRequest> => {
      const res = await fetch(`${API_BASE}/replay/saved`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });
      if (!res.ok) throw new Error('Failed to save request');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['saved-requests'] });
    },
  });
}

export function useDeleteSavedRequest() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const res = await fetch(`${API_BASE}/replay/saved/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error('Failed to delete saved request');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['saved-requests'] });
    },
  });
}

export function useReplayRequest() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, modifications }: { id: string; modifications?: Partial<SavedRequest['request']> }): Promise<ReplayResult> => {
      const res = await fetch(`${API_BASE}/replay/execute/${id}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ modifications }),
      });
      if (!res.ok) throw new Error('Failed to replay request');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['replay-history'] });
    },
  });
}
