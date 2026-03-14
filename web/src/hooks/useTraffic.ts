import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import type { TrafficEntry, TrafficFilter } from '@/types/traffic'

const API_BASE = '/api'

async function fetchTraffic(filter?: TrafficFilter): Promise<TrafficEntry[]> {
  const params = new URLSearchParams()
  if (filter?.url) params.set('url', filter.url)
  if (filter?.method) params.set('method', filter.method)
  if (filter?.limit) params.set('limit', filter.limit.toString())
  if (filter?.offset) params.set('offset', filter.offset.toString())
  if (filter?.search) params.set('search', filter.search)

  const response = await fetch(`${API_BASE}/traffic?${params}`)
  if (!response.ok) throw new Error('Failed to fetch traffic')
  return response.json()
}

async function fetchTrafficEntry(id: string): Promise<TrafficEntry> {
  const response = await fetch(`${API_BASE}/traffic/${id}`)
  if (!response.ok) throw new Error('Failed to fetch traffic entry')
  return response.json()
}

async function clearTraffic(): Promise<void> {
  const response = await fetch(`${API_BASE}/traffic/clear`, { method: 'POST' })
  if (!response.ok) throw new Error('Failed to clear traffic')
}

async function fetchTrafficCount(): Promise<number> {
  const response = await fetch(`${API_BASE}/traffic/count`)
  if (!response.ok) throw new Error('Failed to fetch traffic count')
  const data = await response.json()
  return data.count
}

export function useTraffic(filter?: TrafficFilter) {
  return useQuery({
    queryKey: ['traffic', filter],
    queryFn: () => fetchTraffic(filter),
    refetchInterval: 1000, // Poll every second
  })
}

export function useTrafficEntry(id: string | null) {
  return useQuery({
    queryKey: ['traffic', id],
    queryFn: () => fetchTrafficEntry(id!),
    enabled: !!id,
  })
}

export function useTrafficCount() {
  return useQuery({
    queryKey: ['traffic-count'],
    queryFn: fetchTrafficCount,
    refetchInterval: 1000,
  })
}

export function useClearTraffic() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: clearTraffic,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['traffic'] })
      queryClient.invalidateQueries({ queryKey: ['traffic-count'] })
    },
  })
}
