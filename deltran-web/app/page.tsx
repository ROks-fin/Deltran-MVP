'use client'

import { useEffect } from 'react'
import {
  Activity, Target, Clock, Zap, Layers, AlertTriangle, Shield,
  Sparkles, BarChart3, DollarSign, CheckCircle, Globe2
} from 'lucide-react'
import { motion } from 'framer-motion'
import { useRouter } from 'next/navigation'
import { TransactionsTable } from './components/transactions/TransactionsTable'
import { PaymentFlow } from './components/flow/PaymentFlow'
import { RiskHeatmap } from './components/analytics/RiskHeatmap'
import { CurrencyDonut } from './components/analytics/CurrencyDonut'
import { DailyMetricsCharts } from './components/charts/DailyMetricsCharts'
import { ProtectedRoute } from './components/auth/ProtectedRoute'
import { useTransactions } from './hooks/useTransactions'
import { useRiskData } from './hooks/useRiskData'
import { useCurrencyDistribution } from './hooks/useCurrencyDistribution'
import { useAuth } from './hooks/useAuth'
import { useSystemMetrics } from './hooks/useSystemMetrics'
import { useWebSocket } from './hooks/useWebSocket'

// Premium Components
import { MetricCard, GlassCard } from './components/premium/PremiumCard'
import { PremiumButton } from './components/premium/PremiumButton'
import { SectionReveal, FadeIn } from './components/premium/PageTransition'

