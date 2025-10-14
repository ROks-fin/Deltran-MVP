'use client'

import { useQuery } from '@tanstack/react-query'

export interface DailyMetric {
  date: string
  payment_count: number
  total_volume: number
  volume_usd: number
  volume_eur: number
  volume_gbp: number
  settled: number
  pending: number
  failed: number
  success_rate: number
}

export function useDailyMetrics(days: number = 30) {
  return useQuery<DailyMetric[]>({
    queryKey: ['daily-metrics', days],
    queryFn: async () => {
      const response = await fetch(`/api/v1/metrics/daily?days=${days}`)
      if (!response.ok) {
        throw new Error('Failed to fetch daily metrics')
      }

      const data = await response.json()
      return data.metrics || []
    },
    staleTime: 60000, // 1 minute
    refetchInterval: 120000, // 2 minutes
  })
}
