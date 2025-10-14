'use client'

import { useState } from 'react'
import { motion } from 'framer-motion'
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts'
import { TrendingUp, BarChart3, Activity } from 'lucide-react'
import { useDailyMetrics } from '@/app/hooks/useDailyMetrics'

type TimePeriod = 7 | 30 | 90

export function DailyMetricsCharts() {
  const [period, setPeriod] = useState<TimePeriod>(30)
  const { data: metrics = [], isLoading } = useDailyMetrics(period)

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="h-[400px] bg-zinc-800/30 rounded-xl animate-pulse" />
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="h-[350px] bg-zinc-800/30 rounded-xl animate-pulse" />
          <div className="h-[350px] bg-zinc-800/30 rounded-xl animate-pulse" />
        </div>
      </div>
    )
  }

  // Show empty state if no data
  if (!metrics || metrics.length === 0) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-2xl font-bold text-white mb-1">Historical Trends</h3>
            <p className="text-sm text-zinc-400">No historical data available</p>
          </div>
        </div>

        <div className="p-12 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
          <div className="text-center">
            <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-zinc-800/50 flex items-center justify-center">
              <TrendingUp className="w-8 h-8 text-zinc-600" />
            </div>
            <h4 className="text-lg font-semibold text-white mb-2">No Historical Data</h4>
            <p className="text-sm text-zinc-400 max-w-md mx-auto">
              Historical metrics will appear here once the backend provides daily aggregated data.
              Connect to the metrics API endpoint to see payment trends over time.
            </p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header with Period Selector */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-2xl font-bold text-white mb-1">Historical Trends</h3>
          <p className="text-sm text-zinc-400">Last {period} days of payment activity</p>
        </div>

        {/* Period Selector */}
        <div className="flex gap-2">
          {[7, 30, 90].map((days) => (
            <button
              key={days}
              onClick={() => setPeriod(days as TimePeriod)}
              className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                period === days
                  ? 'bg-gradient-to-r from-deltran-gold to-deltran-gold-light text-black shadow-lg'
                  : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-white'
              }`}
            >
              {days}D
            </button>
          ))}
        </div>
      </div>

      {/* Chart 1: Payment Volume Trend (Line Chart) */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        className="p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800"
      >
        <div className="flex items-center gap-3 mb-6">
          <div className="p-2 rounded-lg bg-blue-500/10 border border-blue-500/30">
            <TrendingUp className="w-5 h-5 text-blue-400" />
          </div>
          <div>
            <h4 className="text-lg font-semibold text-white">Payment Volume Trend</h4>
            <p className="text-xs text-zinc-500">Total volume by currency over time</p>
          </div>
        </div>

        <ResponsiveContainer width="100%" height={350}>
          <LineChart data={metrics}>
            <defs>
              <linearGradient id="colorUSD" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#d4af37" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#d4af37" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="colorEUR" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
              </linearGradient>
              <linearGradient id="colorGBP" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#27272a" />
            <XAxis
              dataKey="date"
              stroke="#71717a"
              tick={{ fill: '#71717a', fontSize: 12 }}
              tickFormatter={(value) => {
                const date = new Date(value)
                return `${date.getMonth() + 1}/${date.getDate()}`
              }}
            />
            <YAxis
              stroke="#71717a"
              tick={{ fill: '#71717a', fontSize: 12 }}
              tickFormatter={(value) => `$${(value / 1000).toFixed(0)}k`}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: '#18181b',
                border: '1px solid #27272a',
                borderRadius: '8px',
                color: '#fff',
              }}
              formatter={(value: number) => [`$${value.toLocaleString()}`, '']}
              labelFormatter={(label) => new Date(label).toLocaleDateString()}
            />
            <Legend
              wrapperStyle={{ color: '#71717a' }}
              onClick={(e) => console.log('Legend clicked:', e)}
            />
            <Line
              type="monotone"
              dataKey="volume_usd"
              stroke="#d4af37"
              strokeWidth={2}
              dot={false}
              name="USD"
              fill="url(#colorUSD)"
            />
            <Line
              type="monotone"
              dataKey="volume_eur"
              stroke="#3b82f6"
              strokeWidth={2}
              dot={false}
              name="EUR"
              fill="url(#colorEUR)"
            />
            <Line
              type="monotone"
              dataKey="volume_gbp"
              stroke="#10b981"
              strokeWidth={2}
              dot={false}
              name="GBP"
              fill="url(#colorGBP)"
            />
          </LineChart>
        </ResponsiveContainer>
      </motion.div>

      {/* Bottom Row: Bar Chart + Area Chart */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Chart 2: Payment Count Trend (Bar Chart) */}
        <motion.div
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800"
        >
          <div className="flex items-center gap-3 mb-6">
            <div className="p-2 rounded-lg bg-purple-500/10 border border-purple-500/30">
              <BarChart3 className="w-5 h-5 text-purple-400" />
            </div>
            <div>
              <h4 className="text-lg font-semibold text-white">Payment Count</h4>
              <p className="text-xs text-zinc-500">Daily payment volume by status</p>
            </div>
          </div>

          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={metrics}>
              <CartesianGrid strokeDasharray="3 3" stroke="#27272a" />
              <XAxis
                dataKey="date"
                stroke="#71717a"
                tick={{ fill: '#71717a', fontSize: 11 }}
                tickFormatter={(value) => {
                  const date = new Date(value)
                  return `${date.getMonth() + 1}/${date.getDate()}`
                }}
              />
              <YAxis stroke="#71717a" tick={{ fill: '#71717a', fontSize: 11 }} />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#18181b',
                  border: '1px solid #27272a',
                  borderRadius: '8px',
                  color: '#fff',
                }}
                labelFormatter={(label) => new Date(label).toLocaleDateString()}
              />
              <Legend wrapperStyle={{ color: '#71717a', fontSize: '12px' }} />
              <Bar dataKey="settled" stackId="a" fill="#10b981" name="Settled" />
              <Bar dataKey="pending" stackId="a" fill="#f59e0b" name="Pending" />
              <Bar dataKey="failed" stackId="a" fill="#ef4444" name="Failed" />
            </BarChart>
          </ResponsiveContainer>
        </motion.div>

        {/* Chart 3: Success Rate Trend (Area Chart) */}
        <motion.div
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800"
        >
          <div className="flex items-center gap-3 mb-6">
            <div className="p-2 rounded-lg bg-green-500/10 border border-green-500/30">
              <Activity className="w-5 h-5 text-green-400" />
            </div>
            <div>
              <h4 className="text-lg font-semibold text-white">Success Rate</h4>
              <p className="text-xs text-zinc-500">Settlement success percentage</p>
            </div>
          </div>

          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={metrics}>
              <defs>
                <linearGradient id="colorSuccess" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#10b981" stopOpacity={0.4} />
                  <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#27272a" />
              <XAxis
                dataKey="date"
                stroke="#71717a"
                tick={{ fill: '#71717a', fontSize: 11 }}
                tickFormatter={(value) => {
                  const date = new Date(value)
                  return `${date.getMonth() + 1}/${date.getDate()}`
                }}
              />
              <YAxis
                stroke="#71717a"
                tick={{ fill: '#71717a', fontSize: 11 }}
                domain={[80, 100]}
                tickFormatter={(value) => `${value}%`}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#18181b',
                  border: '1px solid #27272a',
                  borderRadius: '8px',
                  color: '#fff',
                }}
                formatter={(value: number) => [`${value.toFixed(2)}%`, 'Success Rate']}
                labelFormatter={(label) => new Date(label).toLocaleDateString()}
              />
              <Area
                type="monotone"
                dataKey="success_rate"
                stroke="#10b981"
                strokeWidth={2}
                fill="url(#colorSuccess)"
              />
              {/* Target line at 95% */}
              <line
                x1="0%"
                y1="95"
                x2="100%"
                y2="95"
                stroke="#f59e0b"
                strokeDasharray="5 5"
                strokeWidth={1}
              />
            </AreaChart>
          </ResponsiveContainer>

          <div className="mt-4 flex items-center justify-center gap-4 text-xs">
            <div className="flex items-center gap-2">
              <div className="w-3 h-0.5 bg-green-400" />
              <span className="text-zinc-400">Success Rate</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-0.5 bg-yellow-400 opacity-60" style={{ borderTop: '1px dashed' }} />
              <span className="text-zinc-400">Target (95%)</span>
            </div>
          </div>
        </motion.div>
      </div>
    </div>
  )
}
