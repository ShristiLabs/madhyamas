import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPost, apiPostVoid, apiPut, apiDeleteVoid } from './client';

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

export interface RequestModifications {
  url?: string;
  method?: string;
  headers?: Record<string, string>;
  remove_headers?: string[];
  body?: string;
  follow_redirects?: boolean;
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

export interface ReplayBatchConfig {
  iterations: number;
  concurrency: number;
  delay_ms?: number;
}

export interface ReplayBatchResult {
  saved_request_id: string;
  results: ReplayResult[];
  total: number;
  succeeded: number;
  failed: number;
  min_ms: number;
  max_ms: number;
  avg_ms: number;
  p95_ms: number;
  started_at: string;
  finished_at: string;
}

// ==================== Breakpoints API ====================

export function useBreakpoints() {
  return useQuery({
    queryKey: ['breakpoints'],
    queryFn: async (): Promise<BreakpointRule[]> => {
      return apiGet<BreakpointRule[]>('/breakpoints');
    },
  });
}

export function usePausedTraffic() {
  return useQuery({
    queryKey: ['paused-traffic'],
    queryFn: async (): Promise<PausedTraffic[]> => {
      return apiGet<PausedTraffic[]>('/breakpoints/paused');
    },
  });
}

export function useCreateBreakpoint() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (breakpoint: Omit<BreakpointRule, 'id' | 'hit_count'>): Promise<BreakpointRule> => {
      return apiPost<BreakpointRule>('/breakpoints', breakpoint);
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
      return apiDeleteVoid(`/breakpoints/${id}`);
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
      return apiPostVoid(`/breakpoints/paused/${id}/resume`, action);
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
      return apiGet<MockRule[]>('/mocks');
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
      return apiPost<MockRule>('/mocks', body);
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
      return apiDeleteVoid(`/mocks/${id}`);
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
      return apiPostVoid(`/mocks/${id}/toggle`, { enabled });
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
      return apiPost<{ id: string }>(`/mocks/${id}/duplicate`, { new_name: newName });
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
      return apiGet<MockCollection[]>('/mocks/collections');
    },
  });
}

export function useCreateMockCollection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (collection: { name: string; description?: string; tags?: string[] }): Promise<{ id: string }> => {
      return apiPost<{ id: string }>('/mocks/collections', collection);
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
      // DELETE with a JSON body — pass via RequestInit since apiDeleteVoid
      // doesn't accept a body parameter directly.
      return apiDeleteVoid(`/mocks/collections/${id}`, {
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ delete_rules: deleteRules }),
      });
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
      return apiPost<{ toggled: number }>(`/mocks/collections/${id}/toggle`, { enabled });
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
      return apiGet<MockHitRecord[]>('/mocks/analytics');
    },
  });
}

export function useMockRuleAnalytics(id: string) {
  return useQuery({
    queryKey: ['mock-analytics', id],
    queryFn: async (): Promise<MockHitStats> => {
      return apiGet<MockHitStats>(`/mocks/${id}/analytics`);
    },
    enabled: !!id,
  });
}

export function useMockHitHistory(id: string) {
  return useQuery({
    queryKey: ['mock-hit-history', id],
    queryFn: async (): Promise<MockHitRecord[]> => {
      return apiGet<MockHitRecord[]>(`/mocks/${id}/history`);
    },
    enabled: !!id,
  });
}

export function useClearMockHitHistory() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<void> => {
      return apiPostVoid('/mocks/analytics/clear');
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
      return apiGet<MockRule[]>('/mocks/export');
    },
  });
}

export function useImportMocks() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ format, data }: { format: 'har' | 'openapi' | 'postman'; data: string }): Promise<{ imported: number }> => {
      return apiPost<{ imported: number }>('/mocks/import', { format, data });
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
      return apiGet<{ recording: boolean }>('/mocks/recording/status');
    },
  });
}

export function useSetMockRecording() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (enabled: boolean): Promise<{ recording: boolean }> => {
      return apiPost<{ recording: boolean }>('/mocks/recording', { enabled });
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
      return apiGet<MockRule[]>('/mocks/recording/recorded');
    },
  });
}

export function usePromoteRecordedMocks() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<{ promoted: number }> => {
      return apiPost<{ promoted: number }>('/mocks/recording/promote');
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
      return apiPostVoid('/mocks/recording/clear');
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
      return apiPut<void>(`/mocks/${id}`, rule);
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
      return apiPost<TestMockResult>(`/mocks/${id}/test`, { request });
    },
  });
}

export function usePreviewMockMatch() {
  return useMutation({
    mutationFn: async (request: TestMockRequest): Promise<TestMockResult> => {
      return apiPost<TestMockResult>('/mocks/preview', { request });
    },
  });
}

// ==================== Mock Versioning API ====================

