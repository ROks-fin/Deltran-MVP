'use client'

import { useQuery } from '@tanstack/react-query'
import { Transaction, PaymentStatus } from '../types/transaction'
import { CACHE_TIMES } from '../config/limits'
import { FilterParams } from '../components/filters/AdvancedFilters'

export function useFilteredTransactions(filters: FilterParams) {
  return useQuery({
    queryKey: ['payments', filters],
    queryFn: async () => {
      // Build query params
      const params = new URLSearchParams()

      if (filters.status) params.append('status', filters.status.toLowerCase())
      if (filters.currency) params.append('currency', filters.currency)
      if (filters.senderBIC) params.append('sender_bic', filters.senderBIC)
      if (filters.receiverBIC) params.append('receiver_bic', filters.receiverBIC)
      if (filters.dateFrom) params.append('date_from', filters.dateFrom)
      if (filters.dateTo) params.append('date_to', filters.dateTo)
      if (filters.minAmount) params.append('min_amount', filters.minAmount)
      if (filters.maxAmount) params.append('max_amount', filters.maxAmount)
      if (filters.reference) params.append('reference', filters.reference)

      const queryString = params.toString()
      const url = `/api/v1/payments${queryString ? `?${queryString}` : ''}`

      try {
        const response = await fetch(url)

        if (response.ok) {
          const data = await response.json()

          // Handle both formats: { payments: [...] } or direct array
          const payments = data.payments || data

          if (Array.isArray(payments) && payments.length > 0) {
            // Transform backend data to frontend format
            return payments.map((tx: any) => ({
              payment_id: tx.payment_reference || tx.id,
              amount: tx.amount?.toString() || '0',
              currency: tx.currency || 'USD',
              sender_account: tx.sender_bic || tx.sender_name || 'Unknown',
              recipient_account: tx.receiver_bic || tx.receiver_name || 'Unknown',
              status: normalizeStatus(tx.status || 'Initiated'),
              created_at: tx.created_at || new Date().toISOString(),
              reference: tx.payment_reference || tx.id,
              processing_time: tx.processing_time_ms || undefined,
              risk_score: tx.risk_score || 0,
            })).sort(
              (a: Transaction, b: Transaction) =>
                new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
            ) as Transaction[]
          }

          // Empty result
          return []
        }
      } catch (error) {
        console.error('Failed to fetch filtered payments:', error)
      }

      // Fallback to empty array on error
      return []
    },
    refetchInterval: CACHE_TIMES.TRANSACTIONS * 1000,
    staleTime: CACHE_TIMES.TRANSACTIONS * 1000,
    retry: 2,
  })
}

// Convert lowercase status to PascalCase for StatusBadge
function normalizeStatus(status: string): PaymentStatus {
  const statusMap: Record<string, PaymentStatus> = {
    'initiated': 'Initiated',
    'validated': 'Validated',
    'screened': 'Screened',
    'approved': 'Approved',
    'queued': 'Queued',
    'settling': 'Settling',
    'settled': 'Settled',
    'completed': 'Completed',
    'rejected': 'Rejected',
    'failed': 'Failed',
    'cancelled': 'Cancelled',
  }

  return statusMap[status.toLowerCase()] || status as PaymentStatus
}
