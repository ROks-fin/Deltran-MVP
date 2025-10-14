'use client'

import { useState } from 'react'
import { motion } from 'framer-motion'
import { Shield, AlertTriangle, Clock, CheckCircle2, TrendingUp } from 'lucide-react'
import { ProtectedRoute } from '@/app/components/auth/ProtectedRoute'
import { ComplianceQueueTable } from '@/app/components/compliance/ComplianceQueueTable'
import { ComplianceReviewModal } from '@/app/components/compliance/ComplianceReviewModal'
import { useComplianceReviews, useComplianceStats, ComplianceReview } from '@/app/hooks/useComplianceReviews'

type TabType = 'pending' | 'high_risk' | 'all_hits' | 'history'

function CompliancePageContent() {
  const [activeTab, setActiveTab] = useState<TabType>('pending')
  const [selectedReview, setSelectedReview] = useState<ComplianceReview | null>(null)

  // Fetch data based on active tab
  const pendingReviews = useComplianceReviews({ status: 'review' })
  const highRiskReviews = useComplianceReviews({ risk_level: 'HIGH' })
  const allHitsReviews = useComplianceReviews({})
  const historyReviews = useComplianceReviews({ status: 'approved,rejected,escalate' })

  const { data: stats } = useComplianceStats()

  // Get data for current tab
  const getCurrentData = () => {
    switch (activeTab) {
      case 'pending':
        return pendingReviews
      case 'high_risk':
        return highRiskReviews
      case 'all_hits':
        return allHitsReviews
      case 'history':
        return historyReviews
      default:
        return pendingReviews
    }
  }

  const { data: reviews = [], isLoading } = getCurrentData()

  const handleReview = (reviewId: string) => {
    const review = reviews.find((r) => r.id === reviewId)
    if (review) {
      setSelectedReview(review)
    }
  }

  const tabs = [
    {
      id: 'pending' as TabType,
      label: 'Pending Reviews',
      count: stats?.pendingReviews || 0,
      icon: Clock,
      color: 'yellow',
    },
    {
      id: 'high_risk' as TabType,
      label: 'High Risk',
      count: stats?.highRiskCount || 0,
      icon: AlertTriangle,
      color: 'red',
    },
    {
      id: 'all_hits' as TabType,
      label: 'All Hits',
      count: reviews.length,
      icon: Shield,
      color: 'blue',
    },
    {
      id: 'history' as TabType,
      label: 'History',
      count: stats?.reviewedToday || 0,
      icon: CheckCircle2,
      color: 'green',
    },
  ]

  return (
    <div className="min-h-screen bg-deltran-dark">
      <div className="container mx-auto px-6 py-8">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="mb-8"
        >
          <div className="flex items-center gap-3 mb-3">
            <div className="p-3 rounded-xl bg-gradient-to-br from-deltran-gold/20 to-transparent border border-deltran-gold/30">
              <Shield className="w-8 h-8 text-deltran-gold" />
            </div>
            <div>
              <h1 className="text-4xl font-bold text-white">Compliance Review Dashboard</h1>
              <p className="text-zinc-400 mt-1">Review payments flagged by sanctions screening</p>
            </div>
          </div>
        </motion.div>

        {/* Stats Summary */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8"
        >
          <StatCard
            title="Pending Reviews"
            value={stats?.pendingReviews || 0}
            icon={Clock}
            color="yellow"
            pulse
          />
          <StatCard
            title="High Risk Count"
            value={stats?.highRiskCount || 0}
            icon={AlertTriangle}
            color="red"
          />
          <StatCard
            title="Reviewed Today"
            value={stats?.reviewedToday || 0}
            icon={CheckCircle2}
            color="green"
          />
        </motion.div>

        {/* Tabs */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="mb-6"
        >
          <div className="rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 p-2 flex gap-2 overflow-x-auto">
            {tabs.map((tab) => {
              const Icon = tab.icon
              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className={`relative flex-1 min-w-[150px] px-4 py-3 rounded-lg transition-all ${
                    activeTab === tab.id
                      ? 'bg-zinc-800 text-white shadow-lg'
                      : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50'
                  }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <Icon className={`w-4 h-4 ${getIconColor(tab.color, activeTab === tab.id)}`} />
                    <span className="text-sm font-medium">{tab.label}</span>
                    {tab.count > 0 && (
                      <span
                        className={`px-2 py-0.5 rounded-full text-xs font-bold ${
                          activeTab === tab.id
                            ? getBadgeColor(tab.color)
                            : 'bg-zinc-700 text-zinc-400'
                        }`}
                      >
                        {tab.count}
                      </span>
                    )}
                  </div>
                </button>
              )
            })}
          </div>
        </motion.div>

        {/* Queue Table */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
        >
          <ComplianceQueueTable reviews={reviews} isLoading={isLoading} onReview={handleReview} />
        </motion.div>

        {/* Audit Trail Section - Recent Decisions */}
        {activeTab === 'history' && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.4 }}
            className="mt-8"
          >
            <div className="rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 p-6">
              <h3 className="text-lg font-semibold text-white mb-4">Recent Decisions</h3>
              <p className="text-sm text-zinc-400">
                Showing recent compliance decisions. Full audit trail available in system logs.
              </p>
            </div>
          </motion.div>
        )}
      </div>

      {/* Review Modal */}
      <ComplianceReviewModal
        review={selectedReview}
        isOpen={!!selectedReview}
        onClose={() => setSelectedReview(null)}
      />
    </div>
  )
}

// Stat Card Component
function StatCard({
  title,
  value,
  icon: Icon,
  color,
  pulse,
}: {
  title: string
  value: number
  icon: any
  color: string
  pulse?: boolean
}) {
  return (
    <div className="p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 hover:border-zinc-700 transition-all">
      <div className="flex items-center justify-between mb-4">
        <div className={`p-3 rounded-lg bg-${color}-500/10 border border-${color}-500/30`}>
          <Icon className={`w-6 h-6 text-${color}-400`} />
        </div>
        {pulse && value > 0 && (
          <motion.div
            animate={{ scale: [1, 1.2, 1], opacity: [0.5, 1, 0.5] }}
            transition={{ duration: 2, repeat: Infinity }}
            className={`w-2 h-2 rounded-full bg-${color}-400`}
          />
        )}
      </div>

      <div>
        <p className="text-sm text-zinc-400 mb-1">{title}</p>
        <p className="text-3xl font-bold text-white">{value}</p>
      </div>
    </div>
  )
}

// Helper functions
function getIconColor(color: string, isActive: boolean): string {
  if (!isActive) return ''

  const colors = {
    yellow: 'text-yellow-400',
    red: 'text-red-400',
    blue: 'text-blue-400',
    green: 'text-green-400',
  }

  return colors[color as keyof typeof colors] || ''
}

function getBadgeColor(color: string): string {
  const colors = {
    yellow: 'bg-yellow-500/20 text-yellow-400',
    red: 'bg-red-500/20 text-red-400',
    blue: 'bg-blue-500/20 text-blue-400',
    green: 'bg-green-500/20 text-green-400',
  }

  return colors[color as keyof typeof colors] || 'bg-zinc-700 text-zinc-400'
}

export default function CompliancePage() {
  return (
    <ProtectedRoute>
      <CompliancePageContent />
    </ProtectedRoute>
  )
}
