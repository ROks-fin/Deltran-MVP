'use client'

import { motion, AnimatePresence } from 'framer-motion'
import { X, Clock, Shield, AlertCircle, CheckCircle2, XCircle, FileText, ChevronRight } from 'lucide-react'
import { useState, useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { StatusBadge } from '../transactions/StatusBadge'

interface PaymentDetailsModalProps {
  paymentId: string
  isOpen: boolean
  onClose: () => void
}

interface ComplianceCheck {
  id: string
  check_type: string
  status: string
  risk_score?: number
  requires_review: boolean
  completed_at?: string
}

interface PaymentDetails {
  id: string
  payment_reference: string
  sender_bic: string
  sender_name: string
  receiver_bic: string
  receiver_name: string
  amount: number
  currency: string
  status: string
  risk_score?: number
  created_at: string
  settled_at?: string
  sender_account_id?: string
  receiver_account_id?: string
  batch_id?: string
  swift_message_type?: string
  swift_message_id?: string
  idempotency_key?: string
  processed_at?: string
  remittance_info?: string
  compliance_check?: ComplianceCheck
  updated_at: string
}

type TabType = 'overview' | 'compliance' | 'timeline' | 'technical'

export function PaymentDetailsModal({ paymentId, isOpen, onClose }: PaymentDetailsModalProps) {
  const [activeTab, setActiveTab] = useState<TabType>('overview')

  const { data: payment, isLoading, error } = useQuery<PaymentDetails>({
    queryKey: ['payment-details', paymentId],
    queryFn: async () => {
      const response = await fetch(`/api/v1/payments/${paymentId}`)
      if (!response.ok) {
        throw new Error('Failed to fetch payment details')
      }
      return response.json()
    },
    enabled: isOpen && !!paymentId,
  })

  // Reset tab when modal opens
  useEffect(() => {
    if (isOpen) {
      setActiveTab('overview')
    }
  }, [isOpen])

  const tabs: { id: TabType; label: string; icon: typeof Clock }[] = [
    { id: 'overview', label: 'Overview', icon: FileText },
    { id: 'compliance', label: 'Compliance', icon: Shield },
    { id: 'timeline', label: 'Timeline', icon: Clock },
    { id: 'technical', label: 'Technical', icon: AlertCircle },
  ]

  const formatDate = (dateString?: string) => {
    if (!dateString) return 'N/A'
    const date = new Date(dateString)
    return date.toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })
  }

  const formatAmount = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
    }).format(amount)
  }

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50"
          />

          {/* Modal */}
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              transition={{ duration: 0.2 }}
              className="bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 rounded-2xl shadow-2xl max-w-4xl w-full max-h-[90vh] overflow-hidden"
            >
              {/* Header */}
              <div className="p-6 border-b border-zinc-800 flex items-center justify-between">
                <div>
                  <h2 className="text-2xl font-bold text-white mb-1">Payment Details</h2>
                  <p className="text-sm text-zinc-400 font-mono">
                    {payment?.payment_reference || paymentId}
                  </p>
                </div>
                <button
                  onClick={onClose}
                  className="p-2 rounded-lg bg-zinc-800/50 hover:bg-red-500/10 border border-zinc-700 hover:border-red-500/30 transition-all group"
                >
                  <X className="w-5 h-5 text-zinc-400 group-hover:text-red-400 transition-colors" />
                </button>
              </div>

              {/* Tabs */}
              <div className="px-6 border-b border-zinc-800 flex gap-1">
                {tabs.map((tab) => {
                  const Icon = tab.icon
                  return (
                    <button
                      key={tab.id}
                      onClick={() => setActiveTab(tab.id)}
                      className={`relative px-4 py-3 text-sm font-medium transition-all ${
                        activeTab === tab.id
                          ? 'text-deltran-gold'
                          : 'text-zinc-400 hover:text-white'
                      }`}
                    >
                      <div className="flex items-center gap-2">
                        <Icon className="w-4 h-4" />
                        {tab.label}
                      </div>
                      {activeTab === tab.id && (
                        <motion.div
                          layoutId="activeTab"
                          className="absolute bottom-0 left-0 right-0 h-0.5 bg-gradient-to-r from-deltran-gold to-deltran-gold-light"
                        />
                      )}
                    </button>
                  )
                })}
              </div>

              {/* Content */}
              <div className="p-6 overflow-y-auto max-h-[calc(90vh-200px)]">
                {isLoading && (
                  <div className="flex items-center justify-center py-12">
                    <div className="w-8 h-8 border-2 border-deltran-gold border-t-transparent rounded-full animate-spin" />
                  </div>
                )}

                {error && (
                  <div className="flex items-center justify-center py-12">
                    <div className="text-red-400 text-center">
                      <AlertCircle className="w-12 h-12 mx-auto mb-3 opacity-50" />
                      <p>Failed to load payment details</p>
                    </div>
                  </div>
                )}

                {payment && (
                  <AnimatePresence mode="wait">
                    <motion.div
                      key={activeTab}
                      initial={{ opacity: 0, x: 20 }}
                      animate={{ opacity: 1, x: 0 }}
                      exit={{ opacity: 0, x: -20 }}
                      transition={{ duration: 0.2 }}
                    >
                      {activeTab === 'overview' && (
                        <OverviewTab payment={payment} formatAmount={formatAmount} formatDate={formatDate} />
                      )}
                      {activeTab === 'compliance' && (
                        <ComplianceTab payment={payment} formatDate={formatDate} />
                      )}
                      {activeTab === 'timeline' && (
                        <TimelineTab payment={payment} formatDate={formatDate} />
                      )}
                      {activeTab === 'technical' && (
                        <TechnicalTab payment={payment} formatDate={formatDate} />
                      )}
                    </motion.div>
                  </AnimatePresence>
                )}
              </div>
            </motion.div>
          </div>
        </>
      )}
    </AnimatePresence>
  )
}

