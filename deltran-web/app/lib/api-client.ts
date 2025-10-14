import axios, { AxiosError, InternalAxiosRequestConfig } from 'axios'
import { API_CONFIG } from '../config/limits'

// Generate UUID v4
function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0
    const v = c === 'x' ? r : (r & 0x3) | 0x8
    return v.toString(16)
  })
}

export const apiClient = axios.create({
  baseURL: API_CONFIG.BASE_URL,
  timeout: API_CONFIG.TIMEOUT,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Request interceptor - add mandatory headers
apiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    // Mandatory headers for all requests
    config.headers['X-Tenant-ID'] = API_CONFIG.TENANT_ID
    config.headers['X-Correlation-ID'] = generateUUID()

    // Add Idempotency-Key for POST/PUT/PATCH requests
    if (['post', 'put', 'patch'].includes(config.method?.toLowerCase() || '')) {
      config.headers['Idempotency-Key'] = generateUUID()
    }

    // Add Authorization if available
    if (typeof window !== 'undefined') {
      const authTokens = localStorage.getItem('auth_tokens')
      if (authTokens) {
        try {
          const { accessToken } = JSON.parse(authTokens)
          if (accessToken) {
            config.headers['Authorization'] = `Bearer ${accessToken}`
          }
        } catch (e) {
          console.error('Failed to parse auth tokens:', e)
        }
      }
    }

    return config
  },
  (error) => {
    return Promise.reject(error)
  }
)

// Response interceptor - handle errors and retries
apiClient.interceptors.response.use(
  (response) => response,
  async (error: AxiosError) => {
    const originalRequest = error.config as InternalAxiosRequestConfig & { _retry?: boolean; _retryCount?: number }

    // Handle 429 Rate Limit with retry
    if (error.response?.status === 429 && originalRequest && !originalRequest._retry) {
      originalRequest._retry = true
      originalRequest._retryCount = (originalRequest._retryCount || 0) + 1

      // Exponential backoff: min(1000 * 2^attempt, 30000)
      const delay = Math.min(1000 * Math.pow(2, originalRequest._retryCount), 30000)

      await new Promise((resolve) => setTimeout(resolve, delay))

      return apiClient(originalRequest)
    }

    // Handle 401 Unauthorized - attempt token refresh
    if (error.response?.status === 401 && originalRequest && !originalRequest._retry) {
      originalRequest._retry = true

      try {
        // Try to refresh token
        const { AuthService } = await import('../services/auth.service')
        await AuthService.refreshAccessToken()
        return apiClient(originalRequest)
      } catch (refreshError) {
        // Refresh failed, redirect to login
        if (typeof window !== 'undefined') {
          window.location.href = '/login'
        }
        return Promise.reject(refreshError)
      }
    }

    return Promise.reject(error)
  }
)

// Response types
export interface LiveMetrics {
  tps: number
  latency_p95_ms: number
  error_rate: number
  total_payments: number
  successful_payments: number
  failed_payments: number
  queue_depth: number
  last_update: string
}

export interface RecentTransaction {
  payment_id: string
  amount: string
  currency: string
  from_bank: string
  to_bank: string
  status: string
  timestamp: string
}

// Additional response types
export interface Bank {
  bank_id: string
  name: string
  swift_code: string
  country: string
  primary_currency: string
  status?: string
}

export interface Payment {
  payment_id: string
  sender_bank_id: string
  receiver_bank_id: string
  amount: number
  currency: string
  sender_account: string
  receiver_account: string
  reference: string
  sender_name: string
  receiver_name: string
  status: string
  created_at?: string
  updated_at?: string
}

export interface Batch {
  batch_id: string
  currency: string
  total_amount: number
  payment_count: number
  status: string
  created_at: string
  settled_at?: string
}

export interface ComplianceLimit {
  bank_id: string
  limit_type: string
  amount: number
  currency: string
  used_amount?: number
  percentage_used?: number
}

