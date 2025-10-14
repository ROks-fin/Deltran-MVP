'use client'

import { motion } from 'framer-motion'
import { Transaction } from '@/app/types/transaction'
import { useFlowData } from '@/app/hooks/useFlowData'
import { FlowNode } from './FlowNode'
import { FlowParticle } from './FlowParticle'

interface PaymentFlowProps {
  transactions?: Transaction[]
  onNodeClick?: (status: string) => void
}

export function PaymentFlow({ transactions = [], onNodeClick }: PaymentFlowProps) {
  const flowData = useFlowData(transactions)

  return (
    <div className="rounded-xl p-6 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-xl font-semibold text-white">Payment Pipeline</h3>
        <div className="flex items-center gap-4">
          <div className="text-sm text-zinc-400">
            Flow Rate: <span className="text-deltran-gold font-mono">{flowData.flowRate.toFixed(1)}</span> tx/s
          </div>
          <div className="flex items-center gap-2">
            <motion.div
              animate={{ scale: [1, 1.2, 1], opacity: [0.5, 1, 0.5] }}
              transition={{ duration: 2, repeat: Infinity }}
              className="w-2 h-2 rounded-full bg-green-400"
            />
            <span className="text-sm text-zinc-400">
              Live · {transactions.length}
            </span>
          </div>
        </div>
      </div>

      {/* Flow visualization */}
      <div className="relative">
        {/* Scrollable container */}
        <div className="overflow-x-auto pb-4">
          <div className="relative min-w-[1200px] h-[160px] flex items-center">
            {/* Connection lines */}
            <svg
              className="absolute inset-0 w-full h-full pointer-events-none"
              style={{ zIndex: 0 }}
            >
              <defs>
                <linearGradient id="lineGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                  <stop offset="0%" stopColor="#d4af37" stopOpacity="0.2" />
                  <stop offset="50%" stopColor="#d4af37" stopOpacity="0.5" />
                  <stop offset="100%" stopColor="#d4af37" stopOpacity="0.2" />
                </linearGradient>
              </defs>

              {/* Lines between nodes */}
              {flowData.nodes.slice(0, -1).map((node, index) => {
                const startX = 40 + index * 120 + 60 // center of current node
                const endX = 40 + (index + 1) * 120 // start of next node
                const y = 80 // middle height

                return (
                  <g key={`line-${index}`}>
                    {/* Main line */}
                    <motion.line
                      x1={startX}
                      y1={y}
                      x2={endX}
                      y2={y}
                      stroke="url(#lineGradient)"
                      strokeWidth="2"
                      strokeDasharray="5,5"
                      initial={{ pathLength: 0 }}
                      animate={{ pathLength: 1 }}
                      transition={{ duration: 1, delay: index * 0.1 }}
                    />

                    {/* Animated flow dot */}
                    <motion.circle
                      r="3"
                      fill="#d4af37"
                      animate={{
                        cx: [startX, endX],
                      }}
                      transition={{
                        duration: 2,
                        repeat: Infinity,
                        ease: 'linear',
                        delay: index * 0.3,
                      }}
                      cy={y}
                    />
                  </g>
                )
              })}
            </svg>

            {/* Nodes */}
            <div className="relative flex gap-[60px] px-10" style={{ zIndex: 1 }}>
              {flowData.nodes.map((node, index) => (
                <motion.div
                  key={node.status}
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: index * 0.05, duration: 0.3 }}
                >
                  <FlowNode
                    status={node.status}
                    count={node.count}
                    volume={node.volume}
                    isActive={node.count > 0}
                    onClick={() => onNodeClick?.(node.status)}
                  />
                </motion.div>
              ))}
            </div>

            {/* Particles layer */}
            <div className="absolute inset-0 pointer-events-none" style={{ zIndex: 2 }}>
              {transactions.slice(0, 8).map((tx, index) => (
                <FlowParticle
                  key={`${tx.payment_id}-${index}`}
                  currency={tx.currency}
                  amount={parseFloat(tx.amount)}
                  delay={index * 0.8}
                  duration={10}
                />
              ))}
            </div>
          </div>
        </div>

        {/* Bottleneck warning */}
        {flowData.bottleneck && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-4 p-3 rounded-lg bg-red-500/10 border border-red-500/30 flex items-center gap-2"
          >
            <div className="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
            <span className="text-sm text-red-400">
              Bottleneck detected at <span className="font-semibold">{flowData.bottleneck}</span> stage
            </span>
          </motion.div>
        )}

        {/* Currency legend */}
        <div className="mt-4 flex flex-wrap gap-3 text-xs">
          <div className="flex items-center gap-1.5">
            <div className="w-2 h-2 rounded-full bg-[#4ade80]" />
            <span className="text-zinc-500">USD</span>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="w-2 h-2 rounded-full bg-[#60a5fa]" />
            <span className="text-zinc-500">EUR</span>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="w-2 h-2 rounded-full bg-[#a78bfa]" />
            <span className="text-zinc-500">GBP</span>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="w-2 h-2 rounded-full bg-[#fbbf24]" />
            <span className="text-zinc-500">AED</span>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="w-2 h-2 rounded-full bg-[#fb923c]" />
            <span className="text-zinc-500">INR</span>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="w-2 h-2 rounded-full bg-[#f87171]" />
            <span className="text-zinc-500">PKR</span>
          </div>
        </div>
      </div>
    </div>
  )
}