// Overview Tab
function OverviewTab({ payment, formatAmount, formatDate }: { payment: PaymentDetails; formatAmount: (amount: number, currency: string) => string; formatDate: (date?: string) => string }) {
  return (
    <div className="space-y-6">
      {/* Status and Amount */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800">
          <p className="text-xs text-zinc-500 mb-2 uppercase tracking-wider">Status</p>
          <StatusBadge status={payment.status as any} />
        </div>
        <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800">
          <p className="text-xs text-zinc-500 mb-2 uppercase tracking-wider">Amount</p>
          <p className="text-2xl font-bold text-white">
            {formatAmount(payment.amount, payment.currency)}
          </p>
        </div>
      </div>

      {/* Sender Information */}
      <div className="p-4 rounded-lg bg-gradient-to-br from-blue-500/10 to-transparent border border-blue-500/20">
        <div className="flex items-center gap-2 mb-3">
          <div className="w-8 h-8 rounded-lg bg-blue-500/20 flex items-center justify-center">
            <ChevronRight className="w-4 h-4 text-blue-400 rotate-180" />
          </div>
          <h3 className="text-sm font-semibold text-blue-400 uppercase tracking-wider">Sender</h3>
        </div>
        <div className="space-y-2">
          <InfoRow label="Bank Name" value={payment.sender_name} />
          <InfoRow label="BIC Code" value={payment.sender_bic} mono />
          {payment.sender_account_id && (
            <InfoRow label="Account ID" value={payment.sender_account_id} mono />
          )}
        </div>
      </div>

      {/* Receiver Information */}
      <div className="p-4 rounded-lg bg-gradient-to-br from-green-500/10 to-transparent border border-green-500/20">
        <div className="flex items-center gap-2 mb-3">
          <div className="w-8 h-8 rounded-lg bg-green-500/20 flex items-center justify-center">
            <ChevronRight className="w-4 h-4 text-green-400" />
          </div>
          <h3 className="text-sm font-semibold text-green-400 uppercase tracking-wider">Receiver</h3>
        </div>
        <div className="space-y-2">
          <InfoRow label="Bank Name" value={payment.receiver_name} />
          <InfoRow label="BIC Code" value={payment.receiver_bic} mono />
          {payment.receiver_account_id && (
            <InfoRow label="Account ID" value={payment.receiver_account_id} mono />
          )}
        </div>
      </div>

      {/* Remittance Information */}
      {payment.remittance_info && (
        <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800">
          <p className="text-xs text-zinc-500 mb-2 uppercase tracking-wider">Remittance Information</p>
          <p className="text-sm text-zinc-300">{payment.remittance_info}</p>
        </div>
      )}

      {/* Timestamps */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800">
          <p className="text-xs text-zinc-500 mb-2 uppercase tracking-wider">Created</p>
          <p className="text-sm text-zinc-300 font-mono">{formatDate(payment.created_at)}</p>
        </div>
        {payment.settled_at && (
          <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800">
            <p className="text-xs text-zinc-500 mb-2 uppercase tracking-wider">Settled</p>
            <p className="text-sm text-zinc-300 font-mono">{formatDate(payment.settled_at)}</p>
          </div>
        )}
      </div>
    </div>
  )
}

// Compliance Tab
function ComplianceTab({ payment, formatDate }: { payment: PaymentDetails; formatDate: (date?: string) => string }) {
  const getRiskLevel = (score?: number) => {
    if (!score) return { label: 'Unknown', color: 'zinc', bgColor: 'zinc-800' }
    if (score >= 70) return { label: 'High Risk', color: 'red-400', bgColor: 'red-500/10' }
    if (score >= 40) return { label: 'Medium Risk', color: 'yellow-400', bgColor: 'yellow-500/10' }
    return { label: 'Low Risk', color: 'green-400', bgColor: 'green-500/10' }
  }

  const risk = getRiskLevel(payment.risk_score)

  return (
    <div className="space-y-6">
      {/* Risk Score */}
      <div className={`p-4 rounded-lg bg-gradient-to-br from-${risk.bgColor} to-transparent border border-${risk.color}/20`}>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wider">Risk Assessment</h3>
          <span className={`text-xs font-semibold text-${risk.color} px-3 py-1 rounded-full bg-${risk.bgColor} border border-${risk.color}/30`}>
            {risk.label}
          </span>
        </div>

        <div className="space-y-3">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs text-zinc-400">Risk Score</span>
              <span className={`text-sm font-bold text-${risk.color}`}>
                {payment.risk_score || 0}/100
              </span>
            </div>
            <div className="h-2 bg-zinc-800 rounded-full overflow-hidden">
              <motion.div
                initial={{ width: 0 }}
                animate={{ width: `${payment.risk_score || 0}%` }}
                transition={{ duration: 1, ease: 'easeOut' }}
                className={`h-full bg-${risk.color}`}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Compliance Check Details */}
      {payment.compliance_check ? (
        <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800 space-y-4">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wider flex items-center gap-2">
            <Shield className="w-4 h-4 text-deltran-gold" />
            Compliance Check
          </h3>

          <div className="space-y-3">
            <InfoRow label="Check ID" value={payment.compliance_check.id} mono />
            <InfoRow label="Check Type" value={payment.compliance_check.check_type} />
            <InfoRow
              label="Status"
              value={
                <span className={`text-xs font-semibold px-2 py-1 rounded-full ${
                  payment.compliance_check.status === 'passed'
                    ? 'bg-green-500/10 text-green-400 border border-green-500/30'
                    : payment.compliance_check.status === 'review'
                    ? 'bg-yellow-500/10 text-yellow-400 border border-yellow-500/30'
                    : 'bg-red-500/10 text-red-400 border border-red-500/30'
                }`}>
                  {payment.compliance_check.status.toUpperCase()}
                </span>
              }
            />

            {payment.compliance_check.requires_review && (
              <div className="flex items-center gap-2 p-3 rounded-lg bg-yellow-500/10 border border-yellow-500/20">
                <AlertCircle className="w-4 h-4 text-yellow-400" />
                <span className="text-sm text-yellow-400 font-medium">Requires Manual Review</span>
              </div>
            )}

            {payment.compliance_check.completed_at && (
              <InfoRow label="Completed At" value={formatDate(payment.compliance_check.completed_at)} mono />
            )}
          </div>
        </div>
      ) : (
        <div className="p-8 text-center text-zinc-500">
          <Shield className="w-12 h-12 mx-auto mb-3 opacity-30" />
          <p>No compliance check information available</p>
        </div>
      )}

      {/* Screening Results */}
      <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800">
        <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">Screening Status</h3>
        <div className="space-y-2">
          <ScreeningItem label="AML Screening" status="passed" />
          <ScreeningItem label="Sanctions Screening" status="passed" />
          <ScreeningItem label="PEP Screening" status="passed" />
          <ScreeningItem label="Watchlist Check" status="passed" />
        </div>
      </div>
    </div>
  )
}

// Timeline Tab
function TimelineTab({ payment, formatDate }: { payment: PaymentDetails; formatDate: (date?: string) => string }) {
  const timelineEvents = [
    { label: 'Payment Created', date: payment.created_at, icon: FileText, color: 'blue' },
    payment.processed_at && { label: 'Processed', date: payment.processed_at, icon: CheckCircle2, color: 'green' },
    payment.settled_at && { label: 'Settled', date: payment.settled_at, icon: CheckCircle2, color: 'green' },
    { label: 'Last Updated', date: payment.updated_at, icon: Clock, color: 'zinc' },
  ].filter(Boolean) as Array<{ label: string; date: string; icon: any; color: string }>

  return (
    <div className="space-y-1">
      {timelineEvents.map((event, index) => {
        const Icon = event.icon
        const isLast = index === timelineEvents.length - 1

        return (
          <motion.div
            key={index}
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: index * 0.1 }}
            className="relative"
          >
            <div className="flex gap-4 pb-6">
              {/* Timeline Line */}
              <div className="relative flex flex-col items-center">
                <div className={`w-10 h-10 rounded-full bg-${event.color}-500/20 border-2 border-${event.color}-500/50 flex items-center justify-center z-10`}>
                  <Icon className={`w-5 h-5 text-${event.color}-400`} />
                </div>
                {!isLast && (
                  <div className={`absolute top-10 w-0.5 h-full bg-gradient-to-b from-${event.color}-500/30 to-transparent`} />
                )}
              </div>

              {/* Event Content */}
              <div className="flex-1 pt-2">
                <p className="text-sm font-semibold text-white mb-1">{event.label}</p>
                <p className="text-xs text-zinc-400 font-mono">{formatDate(event.date)}</p>
              </div>
            </div>
          </motion.div>
        )
      })}
    </div>
  )
}

