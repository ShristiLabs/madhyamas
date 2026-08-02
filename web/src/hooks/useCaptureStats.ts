/**
 * Hook for fetching recording quota statistics from the backend.
 *
 * Polls `GET /api/capture/stats` to report current usage against the
 * configured limits (max_entries, max_total_size_bytes, max_body_size).
 * Used by the header quota indicator and the ConfigDialog capture tab.
 */
import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/lib/api/client";

export interface CaptureStats {
  entry_count: number;
  max_entries: number;
  total_size_bytes: number;
  max_total_size_bytes: number;
  max_body_size: number;
  capture_enabled: boolean;
  capture_request_bodies: boolean;
  capture_response_bodies: boolean;
  ignored_domains: string[];
}

async function fetchCaptureStats(): Promise<CaptureStats> {
  return apiGet<CaptureStats>("/capture/stats");
}

/**
 * Fetch recording quota stats. Polls every 5 seconds by default.
 * Set `enabled: false` to disable polling.
 */
export function useCaptureStats(
  options?: { enabled?: boolean; refetchInterval?: number },
) {
  return useQuery({
    queryKey: ["capture-stats"],
    queryFn: fetchCaptureStats,
    enabled: options?.enabled ?? true,
    refetchInterval: options?.refetchInterval ?? 5000,
    staleTime: 2000,
  });
}
