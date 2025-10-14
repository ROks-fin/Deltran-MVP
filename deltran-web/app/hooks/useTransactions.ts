'use client'

import { useQuery } from '@tanstack/react-query'
import { Transaction, PaymentStatus } from '../types/transaction'
import { CACHE_TIMES } from '../config/limits'

// Mock data с всеми 11 статусами для тестирования анимаций
const MOCK_TRANSACTIONS: Transaction[] = [
  {
    payment_id: 'PAY-2024-001',
    amount: '125000.50',
    currency: 'USD',
    sender_account: 'ACC-US-4521',
    recipient_account: 'ACC-UK-8932',
    status: 'Completed',
    created_at: new Date(Date.now() - 300000).toISOString(),
    reference: 'INV-2024-Q1-045',
    processing_time: 234,
    risk_score: 12,
  },
  {
    payment_id: 'PAY-2024-002',
    amount: '75000.00',
    currency: 'EUR',
    sender_account: 'ACC-EU-1234',
    recipient_account: 'ACC-AE-5678',
    status: 'Settling',
    created_at: new Date(Date.now() - 180000).toISOString(),
    reference: 'WIRE-2024-0892',
    processing_time: 156,
    risk_score: 8,
  },
  {
    payment_id: 'PAY-2024-003',
    amount: '50000.00',
    currency: 'GBP',
    sender_account: 'ACC-UK-9876',
    recipient_account: 'ACC-IN-3456',
    status: 'Settled',
    created_at: new Date(Date.now() - 120000).toISOString(),
    reference: 'TRF-UK-IN-092',
    processing_time: 198,
    risk_score: 5,
  },
  {
    payment_id: 'PAY-2024-004',
    amount: '200000.00',
    currency: 'USD',
    sender_account: 'ACC-US-7654',
    recipient_account: 'ACC-IL-2109',
    status: 'Approved',
    created_at: new Date(Date.now() - 90000).toISOString(),
    reference: 'CORP-PAY-445',
    processing_time: 89,
    risk_score: 15,
  },
  {
    payment_id: 'PAY-2024-005',
    amount: '15000.00',
    currency: 'AED',
    sender_account: 'ACC-AE-1111',
    recipient_account: 'ACC-PK-2222',
    status: 'Queued',
    created_at: new Date(Date.now() - 60000).toISOString(),
    reference: 'REG-AE-PK-12',
    risk_score: 20,
  },
  {
    payment_id: 'PAY-2024-006',
    amount: '95000.00',
    currency: 'EUR',
    sender_account: 'ACC-EU-5555',
    recipient_account: 'ACC-US-6666',
    status: 'Validated',
    created_at: new Date(Date.now() - 45000).toISOString(),
    reference: 'SWIFT-EU-US-78',
    risk_score: 7,
  },
  {
    payment_id: 'PAY-2024-007',
    amount: '32000.00',
    currency: 'INR',
    sender_account: 'ACC-IN-8888',
    recipient_account: 'ACC-UK-9999',
    status: 'Screened',
    created_at: new Date(Date.now() - 30000).toISOString(),
    reference: 'COMP-CHK-341',
    risk_score: 18,
  },
  {
    payment_id: 'PAY-2024-008',
    amount: '8500.00',
    currency: 'USD',
    sender_account: 'ACC-US-3333',
    recipient_account: 'ACC-EU-4444',
    status: 'Initiated',
    created_at: new Date(Date.now() - 15000).toISOString(),
    reference: 'NEW-TRANS-89',
    risk_score: 10,
  },
  {
    payment_id: 'PAY-2024-009',
    amount: '450000.00',
    currency: 'GBP',
    sender_account: 'ACC-UK-7777',
    recipient_account: 'ACC-AE-8888',
    status: 'Failed',
    created_at: new Date(Date.now() - 240000).toISOString(),
    reference: 'FAIL-UK-AE-02',
    processing_time: 345,
    risk_score: 95,
  },
  {
    payment_id: 'PAY-2024-010',
    amount: '12500.00',
    currency: 'PKR',
    sender_account: 'ACC-PK-1234',
    recipient_account: 'ACC-IN-5678',
    status: 'Rejected',
    created_at: new Date(Date.now() - 210000).toISOString(),
    reference: 'SANC-CHK-FAIL',
    risk_score: 88,
  },
  {
    payment_id: 'PAY-2024-011',
    amount: '67000.00',
    currency: 'ILS',
    sender_account: 'ACC-IL-9999',
    recipient_account: 'ACC-US-1111',
    status: 'Cancelled',
    created_at: new Date(Date.now() - 420000).toISOString(),
    reference: 'USER-CANC-78',
    risk_score: 0,
  },
]

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

// Extract country corridor from BIC codes (first 2 letters after bank code)
function extractCorridor(from_bank: string, to_bank: string): string {
  try {
    // BIC format: AAAABBCCXXX (A=bank, B=country, C=location, X=branch)
    const fromCountry = from_bank.slice(4, 6)
    const toCountry = to_bank.slice(4, 6)
    return `${fromCountry}-${toCountry}`
  } catch {
    return 'Unknown'
  }
}

export function useTransactions() {
  return useQuery({
    queryKey: ['transactions'],
    queryFn: async () => {
      try {
        // Use Next.js API route (no CORS issues)
        let response = await fetch('/api/payments')

        if (!response.ok) {
          // Fallback to legacy endpoint
          response = await fetch('/api/v1/transactions/recent')
        }

        if (response.ok) {
          const rawData = await response.json()

          // Handle both formats: { payments: [...] } or direct array
          const payments = rawData.payments || rawData

          if (Array.isArray(payments) && payments.length > 0) {
            // Transform backend data to frontend format
            return payments.map((tx: any) => ({
              payment_id: tx.payment_id || tx.id,
              amount: tx.amount?.toString() || '0',
              currency: tx.currency || 'USD',
              sender_account: tx.sender_bank_id || tx.from_bank || tx.sender_account || 'Unknown',
              recipient_account: tx.receiver_bank_id || tx.to_bank || tx.recipient_account || 'Unknown',
              status: normalizeStatus(tx.status || 'Initiated'),
              created_at: tx.created_at || tx.timestamp || new Date().toISOString(),
              reference: tx.reference || tx.payment_id || tx.id,
              processing_time: tx.processing_time_ms || undefined,
              risk_score: tx.risk_score || 0,
              corridor: extractCorridor(
                tx.sender_bank_id || tx.from_bank || '',
                tx.receiver_bank_id || tx.to_bank || ''
              ),
            })).sort(
              (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
            ) as Transaction[]
          }
        }
      } catch (error) {
        console.error('Failed to fetch transactions from API:', error)
      }

      // Fallback to mock data when API is unavailable
      console.log('Using mock transaction data - API not available')
      return MOCK_TRANSACTIONS.sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      )
    },
    refetchInterval: CACHE_TIMES.TRANSACTIONS * 1000, // 30 seconds
    staleTime: CACHE_TIMES.TRANSACTIONS * 1000,
    retry: 3,
    retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 10000),
  })
}
