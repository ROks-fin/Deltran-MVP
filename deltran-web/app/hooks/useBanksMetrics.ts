'use client'

import { useQuery } from '@tanstack/react-query'

export interface BankMetric {
  bank_id: string
  bank_name: string
  bic: string
  country: string
  sent_count: number
  received_count: number
  sent_volume: number
  received_volume: number
  status: 'active' | 'inactive'
  last_activity?: string
}

export function useBanksMetrics() {
  return useQuery<BankMetric[]>({
    queryKey: ['banks-metrics'],
    queryFn: async () => {
      const response = await fetch('/api/v1/metrics/banks')
      if (!response.ok) {
        throw new Error('Failed to fetch banks metrics')
      }

      const data = await response.json()
      return data.banks || []
    },
    staleTime: 30000, // 30 seconds
    refetchInterval: 60000, // 1 minute
  })
}
