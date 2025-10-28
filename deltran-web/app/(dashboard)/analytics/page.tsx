'use client';

import React from 'react';
import { motion } from 'framer-motion';
import {
  TrendingUp,
  DollarSign,
  Activity,
  Clock,
  Target,
  AlertTriangle,
  CheckCircle,
  XCircle,
  BarChart3,
} from 'lucide-react';
import { useTransactions } from '@/app/hooks/useTransactions';
import { useSystemMetrics } from '@/app/hooks/useSystemMetrics';
import { useRiskData } from '@/app/hooks/useRiskData';
import { useCurrencyDistribution } from '@/app/hooks/useCurrencyDistribution';
import { MetricCard, GlassCard, PremiumCard } from '@/app/components/premium/PremiumCard';
import { SectionReveal, FadeIn } from '@/app/components/premium/PageTransition';
import { RiskHeatmap } from '@/app/components/analytics/RiskHeatmap';
import { CurrencyDonut } from '@/app/components/analytics/CurrencyDonut';
import { DailyMetricsCharts } from '@/app/components/charts/DailyMetricsCharts';

export default function AnalyticsPage() {
  // Fetch real data from API
  const { data: transactions, isLoading: transactionsLoading } = useTransactions();
  const { data: metrics, isLoading: metricsLoading } = useSystemMetrics();
  const riskData = useRiskData(transactions);
  const currencyData = useCurrencyDistribution(transactions);

  // Calculate analytics from real transactions
  const calculateAnalytics = () => {
    if (!transactions || transactions.length === 0) {
      return {
        totalTransactions: 0,
        totalVolume: 0,
        avgTransactionValue: 0,
        successRate: 0,
        failureRate: 0,
        pendingRate: 0,
        avgProcessingTime: 0,
        peakHour: 'N/A',
      };
    }

    const totalTransactions = transactions.length;
    const totalVolume = transactions.reduce((sum, tx) => {
      const amount = typeof tx.amount === 'string' ? parseFloat(tx.amount) : tx.amount;
      return sum + (amount || 0);
    }, 0);
    const avgTransactionValue = totalVolume / totalTransactions;

    const statusCounts = transactions.reduce((acc, tx) => {
      acc[tx.status] = (acc[tx.status] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    const successCount = statusCounts['completed'] || 0;
    const failureCount = statusCounts['failed'] || 0;
    const pendingCount = statusCounts['pending'] || 0;

    const successRate = (successCount / totalTransactions) * 100;
    const failureRate = (failureCount / totalTransactions) * 100;
    const pendingRate = (pendingCount / totalTransactions) * 100;

    // Calculate average processing time (if available)
    const avgProcessingTime = metrics?.avgProcessingTime || 0;

    // Find peak hour
    const hourCounts = transactions.reduce((acc, tx) => {
      if (tx.created_at) {
        const hour = new Date(tx.created_at).getHours();
        acc[hour] = (acc[hour] || 0) + 1;
      }
      return acc;
    }, {} as Record<number, number>);

    const peakHour = Object.entries(hourCounts).sort(([, a], [, b]) => b - a)[0]?.[0] || 'N/A';

    return {
      totalTransactions,
      totalVolume,
      avgTransactionValue,
      successRate,
      failureRate,
      pendingRate,
      avgProcessingTime,
      peakHour: peakHour !== 'N/A' ? `${peakHour}:00` : 'N/A',
    };
  };

  const analytics = calculateAnalytics();

  const isLoading = transactionsLoading || metricsLoading;

  return (
    <div className="min-h-screen">
      <main className="container mx-auto px-8 py-12">
        {/* Hero Title */}
        <FadeIn direction="up" delay={0}>
          <div className="mb-12">
            <div className="flex items-center gap-4 mb-4">
              <motion.div
                animate={{
                  rotate: [0, 360],
                }}
                transition={{
                  duration: 20,
                  repeat: Infinity,
                  ease: 'linear',
                }}
                className="p-4 rounded-2xl bg-gradient-to-br from-deltran-gold/20 to-deltran-gold-dark/20 backdrop-blur-xl border border-deltran-gold/30"
              >
                <BarChart3 size={32} className="text-deltran-gold" />
              </motion.div>
              <div>
                <h1 className="text-5xl md:text-6xl font-bold text-gradient-gold font-serif">
                  Analytics
                </h1>
                <p className="text-xl text-zinc-400 mt-2">
                  Advanced payment intelligence and business insights
                </p>
              </div>
            </div>
          </div>
        </FadeIn>

        {/* Key Metrics Grid */}
        <SectionReveal delay={0.1}>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-12">
            <MetricCard
              title="Total Transactions"
              value={isLoading ? '--' : analytics.totalTransactions.toLocaleString()}
              change={metrics?.changePayments ? `${metrics.changePayments}` : 'Real-time data'}
              icon={<Activity size={24} />}
              trend="up"
              delay={0}
            />
            <MetricCard
              title="Total Volume"
              value={
                isLoading
                  ? '--'
                  : `$${(analytics.totalVolume / 1000000).toFixed(2)}M`
              }
              change={metrics?.changeVolume ? `${metrics.changeVolume}` : 'Real-time data'}
              icon={<DollarSign size={24} />}
              trend="up"
              delay={0.1}
            />
            <MetricCard
              title="Avg Transaction"
              value={
                isLoading
                  ? '--'
                  : `$${analytics.avgTransactionValue.toLocaleString(undefined, {
                      maximumFractionDigits: 0,
                    })}`
              }
              change="Per transaction"
              icon={<TrendingUp size={24} />}
              trend="neutral"
              delay={0.2}
            />
            <MetricCard
              title="Success Rate"
              value={isLoading ? '--' : `${analytics.successRate.toFixed(1)}%`}
              change={`${analytics.failureRate.toFixed(1)}% failed`}
              icon={<Target size={24} />}
              trend="up"
              delay={0.3}
            />
          </div>
        </SectionReveal>

        {/* Status Distribution */}
        <SectionReveal delay={0.2}>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
            <PremiumCard className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-zinc-300">Completed</h3>
                <CheckCircle className="text-green-400" size={24} />
              </div>
              <div className="text-4xl font-bold text-gradient-gold mb-2">
                {isLoading ? '--' : `${analytics.successRate.toFixed(1)}%`}
              </div>
              <p className="text-sm text-zinc-500">
                {isLoading
                  ? '--'
                  : `${Math.round((analytics.successRate / 100) * analytics.totalTransactions)} transactions`}
              </p>
            </PremiumCard>

            <PremiumCard className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-zinc-300">Pending</h3>
                <Clock className="text-yellow-400" size={24} />
              </div>
              <div className="text-4xl font-bold text-gradient-gold mb-2">
                {isLoading ? '--' : `${analytics.pendingRate.toFixed(1)}%`}
              </div>
              <p className="text-sm text-zinc-500">
                {isLoading
                  ? '--'
                  : `${Math.round((analytics.pendingRate / 100) * analytics.totalTransactions)} transactions`}
              </p>
            </PremiumCard>

            <PremiumCard className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-zinc-300">Failed</h3>
                <XCircle className="text-red-400" size={24} />
              </div>
              <div className="text-4xl font-bold text-gradient-gold mb-2">
                {isLoading ? '--' : `${analytics.failureRate.toFixed(1)}%`}
              </div>
              <p className="text-sm text-zinc-500">
                {isLoading
                  ? '--'
                  : `${Math.round((analytics.failureRate / 100) * analytics.totalTransactions)} transactions`}
              </p>
            </PremiumCard>
          </div>
        </SectionReveal>

        {/* Performance Insights */}
        <SectionReveal delay={0.3}>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-12">
            <GlassCard className="p-6">
              <h3 className="text-xl font-bold text-gradient-gold mb-6">
                Performance Insights
              </h3>
              <div className="space-y-4">
                <div className="flex items-center justify-between p-4 rounded-xl bg-white/5">
                  <div className="flex items-center gap-3">
                    <Clock className="text-deltran-gold" size={20} />
                    <span className="text-zinc-300">Avg Processing Time</span>
                  </div>
                  <span className="text-xl font-bold text-white">
                    {isLoading ? '--' : `${analytics.avgProcessingTime}ms`}
                  </span>
                </div>
                <div className="flex items-center justify-between p-4 rounded-xl bg-white/5">
                  <div className="flex items-center gap-3">
                    <Activity className="text-deltran-gold" size={20} />
                    <span className="text-zinc-300">Peak Hour</span>
                  </div>
                  <span className="text-xl font-bold text-white">
                    {isLoading ? '--' : analytics.peakHour}
                  </span>
                </div>
                <div className="flex items-center justify-between p-4 rounded-xl bg-white/5">
                  <div className="flex items-center gap-3">
                    <Target className="text-deltran-gold" size={20} />
                    <span className="text-zinc-300">Settlement Rate</span>
                  </div>
                  <span className="text-xl font-bold text-white">
                    {isLoading ? '--' : `${metrics?.settlementRate || 0}%`}
                  </span>
                </div>
              </div>
            </GlassCard>

            <GlassCard className="p-6">
              <h3 className="text-xl font-bold text-gradient-gold mb-6">
                Risk Analysis
              </h3>
              <div className="space-y-4">
                <div className="flex items-center justify-between p-4 rounded-xl bg-white/5">
                  <div className="flex items-center gap-3">
                    <AlertTriangle className="text-amber-400" size={20} />
                    <span className="text-zinc-300">High Risk</span>
                  </div>
                  <span className="text-xl font-bold text-amber-400">
                    {isLoading ? '--' : riskData.cells.filter((c) => c.score > 70).length}
                  </span>
                </div>
                <div className="flex items-center justify-between p-4 rounded-xl bg-white/5">
                  <div className="flex items-center gap-3">
                    <Activity className="text-yellow-400" size={20} />
                    <span className="text-zinc-300">Medium Risk</span>
                  </div>
                  <span className="text-xl font-bold text-yellow-400">
                    {isLoading
                      ? '--'
                      : riskData.cells.filter((c) => c.score > 40 && c.score <= 70).length}
                  </span>
                </div>
                <div className="flex items-center justify-between p-4 rounded-xl bg-white/5">
                  <div className="flex items-center gap-3">
                    <CheckCircle className="text-green-400" size={20} />
                    <span className="text-zinc-300">Low Risk</span>
                  </div>
                  <span className="text-xl font-bold text-green-400">
                    {isLoading ? '--' : riskData.cells.filter((c) => c.score <= 40).length}
                  </span>
                </div>
              </div>
            </GlassCard>
          </div>
        </SectionReveal>

        {/* Charts Section */}
        <SectionReveal delay={0.4}>
          <GlassCard className="mb-12 p-6">
            <h3 className="text-2xl font-bold text-gradient-gold mb-6">
              Historical Metrics
            </h3>
            {isLoading ? (
              <div className="h-64 flex items-center justify-center">
                <div className="text-zinc-500">Loading chart data...</div>
              </div>
            ) : (
              <DailyMetricsCharts />
            )}
          </GlassCard>
        </SectionReveal>

        {/* Risk Heatmap & Currency Distribution */}
        <SectionReveal delay={0.5}>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-12">
            <GlassCard className="p-6">
              <h3 className="text-2xl font-bold text-gradient-gold mb-6">
                Risk Heatmap
              </h3>
              {isLoading ? (
                <div className="h-64 flex items-center justify-center">
                  <div className="text-zinc-500">Loading risk data...</div>
                </div>
              ) : (
                <RiskHeatmap cells={riskData.cells} maxScore={riskData.maxScore} />
              )}
            </GlassCard>

            <GlassCard className="p-6">
              <h3 className="text-2xl font-bold text-gradient-gold mb-6">
                Currency Distribution
              </h3>
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
        </SectionReveal>
      </main>
    </div>
  );
}
