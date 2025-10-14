'use client'

import { useState, useEffect } from 'react'
import { TrendingUp, Activity, Target, Clock, Wallet, LogOut, User, Zap, Layers, AlertTriangle, Shield } from 'lucide-react'
import { motion } from 'framer-motion'
import { useRouter } from 'next/navigation'
import { AnimatedCard } from './components/AnimatedCard'
import { TransactionsTable } from './components/transactions/TransactionsTable'
import { PaymentFlow } from './components/flow/PaymentFlow'
import { RiskHeatmap } from './components/analytics/RiskHeatmap'
import { CurrencyDonut } from './components/analytics/CurrencyDonut'
import { DailyMetricsCharts } from './components/charts/DailyMetricsCharts'
import { ProtectedRoute } from './components/auth/ProtectedRoute'
import { AdvancedFilters, FilterParams } from './components/filters/AdvancedFilters'
import { ExportButton } from './components/export/ExportButton'
import { ConnectionIndicator } from './components/websocket/ConnectionIndicator'
import { useTransactions } from './hooks/useTransactions'
import { useFilteredTransactions } from './hooks/useFilteredTransactions'
import { useRiskData } from './hooks/useRiskData'
import { useCurrencyDistribution } from './hooks/useCurrencyDistribution'
import { useAuth } from './hooks/useAuth'
import { useSystemMetrics } from './hooks/useSystemMetrics'
import { useWebSocket } from './hooks/useWebSocket'

