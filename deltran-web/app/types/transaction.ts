// Transaction types based on ledger-core/src/types.rs

export type PaymentStatus =
  | 'Initiated'     // 1 - Серый, мигающая точка
  | 'Validated'     // 2 - Желтый, галочка с bounce
  | 'Screened'      // 3 - Желтый, сканирующая линия
  | 'Approved'      // 4 - Голубой, золотая звезда
  | 'Queued'        // 5 - Голубой, три точки
  | 'Settling'      // 6 - Оранжевый, вращающиеся стрелки
  | 'Settled'       // 7 - Зеленый, checkmark с glow
  | 'Completed'     // 8 - Зеленый, конфетти
  | 'Rejected'      // 9 - Красный, X с shake
  | 'Failed'        // 10 - Красный, молния
  | 'Cancelled'     // 11 - Серый, перечеркнуто

export type Currency = 'USD' | 'EUR' | 'GBP' | 'AED' | 'INR' | 'PKR' | 'ILS'

export interface Transaction {
  payment_id: string
  amount: string  // Decimal в Rust = string в JS
  currency: Currency
  sender_account: string
  recipient_account: string
  status: PaymentStatus
  created_at: string  // ISO timestamp
  reference: string
  processing_time?: number  // в миллисекундах
  risk_score?: number  // 0-100
}

// Status color mapping
export const STATUS_COLORS: Record<PaymentStatus, string> = {
  Initiated: '#666666',
  Validated: '#d4af37',
  Screened: '#f4cf57',
  Approved: '#3b82f6',
  Queued: '#06b6d4',
  Settling: '#f59e0b',
  Settled: '#10b981',
  Completed: '#4ade80',
  Rejected: '#f87171',
  Failed: '#ef4444',
  Cancelled: '#6b7280',
}