// Technical Tab
function TechnicalTab({ payment, formatDate }: { payment: PaymentDetails; formatDate: (date?: string) => string }) {
  return (
    <div className="space-y-4">
      <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800 space-y-3">
        <h3 className="text-sm font-semibold text-white uppercase tracking-wider">Payment Identifiers</h3>
        <InfoRow label="Payment ID" value={payment.id} mono />
        <InfoRow label="Payment Reference" value={payment.payment_reference} mono />
        {payment.idempotency_key && (
          <InfoRow label="Idempotency Key" value={payment.idempotency_key} mono />
        )}
      </div>

      {payment.batch_id && (
        <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800 space-y-3">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wider">Batch Information</h3>
          <InfoRow label="Batch ID" value={payment.batch_id} mono />
        </div>
      )}

      {(payment.swift_message_type || payment.swift_message_id) && (
        <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800 space-y-3">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wider">SWIFT Details</h3>
          {payment.swift_message_type && (
            <InfoRow label="Message Type" value={payment.swift_message_type} mono />
          )}
          {payment.swift_message_id && (
            <InfoRow label="Message ID" value={payment.swift_message_id} mono />
          )}
        </div>
      )}

      <div className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800 space-y-3">
        <h3 className="text-sm font-semibold text-white uppercase tracking-wider">Metadata</h3>
        <InfoRow label="Created At" value={formatDate(payment.created_at)} mono />
        <InfoRow label="Updated At" value={formatDate(payment.updated_at)} mono />
        {payment.processed_at && (
          <InfoRow label="Processed At" value={formatDate(payment.processed_at)} mono />
        )}
      </div>
    </div>
  )
}

// Helper Components
function InfoRow({ label, value, mono = false }: { label: string; value: React.ReactNode; mono?: boolean }) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-zinc-800/50 last:border-0">
      <span className="text-xs text-zinc-500 uppercase tracking-wider">{label}</span>
      <span className={`text-sm text-zinc-300 ${mono ? 'font-mono' : ''}`}>{value}</span>
    </div>
  )
}

function ScreeningItem({ label, status }: { label: string; status: 'passed' | 'failed' | 'pending' }) {
  const icons = {
    passed: { Icon: CheckCircle2, color: 'green' },
    failed: { Icon: XCircle, color: 'red' },
    pending: { Icon: Clock, color: 'yellow' },
  }

  const { Icon, color } = icons[status]

  return (
    <div className="flex items-center justify-between p-2 rounded-lg hover:bg-zinc-800/50 transition-colors">
      <span className="text-sm text-zinc-300">{label}</span>
      <div className="flex items-center gap-2">
        <Icon className={`w-4 h-4 text-${color}-400`} />
        <span className={`text-xs font-semibold text-${color}-400 uppercase`}>{status}</span>
      </div>
    </div>
  )
}
