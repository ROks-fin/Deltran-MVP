'use client'

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'

export interface ComplianceMatch {
  matched_name: string
  match_score: number
  source: 'OFAC' | 'EU' | 'UN' | 'UK'
  matched_field: string
  fuzzy_match: boolean
}

export interface ComplianceReview {
  id: string
  payment_id: string
  payment_reference: string
  sender_name: string
  sender_bic: string
  receiver_name: string
  receiver_bic: string
  amount: number
  currency: string
  risk_level: 'HIGH' | 'MEDIUM' | 'LOW'
  match_score: number
  check_type: string
  status: 'pending' | 'review' | 'approved' | 'rejected'
  requires_review: boolean
  created_at: string
  matches?: ComplianceMatch[]
}

export interface ComplianceFilters {
  status?: string
  risk_level?: string
  page?: number
  page_size?: number
}

export interface ComplianceDecision {
  decision: 'approve' | 'reject' | 'escalate'
  notes: string
}

export function useComplianceReviews(filters: ComplianceFilters = {}) {
  return useQuery<ComplianceReview[]>({
    queryKey: ['compliance-reviews', filters],
    queryFn: async () => {
      const params = new URLSearchParams()

      if (filters.status) params.append('status', filters.status)
      if (filters.risk_level) params.append('risk_level', filters.risk_level)
      if (filters.page) params.append('page', filters.page.toString())
      if (filters.page_size) params.append('page_size', filters.page_size.toString())

      const queryString = params.toString()
      const url = `/api/v1/compliance/reviews${queryString ? `?${queryString}` : ''}`

      const response = await fetch(url)
      if (!response.ok) {
        throw new Error('Failed to fetch compliance reviews')
      }

      const data = await response.json()
      return data.reviews || []
    },
    refetchInterval: 10000, // Refetch every 10 seconds
    staleTime: 5000,
  })
}

export function useSubmitComplianceDecision() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ reviewId, decision }: { reviewId: string; decision: ComplianceDecision }) => {
      const response = await fetch(`/api/v1/compliance/reviews/${reviewId}/decision`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(decision),
      })

      if (!response.ok) {
        throw new Error('Failed to submit decision')
      }

      return response.json()
    },
    onSuccess: () => {
      // Invalidate and refetch compliance reviews
      queryClient.invalidateQueries({ queryKey: ['compliance-reviews'] })
    },
  })
}

export function useComplianceStats() {
  return useQuery({
    queryKey: ['compliance-stats'],
    queryFn: async () => {
      const response = await fetch('/api/v1/compliance/stats')
      if (!response.ok) {
        throw new Error('Failed to fetch compliance stats')
      }

      const data = await response.json()
      return {
        pendingReviews: data.pending_reviews || 0,
        highRiskCount: data.high_risk_count || 0,
        reviewedToday: data.reviewed_today || 0,
      }
    },
    refetchInterval: 10000,
    staleTime: 5000,
  })
}
