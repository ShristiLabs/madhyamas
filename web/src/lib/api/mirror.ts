/**
 * API hooks for the Mirror tool.
 *
 * These hooks wrap the `/api/mirror` endpoints using the shared API
 * client and TanStack Query for caching/invalidation.
 */
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPost, apiPatch } from './client';

/** Mirror status returned by GET /api/mirror. */
export interface MirrorStatus {
  enabled: boolean;
  output_dir: string;
  host_filter: string[] | null;
  save_request_bodies: boolean;
  files_written: number;
  bytes_written: number;
}

/** Query key for the mirror status query. */
const MIRROR_KEY = ['mirror', 'status'] as const;

/**
 * Fetch the current mirror status and statistics.
 */
export async function fetchMirrorStatus(): Promise<MirrorStatus> {
  return apiGet<MirrorStatus>('/mirror');
}

/**
 * React Query hook that fetches the mirror status.
 */
export function useMirrorStatus() {
  return useQuery({
    queryKey: MIRROR_KEY,
    queryFn: fetchMirrorStatus,
    staleTime: 0,
    refetchInterval: 5000,
  });
}

/** Toggle payload for POST /api/mirror/toggle. */
export interface ToggleMirrorPayload {
  enabled: boolean;
}

/**
 * React Query mutation hook that toggles mirroring on/off.
 */
export function useToggleMirror() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: ToggleMirrorPayload) =>
      apiPost<{ enabled: boolean; message: string }>('/mirror/toggle', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: MIRROR_KEY });
    },
  });
}

/** Partial update payload for PATCH /api/mirror/config. */
export interface UpdateMirrorConfigPayload {
  enabled?: boolean;
  output_dir?: string;
  /** Set to null to clear the host filter (mirror all hosts). */
  host_filter?: string[] | null;
  save_request_bodies?: boolean;
}

/**
 * React Query mutation hook that updates the mirror configuration.
 */
export function useUpdateMirrorConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateMirrorConfigPayload) =>
      apiPatch<MirrorStatus>('/mirror/config', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: MIRROR_KEY });
    },
  });
}