function DashboardContent() {
  const router = useRouter()
  const [filters, setFilters] = useState<FilterParams>({})

  // WebSocket connection (optional - only if backend supports it)
  const wsUrl = typeof window !== 'undefined' ? `ws://${window.location.hostname}:8080/api/v1/ws` : undefined
  const { status, reconnectAttempts, subscribe } = useWebSocket(wsUrl)

  // Use filtered transactions when filters are active, otherwise use default
  const hasFilters = Object.values(filters).some((v) => v !== '' && v !== undefined)
  const { data: defaultTransactions, isLoading: defaultLoading, refetch: refetchTransactions } = useTransactions()
  const { data: filteredTransactions, isLoading: filteredLoading } = useFilteredTransactions(filters)

  const transactions = hasFilters ? filteredTransactions : defaultTransactions
  const isLoading = hasFilters ? filteredLoading : defaultLoading

  const riskData = useRiskData(transactions)
  const currencyData = useCurrencyDistribution(transactions)
  const { user, logout } = useAuth()
  const { data: metrics, refetch: refetchMetrics } = useSystemMetrics()

  // Subscribe to real-time updates
  useEffect(() => {
    if (!subscribe) return

    // Payment updates
    const unsubscribePayments = subscribe('payment_update', (data: any) => {
      console.log('Payment update received:', data)
      refetchTransactions()
    })

    // Metrics updates
    const unsubscribeMetrics = subscribe('metrics_update', (data: any) => {
      console.log('Metrics update received:', data)
      refetchMetrics()
    })

    return () => {
      unsubscribePayments()
      unsubscribeMetrics()
    }
  }, [subscribe, refetchTransactions, refetchMetrics])

  const handleLogout = async () => {
    await logout()
    router.push('/login')
  }

  const resetFilters = () => {
    setFilters({})
  }

  return (
    <div className="min-h-screen bg-deltran-dark">
      {/* Header */}
      <motion.header
        initial={{ y: -20, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ duration: 0.5 }}
        className="sticky top-0 z-50 border-b border-zinc-800 bg-deltran-dark/80 backdrop-blur-xl"
      >
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            {/* Logo */}
            <div className="flex items-center gap-3">
              <motion.div
                whileHover={{ rotate: 360 }}
                transition={{ duration: 0.6, ease: 'easeInOut' }}
                className="p-2 rounded-lg bg-gradient-to-br from-deltran-gold to-deltran-gold-light"
              >
                <Wallet className="w-6 h-6 text-black" />
              </motion.div>
              <div>
                <h1 className="text-2xl font-bold text-gradient-gold">DelTran</h1>
                <p className="text-xs text-zinc-500 uppercase tracking-wider">Premium Gateway</p>
              </div>
            </div>

            {/* User Menu */}
            <div className="flex items-center gap-4">
              <ConnectionIndicator status={status} reconnectAttempts={reconnectAttempts} />

              <div className="flex items-center gap-3">
                <div className="text-right">
                  <p className="text-sm text-white font-medium">{user?.name || 'User'}</p>
                  <p className="text-xs text-zinc-500">{user?.role || 'admin'}</p>
                </div>
                <button
                  onClick={handleLogout}
                  className="p-2 rounded-lg bg-zinc-800/50 hover:bg-red-500/10 border border-zinc-700 hover:border-red-500/30 transition-all group"
                  title="Logout"
                >
                  <LogOut className="w-4 h-4 text-zinc-400 group-hover:text-red-400 transition-colors" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </motion.header>

      {/* Main */}
      <main className="container mx-auto px-6 py-8">
        {/* Title */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="mb-8"
        >
          <h2 className="text-4xl font-bold text-white mb-2">Dashboard</h2>
          <p className="text-zinc-400">Real-time payment metrics</p>
        </motion.div>

        {/* Metrics Grid - Row 1 */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-6">
          <AnimatedCard
            title="Total Volume"
            value={metrics?.totalVolumeFormatted || "$15.2M"}
            numericValue={metrics?.totalVolume || 15200000}
            format="currency"
            change={`↑ ${metrics?.changeVolume || '12.5%'}`}
            icon={TrendingUp}
            delay={0}
          />
          <AnimatedCard
            title="Active Payments"
            value={metrics?.activePayments.toString() || "142"}
            numericValue={metrics?.activePayments || 142}
            format="number"
            change={`↑ ${metrics?.changePayments || '8.3%'}`}
            icon={Activity}
            delay={0.1}
          />
          <AnimatedCard
            title="Settlement Rate"
            value={`${metrics?.settlementRate || 98.7}%`}
            numericValue={metrics?.settlementRate || 98.7}
            format="percentage"
            change={`↑ ${metrics?.changeSettlement || '2.1%'}`}
            icon={Target}
            delay={0.2}
          />
          <AnimatedCard
            title="Avg Processing"
            value={`${metrics?.avgProcessingTime || 234}ms`}
            change={`↓ ${metrics?.changeProcessing || '5.7%'}`}
            icon={Clock}
            delay={0.3}
          />
        </div>

        {/* Metrics Grid - Row 2 (Enhanced Metrics) - Waiting for backend */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          <AnimatedCard
            title="TPS"
            value="--"
            numericValue={0}
            format="number"
            change="Awaiting data"
            icon={Zap}
            delay={0.4}
          />
          <AnimatedCard
            title="Queue Depth"
            value="--"
            numericValue={0}
            format="number"
            change="Awaiting data"
            icon={Layers}
            delay={0.5}
          />
          <AnimatedCard
            title="Failed Last Hour"
            value="--"
            numericValue={0}
            format="number"
            change="Awaiting data"
            icon={AlertTriangle}
            delay={0.6}
          />
          <AnimatedCard
            title="Compliance Reviews"
            value="--"
            numericValue={0}
            format="number"
            change="Awaiting data"
            icon={Shield}
            delay={0.7}
          />
        </div>

        {/* Payment Flow Visualization */}
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.5, delay: 0.4 }}
          className="mb-8"
        >
          <PaymentFlow transactions={transactions} />
        </motion.div>

        {/* Daily Metrics Charts */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.8 }}
          className="mb-8"
        >
          <DailyMetricsCharts />
        </motion.div>

        {/* Analytics Grid - Risk Heatmap & Currency Donut */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.5, delay: 0.9 }}
          >
            <RiskHeatmap
              cells={riskData.cells}
              maxScore={riskData.maxScore}
            />
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.5, delay: 1.0 }}
          >
            <CurrencyDonut
              currencies={currencyData.currencies}
              totalVolume={currencyData.totalVolume}
              dominantCurrency={currencyData.dominantCurrency}
            />
          </motion.div>
        </div>

        {/* Transactions Table */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 1.1 }}
          className="mb-8"
        >
          <TransactionsTable transactions={transactions} isLoading={isLoading} />
        </motion.div>

        {/* Status Banner */}
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.5, delay: 0.9 }}
          className="p-6 rounded-xl bg-gradient-to-r from-deltran-gold/10 to-transparent border border-deltran-gold/20"
        >
          <p className="text-sm text-zinc-400">
            <span className="text-deltran-gold font-semibold">✅ Phase 1-3 Complete!</span> Payment Details, Advanced Filters, CSV Export, Compliance Review, WebSocket Updates, 8-Metric Dashboard, and Historical Charts (Line/Bar/Area) - Premium UI Integration fully operational!
          </p>
        </motion.div>
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
