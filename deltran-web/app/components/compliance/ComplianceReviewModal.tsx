'use client'

import { motion, AnimatePresence } from 'framer-motion'
import { X, Shield, AlertTriangle, CheckCircle2, XCircle, AlertOctagon, Loader2 } from 'lucide-react'
import { useState } from 'react'
import toast from 'react-hot-toast'
import { ComplianceReview, useSubmitComplianceDecision } from '@/app/hooks/useComplianceReviews'

interface ComplianceReviewModalProps {
  review: ComplianceReview | null
  isOpen: boolean
  onClose: () => void
}

type DecisionType = 'approve' | 'reject' | 'escalate' | null

export function ComplianceReviewModal({ review, isOpen, onClose }: ComplianceReviewModalProps) {
  const [decision, setDecision] = useState<DecisionType>(null)
  const [notes, setNotes] = useState('')
  const [showConfirmation, setShowConfirmation] = useState(false)

  const submitDecisionMutation = useSubmitComplianceDecision()

  const handleSubmit = async () => {
    if (!decision || !notes || notes.length < 20 || !review) return

    try {
      await submitDecisionMutation.mutateAsync({
        reviewId: review.id,
        decision: {
          decision,
          notes,
        },
      })

      // Show success and close
      setShowConfirmation(true)
      toast.success('Compliance decision submitted successfully!')
      setTimeout(() => {
        handleClose()
      }, 1500)
    } catch (error) {
      console.error('Failed to submit decision:', error)
      toast.error('Failed to submit decision. Please try again.')
    }
  }

  const handleClose = () => {
    setDecision(null)
    setNotes('')
    setShowConfirmation(false)
    onClose()
  }

  const canSubmit = decision && notes.length >= 20 && !submitDecisionMutation.isPending

  if (!review) return null

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={handleClose}
            className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50"
          />

          {/* Modal */}
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              transition={{ duration: 0.2 }}
              className="bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 rounded-2xl shadow-2xl max-w-6xl w-full max-h-[90vh] overflow-hidden"
            >
              {/* Header */}
              <div className="p-6 border-b border-zinc-800 flex items-center justify-between">
                <div>
                  <h2 className="text-2xl font-bold text-white mb-1 flex items-center gap-3">
                    <Shield className="w-7 h-7 text-deltran-gold" />
                    Compliance Review
                  </h2>
                  <p className="text-sm text-zinc-400 font-mono">{review.payment_reference}</p>
                </div>
                <button
                  onClick={handleClose}
                  className="p-2 rounded-lg bg-zinc-800/50 hover:bg-red-500/10 border border-zinc-700 hover:border-red-500/30 transition-all group"
                >
                  <X className="w-5 h-5 text-zinc-400 group-hover:text-red-400 transition-colors" />
                </button>
              </div>

              {/* Content */}
              <div className="flex flex-col lg:flex-row">
                {/* Left Side - Payment & Match Details (60%) */}
                <div className="flex-1 p-6 overflow-y-auto max-h-[calc(90vh-200px)] space-y-6">
                  {/* Payment Information */}
                  <div className="p-4 rounded-xl bg-zinc-800/30 border border-zinc-800">
                    <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">
                      Payment Details
                    </h3>

                    <div className="grid grid-cols-2 gap-4">
                      <InfoCard label="Amount" value={formatCurrency(review.amount, review.currency)} />
                      <InfoCard label="Currency" value={review.currency} />
                      <InfoCard label="Sender BIC" value={review.sender_bic} mono />
                      <InfoCard label="Receiver BIC" value={review.receiver_bic} mono />
                    </div>

                    <div className="mt-4 grid grid-cols-2 gap-4">
                      <div>
                        <p className="text-xs text-zinc-500 mb-1">Sender</p>
                        <p className="text-sm text-zinc-300">{review.sender_name}</p>
                      </div>
                      <div>
                        <p className="text-xs text-zinc-500 mb-1">Receiver</p>
                        <p className="text-sm text-zinc-300">{review.receiver_name}</p>
                      </div>
                    </div>
                  </div>

                  {/* Match Information */}
                  {review.matches && review.matches.length > 0 && (
                    <div className="p-4 rounded-xl bg-gradient-to-br from-red-500/10 to-transparent border border-red-500/30">
                      <div className="flex items-center justify-between mb-4">
                        <h3 className="text-sm font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                          <AlertTriangle className="w-4 h-4 text-red-400" />
                          Sanctions Match Detected
                        </h3>
                        <span className="px-2 py-1 rounded text-xs font-semibold bg-red-500/20 text-red-400 border border-red-500/30">
                          {review.matches.length} Match{review.matches.length > 1 ? 'es' : ''}
                        </span>
                      </div>

                      {review.matches.map((match, idx) => (
                        <div key={idx} className="mb-4 last:mb-0 p-3 rounded-lg bg-zinc-900/50 border border-zinc-800">
                          <div className="space-y-3">
                            <div>
                              <p className="text-xs text-zinc-500 mb-1">Matched Entity Name</p>
                              <p className="text-lg font-bold text-red-400">{match.matched_name}</p>
                            </div>

                            <div className="grid grid-cols-2 gap-3">
                              <div>
                                <p className="text-xs text-zinc-500 mb-1">Match Score</p>
                                <div className="flex items-center gap-2">
                                  <div className="flex-1 h-2 bg-zinc-800 rounded-full overflow-hidden">
                                    <div
                                      className="h-full bg-red-400 transition-all"
                                      style={{ width: `${match.match_score * 100}%` }}
                                    />
                                  </div>
                                  <span className="text-sm font-bold text-red-400">
                                    {Math.round(match.match_score * 100)}%
                                  </span>
                                </div>
                              </div>

                              <div>
                                <p className="text-xs text-zinc-500 mb-1">Match Type</p>
                                <span
                                  className={`inline-block px-2 py-1 rounded text-xs font-semibold ${
                                    match.fuzzy_match
                                      ? 'bg-yellow-500/10 text-yellow-400 border border-yellow-500/30'
                                      : 'bg-red-500/10 text-red-400 border border-red-500/30'
                                  }`}
                                >
                                  {match.fuzzy_match ? 'Fuzzy Match' : 'Exact Match'}
                                </span>
                              </div>
                            </div>

                            <div className="grid grid-cols-2 gap-3">
                              <div>
                                <p className="text-xs text-zinc-500 mb-1">Source List</p>
                                <SourceBadge source={match.source} large />
                              </div>
                              <div>
                                <p className="text-xs text-zinc-500 mb-1">Matched Field</p>
                                <code className="text-xs text-zinc-300 bg-zinc-800 px-2 py-1 rounded">
                                  {match.matched_field}
                                </code>
                              </div>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}

                  {/* Risk Assessment */}
                  <div className="p-4 rounded-xl bg-zinc-800/30 border border-zinc-800">
                    <h3 className="text-sm font-semibold text-white uppercase tracking-wider mb-4">
                      Risk Assessment
                    </h3>

                    <div className="flex items-center justify-between mb-2">
                      <span className="text-xs text-zinc-400">Risk Level</span>
                      <RiskBadge level={review.risk_level} />
                    </div>

                    <div className="flex items-center justify-between">
                      <span className="text-xs text-zinc-400">Overall Match Score</span>
                      <span className="text-sm font-bold text-red-400">
                        {Math.round(review.match_score * 100)}%
                      </span>
                    </div>
                  </div>
                </div>

                {/* Right Side - Decision Panel (40%) */}
                <div className="lg:w-2/5 p-6 border-t lg:border-t-0 lg:border-l border-zinc-800 bg-zinc-900/30">
                  {showConfirmation ? (
                    <motion.div
                      initial={{ opacity: 0, scale: 0.9 }}
                      animate={{ opacity: 1, scale: 1 }}
                      className="flex flex-col items-center justify-center h-full"
                    >
                      <CheckCircle2 className="w-16 h-16 text-green-400 mb-4" />
                      <h3 className="text-xl font-bold text-white mb-2">Decision Submitted</h3>
                      <p className="text-sm text-zinc-400">Review completed successfully</p>
                    </motion.div>
                  ) : (
                    <div className="space-y-6">
                      <h3 className="text-lg font-semibold text-white uppercase tracking-wider">
                        Decision Required
                      </h3>

                      {/* Decision Options */}
                      <div className="space-y-3">
                        <DecisionButton
                          type="approve"
                          icon={CheckCircle2}
                          label="Approve Payment"
                          description="Allow this payment to proceed"
                          selected={decision === 'approve'}
                          onClick={() => setDecision('approve')}
                        />

                        <DecisionButton
                          type="reject"
                          icon={XCircle}
                          label="Reject Payment"
                          description="Block this payment"
                          selected={decision === 'reject'}
                          onClick={() => setDecision('reject')}
                        />

                        <DecisionButton
                          type="escalate"
                          icon={AlertOctagon}
                          label="Escalate Review"
                          description="Send to senior compliance officer"
                          selected={decision === 'escalate'}
                          onClick={() => setDecision('escalate')}
                        />
                      </div>

                      {/* Notes Section */}
                      <div>
                        <label className="block text-sm font-semibold text-white mb-2">
                          Review Notes <span className="text-red-400">*</span>
                        </label>
                        <textarea
                          value={notes}
                          onChange={(e) => setNotes(e.target.value)}
                          placeholder="Explain your decision... (minimum 20 characters)"
                          rows={6}
                          className="w-full px-4 py-3 bg-zinc-800/50 border border-zinc-700 rounded-lg text-sm text-white placeholder:text-zinc-600 focus:outline-none focus:border-deltran-gold/50 focus:bg-zinc-800 transition-all resize-none"
                        />
                        <div className="flex items-center justify-between mt-2">
                          <span
                            className={`text-xs ${
                              notes.length >= 20 ? 'text-green-400' : 'text-zinc-500'
                            }`}
                          >
                            {notes.length} / 20 minimum
                          </span>
                          {notes.length > 0 && notes.length < 20 && (
                            <span className="text-xs text-red-400">At least 20 characters required</span>
                          )}
                        </div>
                      </div>

                      {/* Submit Button */}
                      <button
                        onClick={handleSubmit}
                        disabled={!canSubmit}
                        className="w-full py-3 rounded-lg bg-gradient-to-r from-deltran-gold to-deltran-gold-light hover:from-deltran-gold-light hover:to-deltran-gold text-black font-bold text-sm transition-all disabled:opacity-30 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                      >
                        {submitDecisionMutation.isPending ? (
                          <>
                            <Loader2 className="w-4 h-4 animate-spin" />
                            Submitting...
                          </>
                        ) : (
                          <>
                            <Shield className="w-4 h-4" />
                            Submit Decision
                          </>
                        )}
                      </button>

                      <button
                        onClick={handleClose}
                        className="w-full py-2 rounded-lg bg-zinc-800/50 hover:bg-zinc-800 border border-zinc-700 text-zinc-400 hover:text-white text-sm transition-all"
                      >
                        Cancel
                      </button>
                    </div>
                  )}
                </div>
              </div>
            </motion.div>
          </div>
        </>
      )}
    </AnimatePresence>
  )
}

// Helper Components
function InfoCard({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="p-3 rounded-lg bg-zinc-900/50">
      <p className="text-xs text-zinc-500 mb-1">{label}</p>
      <p className={`text-sm text-zinc-300 ${mono ? 'font-mono' : ''}`}>{value}</p>
    </div>
  )
}

function RiskBadge({ level }: { level: string }) {
  const config = {
    HIGH: { bg: 'bg-red-500/20', text: 'text-red-400', border: 'border-red-500/30' },
    MEDIUM: { bg: 'bg-yellow-500/20', text: 'text-yellow-400', border: 'border-yellow-500/30' },
    LOW: { bg: 'bg-green-500/20', text: 'text-green-400', border: 'border-green-500/30' },
  }

  const { bg, text, border } = config[level as keyof typeof config] || config.MEDIUM

  return (
    <span className={`px-3 py-1 rounded-full text-xs font-bold border ${bg} ${text} ${border}`}>
      {level} RISK
    </span>
  )
}

function SourceBadge({ source, large }: { source: string; large?: boolean }) {
  const colors = {
    OFAC: 'bg-red-500/10 border-red-500/30 text-red-400',
    EU: 'bg-blue-500/10 border-blue-500/30 text-blue-400',
    UN: 'bg-purple-500/10 border-purple-500/30 text-purple-400',
    UK: 'bg-green-500/10 border-green-500/30 text-green-400',
  }

  const color = colors[source as keyof typeof colors] || colors.OFAC

  return (
    <span
      className={`inline-flex items-center gap-1.5 ${
        large ? 'px-3 py-1.5' : 'px-2 py-1'
      } rounded border font-semibold ${large ? 'text-sm' : 'text-xs'} ${color}`}
    >
      <Shield className={large ? 'w-4 h-4' : 'w-3 h-3'} />
      {source}
    </span>
  )
}

function DecisionButton({
  type,
  icon: Icon,
  label,
  description,
  selected,
  onClick,
}: {
  type: 'approve' | 'reject' | 'escalate'
  icon: any
  label: string
  description: string
  selected: boolean
  onClick: () => void
}) {
  const colors = {
    approve: {
      border: 'border-green-500/30 hover:border-green-500',
      bg: 'bg-green-500/10 hover:bg-green-500/20',
      icon: 'text-green-400',
      selectedBorder: 'border-green-500',
      selectedBg: 'bg-green-500/20',
    },
    reject: {
      border: 'border-red-500/30 hover:border-red-500',
      bg: 'bg-red-500/10 hover:bg-red-500/20',
      icon: 'text-red-400',
      selectedBorder: 'border-red-500',
      selectedBg: 'bg-red-500/20',
    },
    escalate: {
      border: 'border-yellow-500/30 hover:border-yellow-500',
      bg: 'bg-yellow-500/10 hover:bg-yellow-500/20',
      icon: 'text-yellow-400',
      selectedBorder: 'border-yellow-500',
      selectedBg: 'bg-yellow-500/20',
    },
  }

  const color = colors[type]

  return (
    <button
      onClick={onClick}
      className={`w-full p-4 rounded-lg border-2 transition-all text-left ${
        selected
          ? `${color.selectedBorder} ${color.selectedBg} shadow-lg`
          : `${color.border} ${color.bg}`
      }`}
    >
      <div className="flex items-start gap-3">
        <Icon className={`w-5 h-5 ${color.icon} mt-0.5`} />
        <div>
          <p className="text-sm font-semibold text-white mb-1">{label}</p>
          <p className="text-xs text-zinc-400">{description}</p>
        </div>
      </div>
    </button>
  )
}

function formatCurrency(amount: number, currency: string): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
  }).format(amount)
}
