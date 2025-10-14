'use client'

import { useQuery } from '@tanstack/react-query'
import { api } from '../lib/api-client'
import { CACHE_TIMES } from '../config/limits'

export interface SystemMetrics {
  totalVolume: number
  totalVolumeFormatted: string
  activePayments: number
  settlementRate: number
  avgProcessingTime: number
  changeVolume: string
  changePayments: string
  changeSettlement: string
  changeProcessing: string
}

export function useSystemMetrics() {
  return useQuery({
    queryKey: ['system-metrics'],
    queryFn: async (): Promise<SystemMetrics> => {
      // Fetch payments to calculate real metrics
      const paymentsResponse = await api.getPayments()
      const payments = paymentsResponse.data.payments || []

      // Calculate total volume
      const totalVolume = payments.reduce((sum, p) => sum + (p.amount || 0), 0)

      // Count active payments (not completed/failed/rejected/cancelled)
      const activeStatuses = ['Initiated', 'Validated', 'Screened', 'Approved', 'Queued', 'Settling']
      const activePayments = payments.filter(p =>
        activeStatuses.includes(p.status)
      ).length

      // Calculate settlement rate
      const completedPayments = payments.filter(p =>
        ['Completed', 'Settled'].includes(p.status)
      ).length
      const settlementRate = payments.length > 0
        ? (completedPayments / payments.length) * 100
        : 0

      // Calculate average processing time from completed payments
      const completedWithTime = payments.filter(p =>
        p.created_at && ['Completed', 'Settled'].includes(p.status)
      )
      const avgProcessingTime = completedWithTime.length > 0
        ? completedWithTime.reduce((sum, p) => {
            const created = new Date(p.created_at!).getTime()
            const now = p.updated_at ? new Date(p.updated_at).getTime() : Date.now()
            return sum + (now - created)
          }, 0) / completedWithTime.length
        : 0

      return {
        totalVolume,
        totalVolumeFormatted: formatCurrency(totalVolume),
        activePayments,
        settlementRate: Math.round(settlementRate * 10) / 10,
        avgProcessingTime: Math.round(avgProcessingTime),
        changeVolume: payments.length > 0 ? 'Live' : 'N/A',
        changePayments: payments.length > 0 ? 'Live' : 'N/A',
        changeSettlement: payments.length > 0 ? 'Live' : 'N/A',
        changeProcessing: payments.length > 0 ? 'Live' : 'N/A',
      }
    },
    refetchInterval: CACHE_TIMES.METRICS * 1000, // 2 seconds
    staleTime: CACHE_TIMES.METRICS * 1000,
    retry: 2,
  })
}

function formatCurrency(amount: number): string {
  if (amount >= 1_000_000) {
    return `$${(amount / 1_000_000).toFixed(1)}M`
  } else if (amount >= 1_000) {
    return `$${(amount / 1_000).toFixed(1)}K`
  }
  return `$${amount.toFixed(2)}`
}
