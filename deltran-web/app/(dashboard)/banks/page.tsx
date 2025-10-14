'use client'

import { useState } from 'react'
import { motion } from 'framer-motion'
import { Building2, TrendingUp, ArrowUpRight, ArrowDownRight, Activity, Medal } from 'lucide-react'
import { ProtectedRoute } from '@/app/components/auth/ProtectedRoute'
import { useBanksMetrics, BankMetric } from '@/app/hooks/useBanksMetrics'

function BanksPageContent() {
  const { data: banks = [], isLoading } = useBanksMetrics()
  const [selectedBank, setSelectedBank] = useState<BankMetric | null>(null)

  // Calculate totals
  const totalSent = banks.reduce((sum, b) => sum + b.sent_count, 0)
  const totalReceived = banks.reduce((sum, b) => sum + b.received_count, 0)
  const totalVolume = banks.reduce((sum, b) => sum + b.sent_volume + b.received_volume, 0)

  // Top 3 banks by volume
  const topBanks = [...banks]
    .sort((a, b) => (b.sent_volume + b.received_volume) - (a.sent_volume + a.received_volume))
    .slice(0, 3)

  if (isLoading) {
    return (
      <div className="min-h-screen bg-deltran-dark p-6">
        <div className="container mx-auto">
          <div className="h-20 bg-zinc-800/30 rounded-xl animate-pulse mb-8" />
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-32 bg-zinc-800/30 rounded-xl animate-pulse" />
            ))}
          </div>
          <div className="h-[500px] bg-zinc-800/30 rounded-xl animate-pulse" />
        </div>
      </div>
    )
  }

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
              <Building2 className="w-8 h-8 text-deltran-gold" />
            </div>
            <div>
              <h1 className="text-4xl font-bold text-white">Banks Dashboard</h1>
              <p className="text-zinc-400 mt-1">Monitor participating financial institutions</p>
            </div>
          </div>
        </motion.div>

        {/* Summary Stats */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8"
        >
          <div className="p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
            <div className="flex items-center justify-between mb-4">
              <div className="p-2 rounded-lg bg-blue-500/10 border border-blue-500/30">
                <Building2 className="w-5 h-5 text-blue-400" />
              </div>
              <span className="text-2xl font-bold text-white">{banks.length}</span>
            </div>
            <p className="text-sm text-zinc-400">Active Banks</p>
          </div>

          <div className="p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
            <div className="flex items-center justify-between mb-4">
              <div className="p-2 rounded-lg bg-green-500/10 border border-green-500/30">
                <Activity className="w-5 h-5 text-green-400" />
              </div>
              <span className="text-2xl font-bold text-white">{totalSent + totalReceived}</span>
            </div>
            <p className="text-sm text-zinc-400">Total Transactions (24h)</p>
          </div>

          <div className="p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
            <div className="flex items-center justify-between mb-4">
              <div className="p-2 rounded-lg bg-deltran-gold/10 border border-deltran-gold/30">
                <TrendingUp className="w-5 h-5 text-deltran-gold" />
              </div>
              <span className="text-2xl font-bold text-white">
                ${(totalVolume / 1000000).toFixed(1)}M
              </span>
            </div>
            <p className="text-sm text-zinc-400">Total Volume (24h)</p>
          </div>
        </motion.div>

        {/* Top Banks Widget */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className="mb-8 p-6 rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800"
        >
          <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
            <Medal className="w-5 h-5 text-deltran-gold" />
            Top 3 Banks by Volume
          </h3>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {topBanks.map((bank, index) => {
              const medals = ['🥇', '🥈', '🥉']
              const totalBankVolume = bank.sent_volume + bank.received_volume

              return (
                <div
                  key={bank.bank_id}
                  className="p-4 rounded-lg bg-zinc-800/30 border border-zinc-800 hover:border-deltran-gold/30 transition-all"
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-2xl">{medals[index]}</span>
                    <span className="text-xs text-zinc-500 font-mono">{bank.bic}</span>
                  </div>
                  <p className="text-sm font-semibold text-white mb-1">{bank.bank_name}</p>
                  <p className="text-lg font-bold text-deltran-gold">
                    ${(totalBankVolume / 1000000).toFixed(2)}M
                  </p>
                </div>
              )
            })}
          </div>
        </motion.div>

        {/* Banks Table */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
          className="rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 overflow-hidden"
        >
          <div className="p-6 border-b border-zinc-800">
            <h3 className="text-xl font-semibold text-white">All Participating Banks</h3>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800 bg-zinc-900/50">
                  <th className="text-left py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    Bank
                  </th>
                  <th className="text-left py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    BIC Code
                  </th>
                  <th className="text-left py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    Country
                  </th>
                  <th className="text-right py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    Sent (24h)
                  </th>
                  <th className="text-right py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    Received (24h)
                  </th>
                  <th className="text-right py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    Total Volume
                  </th>
                  <th className="text-center py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    Flow
                  </th>
                  <th className="text-center py-4 px-6 text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    Status
                  </th>
                </tr>
              </thead>
              <tbody>
                {banks.map((bank, index) => {
                  const totalVolume = bank.sent_volume + bank.received_volume
                  const netFlow = bank.sent_volume - bank.received_volume
                  const sentPercentage = (bank.sent_volume / totalVolume) * 100
                  const receivedPercentage = (bank.received_volume / totalVolume) * 100

                  return (
                    <motion.tr
                      key={bank.bank_id}
                      initial={{ opacity: 0, x: -20 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: index * 0.05, duration: 0.3 }}
                      className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors cursor-pointer group"
                      onClick={() => setSelectedBank(bank)}
                    >
                      <td className="py-4 px-6">
                        <div className="flex items-center gap-3">
                          <div className="p-2 rounded-lg bg-blue-500/10 border border-blue-500/30">
                            <Building2 className="w-4 h-4 text-blue-400" />
                          </div>
                          <div>
                            <p className="text-sm font-semibold text-white">{bank.bank_name}</p>
                            <p className="text-xs text-zinc-500">{bank.country}</p>
                          </div>
                        </div>
                      </td>

                      <td className="py-4 px-6">
                        <span className="text-sm font-mono text-zinc-300">{bank.bic}</span>
                      </td>

                      <td className="py-4 px-6">
                        <span className="text-2xl">{getCountryFlag(bank.country)}</span>
                      </td>

                      <td className="py-4 px-6 text-right">
                        <div className="flex items-center justify-end gap-2">
                          <ArrowUpRight className="w-3 h-3 text-blue-400" />
                          <span className="text-sm font-semibold text-white">{bank.sent_count}</span>
                        </div>
                        <p className="text-xs text-zinc-500">
                          ${(bank.sent_volume / 1000000).toFixed(2)}M
                        </p>
                      </td>

                      <td className="py-4 px-6 text-right">
                        <div className="flex items-center justify-end gap-2">
                          <ArrowDownRight className="w-3 h-3 text-green-400" />
                          <span className="text-sm font-semibold text-white">{bank.received_count}</span>
                        </div>
                        <p className="text-xs text-zinc-500">
                          ${(bank.received_volume / 1000000).toFixed(2)}M
                        </p>
                      </td>

                      <td className="py-4 px-6 text-right">
                        <p className="text-sm font-bold text-deltran-gold">
                          ${(totalVolume / 1000000).toFixed(2)}M
                        </p>
                      </td>

                      <td className="py-4 px-6">
                        <div className="flex items-center gap-2">
                          <div className="flex-1 h-2 bg-zinc-800 rounded-full overflow-hidden">
                            <div className="h-full flex">
                              <div
                                className="bg-blue-400"
                                style={{ width: `${sentPercentage}%` }}
                              />
                              <div
                                className="bg-green-400"
                                style={{ width: `${receivedPercentage}%` }}
                              />
                            </div>
                          </div>
                        </div>
                        <p className="text-xs text-zinc-500 text-center mt-1">
                          {netFlow > 0 ? `+$${(netFlow / 1000000).toFixed(1)}M` : `-$${(Math.abs(netFlow) / 1000000).toFixed(1)}M`}
                        </p>
                      </td>

                      <td className="py-4 px-6 text-center">
                        <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-semibold bg-green-500/10 text-green-400 border border-green-500/30">
                          <span className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse" />
                          {bank.status}
                        </span>
                      </td>
                    </motion.tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        </motion.div>
      </div>
    </div>
  )
}

function getCountryFlag(countryCode: string): string {
  const flags: Record<string, string> = {
    US: '🇺🇸',
    GB: '🇬🇧',
    DE: '🇩🇪',
    AE: '🇦🇪',
    FR: '🇫🇷',
    JP: '🇯🇵',
    CH: '🇨🇭',
    SG: '🇸🇬',
  }
  return flags[countryCode] || '🏦'
}

export default function BanksPage() {
  return (
    <ProtectedRoute>
      <BanksPageContent />
    </ProtectedRoute>
  )
}
