'use client'

import { useQuery } from '@tanstack/react-query'
import { api, LiveMetrics } from '../lib/api-client'
import { CACHE_TIMES } from '../config/limits'

export interface MetricsData extends LiveMetrics {
  success_rate: number
}

export function useMetrics() {
  return useQuery({
    queryKey: ['metrics'],
    queryFn: async () => {
      try {
        const { data } = await api.getMetrics()

        // Calculate success rate
        const success_rate = data.total_payments > 0
          ? (data.successful_payments / data.total_payments) * 100
          : 100

        return {
          ...data,
          success_rate,
        } as MetricsData
      } catch (error) {
        console.error('Failed to fetch metrics:', error)
        // Return mock data in development
        if (process.env.NODE_ENV === 'development') {
          return {
            tps: 850,
            latency_p95_ms: 75,
            error_rate: 0.5,
            total_payments: 15420,
            successful_payments: 15343,
            failed_payments: 77,
            queue_depth: 234,
            success_rate: 99.5,
            last_update: new Date().toISOString(),
          } as MetricsData
        }
        throw error
      }
    },
    refetchInterval: CACHE_TIMES.METRICS * 1000, // 2 seconds
    staleTime: CACHE_TIMES.METRICS * 1000,
    retry: 3,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 10000),
  })
}