export interface RiskAssessment {
  risk_score: number
  risk_level: string
  factors: string[]
  approved: boolean
}

export interface NettingStatus {
  active_cycles: number
  pending_settlements: number
  total_netted_today: number
  currencies: string[]
}

export interface ReconciliationStatus {
  total_reconciled: number
  pending_reconciliation: number
  discrepancies: number
  last_reconciliation: string
}

// API methods
export const api = {
  // Health & Metrics
  getMetrics: () => apiClient.get<LiveMetrics>('/api/v1/metrics/live'),
  getHealth: () => apiClient.get('/health'),

  // Banks
  getBanks: () => apiClient.get<{ banks: Bank[] }>('/api/banks'),
  registerBank: (data: Omit<Bank, 'status'>) => apiClient.post('/api/banks/register', data),
  getBankDetails: (bankId: string) => apiClient.get<Bank>(`/api/banks/${bankId}`),

  // Payments
  getTransactions: () => apiClient.get<RecentTransaction[]>('/api/v1/transactions/recent'),
  getPayments: () => apiClient.get<{ payments: Payment[] }>('/api/payments'),
  createPayment: (data: Omit<Payment, 'payment_id' | 'status' | 'created_at' | 'updated_at'>) =>
    apiClient.post('/api/payments', data),
  getPaymentDetails: (paymentId: string) => apiClient.get<Payment>(`/api/payments/${paymentId}`),

  // Batches & Settlement
  getBatches: () => apiClient.get<{ batches: Batch[] }>('/api/batches'),
  createBatch: () => apiClient.post('/api/batches/create'),
  getBatchDetails: (batchId: string) => apiClient.get<Batch>(`/api/batches/${batchId}`),
  executeSettlement: (data: { batch_id: string; currency: string; amount: number; participants: string[] }) =>
    apiClient.post('/api/settlement/execute', data),
  getNettingStatus: () => apiClient.get<NettingStatus>('/api/netting/status'),

  // Compliance
  getComplianceLimits: () => apiClient.get<{ limits: ComplianceLimit[] }>('/api/limits'),
  setComplianceLimit: (data: ComplianceLimit) => apiClient.post('/api/limits/set', data),
  checkCompliance: (data: { bank_id: string; amount: number; currency: string; counterparty: string }) =>
    apiClient.post<{ approved: boolean; reason?: string }>('/api/compliance/check', data),
  getComplianceStatus: () => apiClient.get('/api/compliance/status'),

  // Risk Engine
  assessRisk: (data: {
    bank_id: string
    transaction_amount: number
    currency: string
    counterparty_id: string
    transaction_type: string
  }) => apiClient.post<RiskAssessment>('/api/risk/assess', data),
  getExposure: (bankId: string) => apiClient.get(`/api/risk/exposure/${bankId}`),
  getVaR: (bankId: string) => apiClient.get(`/api/risk/var/${bankId}`),

  // Reconciliation
  getReconciliationStatus: () => apiClient.get<ReconciliationStatus>('/api/reconciliation/status'),
  startReconciliation: (data: { batch_id: string; reconciliation_type: string }) =>
    apiClient.post('/api/reconciliation/start', data),
  getDiscrepancies: () => apiClient.get<{ discrepancies: any[] }>('/api/reconciliation/discrepancies'),

  // Ledger
  createLedgerEntry: (data: {
    account_id: string
    amount: number
    currency: string
    entry_type: string
    reference: string
  }) => apiClient.post('/api/ledger/entry', data),
  getBalance: (accountId: string) => apiClient.get(`/api/ledger/balance/${accountId}`),
  getTransactionHistory: (accountId: string) => apiClient.get(`/api/ledger/history/${accountId}`),

  // Message Bus
  publishMessage: (data: { topic: string; payload: any }) => apiClient.post('/api/messages/publish', data),
  getMessageStats: () => apiClient.get('/api/messages/stats'),
}
