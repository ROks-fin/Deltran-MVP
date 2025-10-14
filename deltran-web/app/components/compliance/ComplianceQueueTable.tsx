'use client'

import { motion } from 'framer-motion'
import { Shield, AlertTriangle, CheckCircle2, Eye } from 'lucide-react'
import { ComplianceReview } from '@/app/hooks/useComplianceReviews'

interface ComplianceQueueTableProps {
  reviews: ComplianceReview[]
  isLoading?: boolean
  onReview: (reviewId: string) => void
}

export function ComplianceQueueTable({ reviews, isLoading, onReview }: ComplianceQueueTableProps) {
  if (isLoading) {
    return (
      <div className="rounded-xl p-6 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
        <div className="space-y-3">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-16 bg-zinc-800/50 rounded animate-pulse" />
          ))}
        </div>
      </div>
    )
  }

  if (reviews.length === 0) {
    return (
      <div className="rounded-xl p-12 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
        <div className="text-center">
          <CheckCircle2 className="w-16 h-16 mx-auto mb-4 text-green-400 opacity-50" />
          <h3 className="text-lg font-semibold text-white mb-2">All Clear!</h3>
          <p className="text-sm text-zinc-400">No compliance reviews pending at this time</p>
        </div>
      </div>
    )
  }

  return (
    <div className="rounded-xl overflow-hidden bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="border-b border-zinc-800 bg-zinc-900/50">
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Payment
              </th>
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Parties
              </th>
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Amount
              </th>
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Risk Level
              </th>
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Match Score
              </th>
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Source
              </th>
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Created
              </th>
              <th className="text-left py-4 px-4 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            {reviews.map((review, index) => (
              <motion.tr
                key={review.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.05, duration: 0.3 }}
                className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors group"
              >
                <td className="py-4 px-4">
                  <div>
                    <p className="text-sm font-mono text-zinc-300">{review.payment_reference}</p>
                    <p className="text-xs text-zinc-500 mt-0.5">{review.check_type}</p>
                  </div>
                </td>

                <td className="py-4 px-4">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-blue-400">From:</span>
                      <span className="text-xs font-mono text-zinc-300">{review.sender_bic}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-green-400">To:</span>
                      <span className="text-xs font-mono text-zinc-300">{review.receiver_bic}</span>
                    </div>
                  </div>
                </td>

                <td className="py-4 px-4">
                  <span className="text-sm font-semibold text-white">
                    {new Intl.NumberFormat('en-US', {
                      style: 'currency',
                      currency: review.currency,
                    }).format(review.amount)}
                  </span>
                </td>

                <td className="py-4 px-4">
                  <RiskBadge level={review.risk_level} />
                </td>

                <td className="py-4 px-4">
                  <div className="flex items-center gap-2">
                    <div className="w-16 h-2 bg-zinc-800 rounded-full overflow-hidden">
                      <div
                        className="h-full transition-all"
                        style={{
                          width: `${review.match_score * 100}%`,
                          backgroundColor: getMatchScoreColor(review.match_score),
                        }}
                      />
                    </div>
                    <span className="text-xs text-zinc-400">{Math.round(review.match_score * 100)}%</span>
                  </div>
                </td>

                <td className="py-4 px-4">
                  {review.matches && review.matches.length > 0 && (
                    <SourceBadge source={review.matches[0].source} />
                  )}
                </td>

                <td className="py-4 px-4">
                  <span className="text-xs text-zinc-500 font-mono">
                    {formatTimeAgo(review.created_at)}
                  </span>
                </td>

                <td className="py-4 px-4">
                  <button
                    onClick={() => onReview(review.id)}
                    className="px-3 py-1.5 rounded-lg bg-deltran-gold/10 hover:bg-deltran-gold/20 border border-deltran-gold/30 hover:border-deltran-gold text-deltran-gold text-xs font-semibold transition-all flex items-center gap-1.5"
                  >
                    <Eye className="w-3 h-3" />
                    Review
                  </button>
                </td>
              </motion.tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

function RiskBadge({ level }: { level: string }) {
  const config = {
    HIGH: {
      bg: 'bg-red-500/10',
      border: 'border-red-500/30',
      text: 'text-red-400',
      glow: 'shadow-[0_0_15px_rgba(239,68,68,0.3)]',
    },
    MEDIUM: {
      bg: 'bg-yellow-500/10',
      border: 'border-yellow-500/30',
      text: 'text-yellow-400',
      glow: '',
    },
    LOW: {
      bg: 'bg-green-500/10',
      border: 'border-green-500/30',
      text: 'text-green-400',
      glow: '',
    },
  }

  const { bg, border, text, glow } = config[level as keyof typeof config] || config.MEDIUM

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-semibold border ${bg} ${border} ${text} ${glow}`}
    >
      <AlertTriangle className="w-3 h-3" />
      {level}
    </span>
  )
}

function SourceBadge({ source }: { source: string }) {
  const colors = {
    OFAC: 'bg-red-500/10 border-red-500/30 text-red-400',
    EU: 'bg-blue-500/10 border-blue-500/30 text-blue-400',
    UN: 'bg-purple-500/10 border-purple-500/30 text-purple-400',
    UK: 'bg-green-500/10 border-green-500/30 text-green-400',
  }

  const color = colors[source as keyof typeof colors] || colors.OFAC

  return (
    <span className={`inline-flex items-center gap-1 px-2 py-1 rounded border text-xs font-semibold ${color}`}>
      <Shield className="w-3 h-3" />
      {source}
    </span>
  )
}

function getMatchScoreColor(score: number): string {
  if (score >= 0.7) return '#f87171' // red
  if (score >= 0.4) return '#f59e0b' // yellow
  return '#4ade80' // green
}

function formatTimeAgo(dateString: string): string {
  const date = new Date(dateString)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const minutes = Math.floor(diff / 60000)

  if (minutes < 1) return 'Just now'
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.floor(hours / 24)
  return `${days}d ago`
}
