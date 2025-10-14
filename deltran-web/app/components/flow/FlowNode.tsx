'use client'

import { motion } from 'framer-motion'
import { PaymentStatus, STATUS_COLORS } from '@/app/types/transaction'

interface FlowNodeProps {
  status: PaymentStatus
  count: number
  volume: number
  isActive?: boolean
  onClick?: () => void
}

export function FlowNode({ status, count, volume, isActive, onClick }: FlowNodeProps) {
  const color = STATUS_COLORS[status]
  const isBottleneck = count > 50 // Перегруженный узел

  return (
    <div className="flex flex-col items-center gap-2 min-w-[80px]">
      {/* Node circle */}
      <motion.div
        whileHover={{ scale: 1.15 }}
        whileTap={{ scale: 0.95 }}
        animate={
          isBottleneck
            ? {
                boxShadow: [
                  `0 0 0px ${color}00`,
                  `0 0 30px ${color}80`,
                  `0 0 0px ${color}00`,
                ],
              }
            : isActive
            ? {
                boxShadow: [
                  `0 0 0px ${color}00`,
                  `0 0 20px ${color}60`,
                  `0 0 0px ${color}00`,
                ],
              }
            : {}
        }
        transition={{
          duration: isBottleneck ? 1 : 2,
          repeat: Infinity,
          ease: 'easeInOut',
        }}
        onClick={onClick}
        className="relative w-[60px] h-[60px] rounded-full flex flex-col items-center justify-center cursor-pointer group"
        style={{
          backgroundColor: `${color}20`,
          border: `2px solid ${color}`,
        }}
      >
        {/* Count */}
        <motion.span
          key={count}
          initial={{ scale: 1.5, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          className="text-xl font-bold"
          style={{ color }}
        >
          {count}
        </motion.span>

        {/* Status abbreviation */}
        <span className="text-[8px] font-mono text-zinc-500 uppercase">
          {status.slice(0, 4)}
        </span>

        {/* Glow effect on hover */}
        <div
          className="absolute inset-0 rounded-full opacity-0 group-hover:opacity-100 transition-opacity duration-300"
          style={{
            background: `radial-gradient(circle, ${color}40 0%, transparent 70%)`,
          }}
        />

        {/* Activity indicator */}
        {count > 0 && (
          <motion.div
            className="absolute -top-1 -right-1 w-3 h-3 rounded-full"
            style={{ backgroundColor: color }}
            animate={{
              scale: [1, 1.3, 1],
              opacity: [0.7, 1, 0.7],
            }}
            transition={{
              duration: 1.5,
              repeat: Infinity,
            }}
          />
        )}
      </motion.div>

      {/* Label */}
      <div className="text-center">
        <p className="text-xs font-medium text-zinc-300">{status}</p>
        <p className="text-[10px] text-zinc-500 font-mono">
          ${(volume / 1000).toFixed(0)}K
        </p>
      </div>
    </div>
  )
}
