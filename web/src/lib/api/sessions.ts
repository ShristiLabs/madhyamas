import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

const API_BASE = '/api';

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
      const res = await fetch(`${API_BASE}/sessions`);
      if (!res.ok) throw new Error('Failed to fetch sessions');
      return res.json();
    },
  });
}

export function useCreateSession() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: CreateSessionInput): Promise<Session> => {
      const res = await fetch(`${API_BASE}/sessions`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(input),
      });
      if (!res.ok) throw new Error('Failed to create session');
      return res.json();
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
      const res = await fetch(`${API_BASE}/sessions/${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error('Failed to delete session');
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
      const res = await fetch(`${API_BASE}/sessions/${id}/switch`, {
        method: 'POST',
      });
      if (!res.ok) throw new Error('Failed to switch session');
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
      const res = await fetch(`${API_BASE}/sessions/import`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(exportData),
      });
      if (!res.ok) throw new Error('Failed to import session');
      return res.json();
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
  const res = await fetch(`${API_BASE}/sessions/${id}/export`);
  if (!res.ok) throw new Error('Failed to export session');
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