function DashboardContent() {
  const router = useRouter()
  const { user } = useAuth()

  // WebSocket connection
  const wsUrl = typeof window !== 'undefined' ? `ws://${window.location.hostname}:8080/api/v1/ws` : undefined
  const { status, subscribe } = useWebSocket(wsUrl)

  // Fetch real data from API
  const { data: transactions, isLoading: transactionsLoading, refetch: refetchTransactions } = useTransactions()
  const { data: metrics, isLoading: metricsLoading, refetch: refetchMetrics } = useSystemMetrics()
  const riskData = useRiskData(transactions)
  const currencyData = useCurrencyDistribution(transactions)

  const isLoading = transactionsLoading || metricsLoading

  // Subscribe to real-time updates
  useEffect(() => {
    if (!subscribe) return

    const unsubscribePayments = subscribe('payment_update', (data: any) => {
      console.log('Payment update received:', data)
      refetchTransactions()
    })

    const unsubscribeMetrics = subscribe('metrics_update', (data: any) => {
      console.log('Metrics update received:', data)
      refetchMetrics()
    })

    return () => {
      unsubscribePayments()
      unsubscribeMetrics()
    }
  }, [subscribe, refetchTransactions, refetchMetrics])

  return (
    <div className="min-h-screen">
      <main className="container mx-auto px-8 py-12">

        {/* PREMIUM HEADER SECTION */}
        <FadeIn direction="up" delay={0}>
          <header className="mb-16 relative">
            {/* Background gradient effect */}
            <div className="absolute inset-0 -z-10">
              <motion.div
                animate={{
                  backgroundPosition: ['0% 50%', '100% 50%', '0% 50%'],
                }}
                transition={{
                  duration: 10,
                  repeat: Infinity,
                  ease: 'linear',
                }}
                className="absolute inset-0 bg-gradient-to-r from-deltran-gold/5 via-deltran-gold-light/5 to-deltran-gold/5 blur-3xl"
                style={{ backgroundSize: '200% 200%' }}
              />
            </div>

            <div className="flex flex-col lg:flex-row items-start lg:items-center justify-between gap-8">
              {/* Left side - Title & Description */}
              <div className="flex-1">
                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.6, delay: 0.1 }}
                  className="flex items-center gap-3 mb-4"
                >
                  <motion.div
                    animate={{
                      rotate: [0, 10, -10, 0],
                      scale: [1, 1.1, 1],
                    }}
                    transition={{
                      duration: 3,
                      repeat: Infinity,
                      ease: 'easeInOut',
                    }}
                  >
                    <Sparkles className="w-10 h-10 text-deltran-gold" />
                  </motion.div>
                  <h1 className="text-6xl md:text-7xl font-bold text-gradient-gold font-serif">
                    DelTran
                  </h1>
                </motion.div>

                <motion.p
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.6, delay: 0.2 }}
                  className="text-2xl text-zinc-300 mb-4"
                >
                  Premium Payment Gateway
                </motion.p>

                <motion.p
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.6, delay: 0.3 }}
                  className="text-lg text-zinc-500 max-w-2xl"
                >
                  Experience Swiss bank-level payment processing with real-time analytics,
                  advanced risk management, and seamless multi-currency support.
                </motion.p>

                {/* User greeting */}
                {user && (
                  <motion.div
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ duration: 0.6, delay: 0.4 }}
                    className="mt-6 flex items-center gap-3"
                  >
                    <div className="w-12 h-12 rounded-full bg-gradient-to-br from-deltran-gold to-deltran-gold-dark flex items-center justify-center">
                      <span className="text-xl font-bold text-white">
                        {user.email?.[0]?.toUpperCase() || 'A'}
                      </span>
                    </div>
                    <div>
                      <p className="text-sm text-zinc-500">Welcome back,</p>
                      <p className="text-lg font-semibold text-white">{user.email}</p>
                    </div>
                  </motion.div>
                )}
              </div>

              {/* Right side - Quick Actions */}
              <motion.div
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.6, delay: 0.5 }}
                className="flex flex-col gap-3"
              >
                <PremiumButton
                  variant="primary"
                  size="lg"
                  icon={<BarChart3 size={20} />}
                  onClick={() => router.push('/analytics')}
                  magnetic={true}
                >
                  View Analytics
                </PremiumButton>
                <PremiumButton
                  variant="secondary"
                  size="lg"
                  icon={<Activity size={20} />}
                  onClick={() => router.push('/transactions')}
                  magnetic={true}
                >
                  All Transactions
                </PremiumButton>
              </motion.div>
            </div>

            {/* Connection status indicator */}
            <motion.div
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.6, delay: 0.6 }}
              className="mt-8 flex items-center gap-2"
            >
              <div className={`w-2 h-2 rounded-full ${status === 'connected' ? 'bg-green-400' : 'bg-yellow-400'} animate-pulse`} />
              <span className="text-sm text-zinc-500">
                {status === 'connected' ? 'Live data stream active' : 'Connecting...'}
              </span>
            </motion.div>
          </header>
        </FadeIn>

        {/* KEY METRICS SECTION */}
        <SectionReveal delay={0.1}>
          <div className="mb-12">
            <h2 className="text-3xl font-bold text-gradient-gold mb-6 font-serif">
              Live Metrics
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
              <MetricCard
                title="Total Volume"
                value={isLoading ? '--' : (metrics?.totalVolumeFormatted || "$0")}
                change={metrics?.changeVolume ? `� ${metrics.changeVolume}` : 'Real-time data'}
                icon={<DollarSign size={24} />}
                trend="up"
                delay={0}
              />
              <MetricCard
                title="Active Payments"
                value={isLoading ? '--' : (metrics?.activePayments?.toString() || "0")}
                change={metrics?.changePayments ? `� ${metrics.changePayments}` : 'Real-time data'}
                icon={<Activity size={24} />}
                trend="up"
                delay={0.1}
              />
              <MetricCard
                title="Settlement Rate"
                value={isLoading ? '--' : `${metrics?.settlementRate || 0}%`}
                change={metrics?.changeSettlement ? `� ${metrics.changeSettlement}` : 'Real-time data'}
                icon={<Target size={24} />}
                trend="up"
                delay={0.2}
              />
              <MetricCard
                title="Avg Processing"
                value={isLoading ? '--' : `${metrics?.avgProcessingTime || 0}ms`}
                change={metrics?.changeProcessing ? `� ${metrics.changeProcessing}` : 'Real-time data'}
                icon={<Clock size={24} />}
                trend="up"
                delay={0.3}
              />
            </div>
          </div>
        </SectionReveal>

        {/* SYSTEM STATUS SECTION */}
        <SectionReveal delay={0.2}>
          <div className="mb-12">
            <h2 className="text-3xl font-bold text-gradient-gold mb-6 font-serif">
              System Status
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
              <MetricCard
                title="TPS"
                value="--"
                change="Awaiting backend data"
                icon={<Zap size={24} />}
                trend="neutral"
                delay={0.4}
              />
              <MetricCard
                title="Queue Depth"
                value="--"
                change="Awaiting backend data"
                icon={<Layers size={24} />}
                trend="neutral"
                delay={0.5}
              />
              <MetricCard
                title="Failed Last Hour"
                value="--"
                change="Awaiting backend data"
                icon={<AlertTriangle size={24} />}
                trend="neutral"
                delay={0.6}
              />
              <MetricCard
                title="Compliance Reviews"
                value="--"
                change="Awaiting backend data"
                icon={<Shield size={24} />}
                trend="neutral"
                delay={0.7}
              />
            </div>
          </div>
        </SectionReveal>

        {/* PAYMENT FLOW VISUALIZATION */}
        <SectionReveal delay={0.3}>
          <div className="mb-12">
            <h2 className="text-3xl font-bold text-gradient-gold mb-6 font-serif">
              Payment Flow
            </h2>
            <GlassCard className="p-6">
              {isLoading ? (
                <div className="h-64 flex items-center justify-center">
                  <div className="text-zinc-500">Loading payment flow...</div>
                </div>
              ) : (
                <PaymentFlow transactions={transactions} />
              )}
            </GlassCard>
          </div>
        </SectionReveal>

        {/* HISTORICAL CHARTS */}
        <SectionReveal delay={0.4}>
          <div className="mb-12">
            <h2 className="text-3xl font-bold text-gradient-gold mb-6 font-serif">
              Historical Metrics
            </h2>
            <GlassCard className="p-6">
              {isLoading ? (
                <div className="h-64 flex items-center justify-center">
                  <div className="text-zinc-500">Loading charts...</div>
                </div>
              ) : (
                <DailyMetricsCharts />
              )}
            </GlassCard>
          </div>
        </SectionReveal>

        {/* ANALYTICS GRID - Risk & Currency */}
        <SectionReveal delay={0.5}>
          <div className="mb-12">
            <h2 className="text-3xl font-bold text-gradient-gold mb-6 font-serif">
              Risk & Currency Analysis
            </h2>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <GlassCard className="p-6">
                <h3 className="text-xl font-semibold text-white mb-4">Risk Heatmap</h3>
                {isLoading ? (
                  <div className="h-64 flex items-center justify-center">
                    <div className="text-zinc-500">Loading risk data...</div>
                  </div>
                ) : (
                  <RiskHeatmap
                    cells={riskData.cells}
                    maxScore={riskData.maxScore}
                  />
                )}
              </GlassCard>

              <GlassCard className="p-6">
                <h3 className="text-xl font-semibold text-white mb-4">Currency Distribution</h3>
                {isLoading ? (
                  <div className="h-64 flex items-center justify-center">
                    <div className="text-zinc-500">Loading currency data...</div>
                  </div>
                ) : (
                  <CurrencyDonut
                    currencies={currencyData.currencies}
                    totalVolume={currencyData.totalVolume}
                    dominantCurrency={currencyData.dominantCurrency}
                  />
                )}
              </GlassCard>
            </div>
          </div>
        </SectionReveal>

        {/* TRANSACTIONS TABLE */}
        <SectionReveal delay={0.6}>
          <div className="mb-12">
            <h2 className="text-3xl font-bold text-gradient-gold mb-6 font-serif">
              Recent Transactions
            </h2>
            <GlassCard>
              <TransactionsTable
                transactions={transactions}
                isLoading={isLoading}
              />
            </GlassCard>
          </div>
        </SectionReveal>

        {/* FEATURES HIGHLIGHT */}
        <SectionReveal delay={0.7}>
          <div className="relative overflow-hidden rounded-2xl">
            {/* Animated gradient background */}
            <motion.div
              animate={{
                backgroundPosition: ['0% 50%', '100% 50%', '0% 50%'],
              }}
              transition={{
                duration: 5,
                repeat: Infinity,
                ease: 'linear',
              }}
              className="absolute inset-0 bg-gradient-to-r from-deltran-gold/20 via-deltran-gold-light/20 to-deltran-gold/20"
              style={{ backgroundSize: '200% 200%' }}
            />

            {/* Glass overlay */}
            <div className="absolute inset-0 backdrop-blur-xl bg-deltran-dark-charcoal/50" />

            {/* Border glow */}
            <div className="absolute inset-0 rounded-2xl ring-1 ring-inset ring-deltran-gold/30" />

            {/* Content */}
            <div className="relative p-8">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
                {/* Feature 1 */}
                <div className="flex flex-col items-center text-center">
                  <motion.div
                    animate={{
                      y: [0, -10, 0],
                    }}
                    transition={{
                      duration: 2,
                      repeat: Infinity,
                      ease: 'easeInOut',
                    }}
                    className="mb-4 p-4 rounded-2xl bg-gradient-to-br from-deltran-gold/20 to-deltran-gold-dark/20"
                  >
                    <CheckCircle className="w-8 h-8 text-deltran-gold" />
                  </motion.div>
                  <h3 className="text-xl font-bold text-white mb-2">Real-Time Processing</h3>
                  <p className="text-zinc-400">
                    Instant payment confirmation with WebSocket live updates
                  </p>
                </div>

                {/* Feature 2 */}
                <div className="flex flex-col items-center text-center">
                  <motion.div
                    animate={{
                      y: [0, -10, 0],
                    }}
                    transition={{
                      duration: 2,
                      delay: 0.2,
                      repeat: Infinity,
                      ease: 'easeInOut',
                    }}
                    className="mb-4 p-4 rounded-2xl bg-gradient-to-br from-deltran-gold/20 to-deltran-gold-dark/20"
                  >
                    <Shield className="w-8 h-8 text-deltran-gold" />
                  </motion.div>
                  <h3 className="text-xl font-bold text-white mb-2">Advanced Security</h3>
                  <p className="text-zinc-400">
                    Bank-grade encryption with AI-powered fraud detection
                  </p>
                </div>

                {/* Feature 3 */}
                <div className="flex flex-col items-center text-center">
                  <motion.div
                    animate={{
                      y: [0, -10, 0],
                    }}
                    transition={{
                      duration: 2,
                      delay: 0.4,
                      repeat: Infinity,
                      ease: 'easeInOut',
                    }}
                    className="mb-4 p-4 rounded-2xl bg-gradient-to-br from-deltran-gold/20 to-deltran-gold-dark/20"
                  >
                    <Globe2 className="w-8 h-8 text-deltran-gold" />
                  </motion.div>
                  <h3 className="text-xl font-bold text-white mb-2">Global Coverage</h3>
                  <p className="text-zinc-400">
                    Multi-currency support with competitive exchange rates
                  </p>
                </div>
              </div>
            </div>
          </div>
        </SectionReveal>

      </main>
    </div>
  )
}

export default function HomePage() {
  return (
    <ProtectedRoute>
      <DashboardContent />
    </ProtectedRoute>
  )
}
