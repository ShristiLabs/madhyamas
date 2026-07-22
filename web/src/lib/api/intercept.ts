import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

const API_BASE = '/api';

// ==================== Types ====================

export interface MatchCondition {
  type: 'url_pattern' | 'all' | 'host' | 'path' | 'method' | 'header' | 'query_param';
  pattern?: string;
  name?: string;
  value?: string;
}

export interface MatchConditionGroup {
  operator: 'and' | 'or';
  conditions: MatchCondition[];
}

export interface CompositeMatchCondition {
  groups: MatchConditionGroup[];
  rootOperator: 'and' | 'or';
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
  body_base64?: string;
  delay_ms?: number;
  delay_variance_ms?: number;
  template_enabled?: boolean;
  template_variables?: Record<string, string>;
}

export interface ResponseConfig {
  type: 'single' | 'sequence' | 'conditional' | 'probabilistic';
  response?: MockResponse;
  responses?: MockResponse[];
  current_index?: number;
  loop?: boolean;
  conditions?: ConditionalResponse[];
  default_response?: MockResponse;
  probabilistic_responses?: ProbabilisticResponse[];
}

export interface ConditionalResponse {
  condition: RequestCondition;
  response: MockResponse;
}

export interface RequestCondition {
  type: 'header_equals' | 'header_regex' | 'query_param' | 'body_contains' | 'body_json_path' | 'time_range' | 'hit_count_range';
  name?: string;
  value?: string;
  pattern?: string;
  path?: string;
  expected?: string;
  start_hour?: number;
  end_hour?: number;
  min_hits?: number;
  max_hits?: number;
}

export interface ProbabilisticResponse {
  weight: number;
  response: MockResponse;
}

export interface MockExpiration {
  type: 'date_time' | 'hit_count' | 'duration';
  expires_at?: string;
  max_hits?: number;
  duration_seconds?: number;
}

export interface MockRuleVersion {
  version: number;
  response_config: ResponseConfig;
  condition: MatchCondition;
  timestamp: string;
  comment?: string;
}

export interface MockRule {
  id: string;
  name: string;
  description?: string;
  tags?: string[];
  collection_id?: string;
  condition: MatchCondition;
  response_config: ResponseConfig;
  enabled: boolean;
  priority: number;
  created_at: string;
  updated_at: string;
  hit_count: number;
  expiration?: MockExpiration;
  version: number;
  version_history: MockRuleVersion[];
  response_schema?: string;
  response_script?: string;
}

export interface MockCollection {
  id: string;
  name: string;
  description?: string;
  tags: string[];
  created_at: string;
  updated_at: string;
  enabled: boolean;
}

export interface MockHitRecord {
  mock_id: string;
  timestamp: string;
  request_url: string;
  request_method: string;
  response_status: number;
  response_time_ms: number;
}

export interface MockHitStats {
  total_hits: number;
  avg_response_time_ms: number;
  min_response_time_ms: number;
  max_response_time_ms: number;
  first_hit?: string;
  last_hit?: string;
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
      // Transform MockRule → CreateMockRequest expected by the backend.
      // The backend expects { name, condition, response: MockResponse, enabled?, priority? }
      // but the UI model uses response_config: ResponseConfig (which wraps MockResponse).
      const rc = mock.response_config;
      const response: MockResponse = rc.response ?? rc.default_response ?? rc.responses?.[0] ?? {
        status_code: 200,
      };
      const body = {
        name: mock.name,
        condition: mock.condition,
        response,
        enabled: mock.enabled,
        priority: mock.priority,
      };
      const res = await fetch(`${API_BASE}/mocks`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
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

export function useDuplicateMock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, newName }: { id: string; newName?: string }): Promise<{ id: string }> => {
      const res = await fetch(`${API_BASE}/mocks/${id}/duplicate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ new_name: newName }),
      });
      if (!res.ok) throw new Error('Failed to duplicate mock');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

// ==================== Mock Collections API ====================

export function useMockCollections() {
  return useQuery({
    queryKey: ['mock-collections'],
    queryFn: async (): Promise<MockCollection[]> => {
      const res = await fetch(`${API_BASE}/mocks/collections`);
      if (!res.ok) throw new Error('Failed to fetch mock collections');
      return res.json();
    },
  });
}

export function useCreateMockCollection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (collection: { name: string; description?: string; tags?: string[] }): Promise<{ id: string }> => {
      const res = await fetch(`${API_BASE}/mocks/collections`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(collection),
      });
      if (!res.ok) throw new Error('Failed to create collection');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mock-collections'] });
    },
  });
}

export function useDeleteMockCollection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, deleteRules }: { id: string; deleteRules?: boolean }): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/collections/${id}`, {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ delete_rules: deleteRules }),
      });
      if (!res.ok) throw new Error('Failed to delete collection');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mock-collections'] });
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