export function useRollbackMock() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, version }: { id: string; version: number }): Promise<void> => {
      return apiPostVoid(`/mocks/${id}/rollback`, { version });
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
      return apiGet<MockRuleVersion[]>(`/mocks/${id}/versions`);
    },
    enabled: !!id,
  });
}

// ==================== Mock Collection Update API ====================

export function useUpdateMockCollection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, collection }: { id: string; collection: Partial<MockCollection> }): Promise<void> => {
      return apiPut<void>(`/mocks/collections/${id}`, collection);
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
      return apiGet<RewriteRule[]>('/rewrites');
    },
  });
}

export function useCreateRewrite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (rewrite: Omit<RewriteRule, 'id' | 'hit_count'>): Promise<RewriteRule> => {
      return apiPost<RewriteRule>('/rewrites', rewrite);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['rewrites'] });
    },
  });
}

export function useUpdateRewrite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      id,
      rewrite,
    }: {
      id: string;
      rewrite: Omit<RewriteRule, 'id' | 'hit_count'>;
    }): Promise<void> => {
      return apiPut<void>(`/rewrites/${id}`, rewrite);
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
      return apiDeleteVoid(`/rewrites/${id}`);
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
      return apiPostVoid(`/rewrites/${id}/toggle`, { enabled });
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
      return apiGet<ThrottleConfig>('/throttle');
    },
  });
}

export function useThrottlePresets() {
  return useQuery({
    queryKey: ['throttle-presets'],
    queryFn: async (): Promise<ThrottleProfile[]> => {
      return apiGet<ThrottleProfile[]>('/throttle/presets');
    },
  });
}

export function useSetThrottle() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (config: ThrottleConfig): Promise<void> => {
      return apiPostVoid('/throttle', config);
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
      return apiGet<SavedRequest[]>('/replay/saved');
    },
  });
}

export function useReplayHistory() {
  return useQuery({
    queryKey: ['replay-history'],
    queryFn: async (): Promise<ReplayResult[]> => {
      return apiGet<ReplayResult[]>('/replay/history');
    },
  });
}

export function useSaveRequest() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (data: { entry_id?: string; request: SavedRequest['request']; name: string }): Promise<SavedRequest> => {
      return apiPost<SavedRequest>('/replay/saved', data);
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
      return apiDeleteVoid(`/replay/saved/${id}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['saved-requests'] });
    },
  });
}

export function useReplayRequest() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, modifications }: { id: string; modifications?: RequestModifications }): Promise<ReplayResult> => {
      return apiPost<ReplayResult>(`/replay/execute/${id}`, { modifications });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['replay-history'] });
    },
  });
}

export function useReplayRequestBatch() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      id,
      modifications,
      config,
    }: {
      id: string;
      modifications?: RequestModifications;
      config: ReplayBatchConfig;
    }): Promise<ReplayBatchResult> => {
      return apiPost<ReplayBatchResult>(`/replay/execute/${id}/batch`, { modifications, config });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['replay-history'] });
    },
  });
}

// ==================== Block List API ====================

export interface BlockListEntry {
  id: string;
  pattern: string;
  note: string | null;
  enabled: boolean;
  hit_count: number;
  status_code: number;
  response_body: string;
  content_type: string;
  created_at: string;
  updated_at: string;
}

export interface BlockListStats {
  total: number;
  enabled: number;
  disabled: number;
  total_hits: number;
}

export interface CreateBlockListEntryRequest {
  pattern: string;
  note?: string;
  enabled?: boolean;
  status_code?: number;
  response_body?: string;
  content_type?: string;
}

export function useBlockList() {
  return useQuery({
    queryKey: ['blocklist'],
    queryFn: async (): Promise<BlockListEntry[]> => {
      return apiGet<BlockListEntry[]>('/blocklist');
    },
  });
}

export function useBlockListStats() {
  return useQuery({
    queryKey: ['blocklist-stats'],
    queryFn: async (): Promise<BlockListStats> => {
      return apiGet<BlockListStats>('/blocklist/stats');
    },
  });
}

export function useCreateBlockListEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (entry: CreateBlockListEntryRequest): Promise<{ id: string }> => {
      return apiPost<{ id: string }>('/blocklist', entry);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist-stats'] });
    },
  });
}

export function useUpdateBlockListEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, entry }: { id: string; entry: BlockListEntry }): Promise<void> => {
      return apiPut<void>(`/blocklist/${id}`, entry);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist-stats'] });
    },
  });
}

export function useDeleteBlockListEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      return apiDeleteVoid(`/blocklist/${id}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist-stats'] });
    },
  });
}

export function useToggleBlockListEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }): Promise<void> => {
      return apiPostVoid(`/blocklist/${id}/toggle`, { enabled });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['blocklist'] });
      queryClient.invalidateQueries({ queryKey: ['blocklist-stats'] });
    },
  });
}

