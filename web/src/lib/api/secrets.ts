/**
 * API hooks for secrets management (issue #87).
 *
 * Values are write-only: the API never returns plaintext secret values —
 * only names. Setting/updating a secret sends the value once and never
 * echoes it back.
 */
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiGet, apiPut, apiDeleteVoid } from './client';

/** Secret names returned by GET /api/secrets. Values are never included. */
export interface SecretsList {
  names: string[];
}

const SECRETS_KEY = ['secrets', 'names'] as const;

/** List secret names (never values). */
export async function fetchSecrets(): Promise<SecretsList> {
  return apiGet<SecretsList>('/secrets');
}

/** React Query hook that lists secret names. */
export function useSecrets() {
  return useQuery({
    queryKey: SECRETS_KEY,
    queryFn: fetchSecrets,
    staleTime: 5000,
  });
}

/** Create or update a secret (write-only value). */
export function useSetSecret() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ name, value }: { name: string; value: string }) =>
      apiPut<unknown>(`/secrets/${encodeURIComponent(name)}`, { value }),
    onSuccess: () => qc.invalidateQueries({ queryKey: SECRETS_KEY }),
  });
}

/** Delete a secret by name. */
export function useDeleteSecret() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) =>
      apiDeleteVoid(`/secrets/${encodeURIComponent(name)}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: SECRETS_KEY }),
  });
}
