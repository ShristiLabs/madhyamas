/**
 * API hooks for Auto Save configuration.
 *
 * These hooks wrap the `/api/autosave` endpoints using the shared API
 * client and TanStack Query for caching/invalidation.
 */
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPatch, apiPost } from './client';

/** Auto Save configuration returned by GET /api/autosave. */
export interface AutoSaveConfig {
  enabled: boolean;
  interval_seconds: number;
  export_format: 'har' | 'session';
  output_dir: string;
  max_backups: number;
  rotate_after_requests: number | null;
  rotate_after_minutes: number | null;
}

/** Query key for the Auto Save config query. */
const AUTOSAVE_KEY = ['autosave', 'config'] as const;

/**
 * Fetch the current Auto Save configuration.
 *
 * Use this inside React Query via {@link useAutoSaveConfig}, or call
 * directly when you need the promise.
 */
export async function fetchAutoSaveConfig(): Promise<AutoSaveConfig> {
  return apiGet<AutoSaveConfig>('/autosave');
}

/**
 * React Query hook that fetches the Auto Save configuration.
 */
export function useAutoSaveConfig() {
  return useQuery({
    queryKey: AUTOSAVE_KEY,
    queryFn: fetchAutoSaveConfig,
    staleTime: 0,
  });
}

/** Partial update payload for PATCH /api/autosave. */
export interface UpdateAutoSaveConfigPayload {
  enabled?: boolean;
  interval_seconds?: number;
  export_format?: 'har' | 'session';
  output_dir?: string;
  max_backups?: number;
  /** Set to null to disable request-based rotation. */
  rotate_after_requests?: number | null;
  /** Set to null to disable time-based rotation. */
  rotate_after_minutes?: number | null;
}

/**
 * React Query mutation hook that updates the Auto Save configuration.
 *
 * On success, the `autosave` query is invalidated so dependent components
 * refetch the latest config.
 */
export function useUpdateAutoSaveConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (payload: UpdateAutoSaveConfigPayload) =>
      apiPatch<AutoSaveConfig>('/autosave', payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AUTOSAVE_KEY });
    },
  });
}

/**
 * Trigger an immediate Auto Save snapshot (manual "save now").
 */
export function useTriggerAutoSaveSnapshot() {
  return useMutation({
    mutationFn: () =>
      apiPost<{ success: boolean; message: string; output_dir: string }>(
        '/autosave/snapshot',
        null,
      ),
  });
}
