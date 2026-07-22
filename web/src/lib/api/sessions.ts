import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPost, apiPostVoid, apiDeleteVoid, apiGetRaw } from './client';

// ==================== Types ====================

export interface Session {
  id: string;
  name: string | null;
  description?: string | null;
  created_at: string;
  updated_at?: string;
  request_count?: number;
  traffic_count?: number;
  notes?: string | null;
  tags?: string[];
}

export interface CreateSessionInput {
  name?: string;
  description?: string;
}

export interface SessionExport {
  version: string;
  exported_at: string;
  session: {
    id: string;
    name: string | null;
    created_at: string;
    updated_at: string;
    request_count: number;
    notes?: string | null;
    tags?: string[];
  };
  entries: unknown[];
}

// ==================== Sessions API ====================

export function useSessions() {
  return useQuery({
    queryKey: ['sessions'],
    queryFn: async (): Promise<Session[]> => {
      return apiGet<Session[]>('/sessions');
    },
  });
}

export function useCreateSession() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: CreateSessionInput): Promise<Session> => {
      return apiPost<Session>('/sessions', input);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sessions'] });
    },
  });
}

export function useDeleteSession() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      return apiDeleteVoid(`/sessions/${id}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sessions'] });
    },
  });
}

export function useSwitchSession() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      return apiPostVoid(`/sessions/${id}/switch`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sessions'] });
      queryClient.invalidateQueries({ queryKey: ['traffic'] });
      queryClient.invalidateQueries({ queryKey: ['traffic-count'] });
    },
  });
}

export function useImportSession() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (exportData: SessionExport): Promise<Session> => {
      return apiPost<Session>('/sessions/import', exportData);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['sessions'] });
    },
  });
}

/**
 * Export a session as a downloadable JSON file.
 * On-demand function (not a hook) since it triggers a browser download.
 */
export async function exportSession(id: string, name?: string): Promise<void> {
  const res = await apiGetRaw(`/sessions/${id}/export`);
  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  const safeName = (name || id).replace(/[^a-z0-9_-]+/gi, '_');
  a.download = `session-${safeName}.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