export function useToggleMockCollection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }): Promise<{ toggled: number }> => {
      const res = await fetch(`${API_BASE}/mocks/collections/${id}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled }),
      });
      if (!res.ok) throw new Error('Failed to toggle collection');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
      queryClient.invalidateQueries({ queryKey: ['mock-collections'] });
    },
  });
}

// ==================== Mock Analytics API ====================

export function useMockAnalytics() {
  return useQuery({
    queryKey: ['mock-analytics'],
    queryFn: async (): Promise<MockHitRecord[]> => {
      const res = await fetch(`${API_BASE}/mocks/analytics`);
      if (!res.ok) throw new Error('Failed to fetch mock analytics');
      return res.json();
    },
  });
}

export function useMockRuleAnalytics(id: string) {
  return useQuery({
    queryKey: ['mock-analytics', id],
    queryFn: async (): Promise<MockHitStats> => {
      const res = await fetch(`${API_BASE}/mocks/${id}/analytics`);
      if (!res.ok) throw new Error('Failed to fetch mock analytics');
      return res.json();
    },
    enabled: !!id,
  });
}

export function useMockHitHistory(id: string) {
  return useQuery({
    queryKey: ['mock-hit-history', id],
    queryFn: async (): Promise<MockHitRecord[]> => {
      const res = await fetch(`${API_BASE}/mocks/${id}/history`);
      if (!res.ok) throw new Error('Failed to fetch hit history');
      return res.json();
    },
    enabled: !!id,
  });
}

export function useClearMockHitHistory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/analytics/clear`, {
        method: 'POST',
      });
      if (!res.ok) throw new Error('Failed to clear hit history');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mock-analytics'] });
      queryClient.invalidateQueries({ queryKey: ['mock-hit-history'] });
    },
  });
}

// ==================== Mock Import/Export API ====================

export function useExportMocks() {
  return useMutation({
    mutationFn: async (): Promise<MockRule[]> => {
      const res = await fetch(`${API_BASE}/mocks/export`);
      if (!res.ok) throw new Error('Failed to export mocks');
      return res.json();
    },
  });
}

export function useImportMocks() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ format, data }: { format: 'har' | 'openapi' | 'postman'; data: string }): Promise<{ imported: number }> => {
      const res = await fetch(`${API_BASE}/mocks/import`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ format, data }),
      });
      if (!res.ok) throw new Error('Failed to import mocks');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

// ==================== Mock Recording API ====================

export function useMockRecordingStatus() {
  return useQuery({
    queryKey: ['mock-recording-status'],
    queryFn: async (): Promise<{ recording: boolean }> => {
      const res = await fetch(`${API_BASE}/mocks/recording/status`);
      if (!res.ok) throw new Error('Failed to fetch recording status');
      return res.json();
    },
  });
}

export function useSetMockRecording() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (enabled: boolean): Promise<{ recording: boolean }> => {
      const res = await fetch(`${API_BASE}/mocks/recording`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled }),
      });
      if (!res.ok) throw new Error('Failed to set recording');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mock-recording-status'] });
    },
  });
}

export function useRecordedMocks() {
  return useQuery({
    queryKey: ['recorded-mocks'],
    queryFn: async (): Promise<MockRule[]> => {
      const res = await fetch(`${API_BASE}/mocks/recording/recorded`);
      if (!res.ok) throw new Error('Failed to fetch recorded mocks');
      return res.json();
    },
  });
}

export function usePromoteRecordedMocks() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<{ promoted: number }> => {
      const res = await fetch(`${API_BASE}/mocks/recording/promote`, {
        method: 'POST',
      });
      if (!res.ok) throw new Error('Failed to promote recorded mocks');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
      queryClient.invalidateQueries({ queryKey: ['recorded-mocks'] });
    },
  });
}

export function useClearRecordedMocks() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/recording/clear`, {
        method: 'POST',
      });
      if (!res.ok) throw new Error('Failed to clear recorded mocks');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['recorded-mocks'] });
    },
  });
}

// ==================== Mock Update API ====================

export function useUpdateMock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, rule }: { id: string; rule: MockRule }): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(rule),
      });
      if (!res.ok) throw new Error('Failed to update mock');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

// ==================== Mock Testing API ====================

export interface TestMockRequest {
  url: string;
  method: string;
  headers: Record<string, string>;
  body?: string;
}

export interface TestMockResult {
  matches: boolean;
  rule_id?: string;
  rule_name?: string;
  response?: MockResponse;
}

export function useTestMock() {
  return useMutation({
    mutationFn: async ({ id, request }: { id: string; request: TestMockRequest }): Promise<TestMockResult> => {
      const res = await fetch(`${API_BASE}/mocks/${id}/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ request }),
      });
      if (!res.ok) throw new Error('Failed to test mock');
      return res.json();
    },
  });
}

export function usePreviewMockMatch() {
  return useMutation({
    mutationFn: async (request: TestMockRequest): Promise<TestMockResult> => {
      const res = await fetch(`${API_BASE}/mocks/preview`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ request }),
      });
      if (!res.ok) throw new Error('Failed to preview mock match');
      return res.json();
    },
  });
}

// ==================== Mock Versioning API ====================

export function useRollbackMock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, version }: { id: string; version: number }): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/${id}/rollback`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ version }),
      });
      if (!res.ok) throw new Error('Failed to rollback mock');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mocks'] });
    },
  });
}

export function useMockVersionHistory(id: string) {
  return useQuery({
    queryKey: ['mock-version-history', id],
    queryFn: async (): Promise<MockRuleVersion[]> => {
      const res = await fetch(`${API_BASE}/mocks/${id}/versions`);
      if (!res.ok) throw new Error('Failed to fetch version history');
      return res.json();
    },
    enabled: !!id,
  });
}

// ==================== Mock Collection Update API ====================

export function useUpdateMockCollection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, collection }: { id: string; collection: Partial<MockCollection> }): Promise<void> => {
      const res = await fetch(`${API_BASE}/mocks/collections/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(collection),
      });
      if (!res.ok) throw new Error('Failed to update collection');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mock-collections'] });
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
