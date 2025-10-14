'use client'

import { motion } from 'framer-motion'
import { Check, X, Zap, Star, Loader2 } from 'lucide-react'
import { PaymentStatus, STATUS_COLORS } from '@/app/types/transaction'

interface StatusBadgeProps {
  status: PaymentStatus
}

export function StatusBadge({ status }: StatusBadgeProps) {
  const color = STATUS_COLORS[status]

  // Анимации для каждого статуса
  const getStatusAnimation = () => {
    switch (status) {
      case 'Initiated':
        return (
          <motion.div
            className="flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div
              animate={{ scale: [1, 1.3, 1], opacity: [0.5, 1, 0.5] }}
              transition={{ duration: 1.5, repeat: Infinity }}
              className="w-2 h-2 rounded-full"
              style={{ backgroundColor: color }}
            />
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Validated':
        return (
          <motion.div
            className="flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div
              initial={{ scale: 0 }}
              animate={{ scale: [0, 1.2, 1] }}
              transition={{ duration: 0.5, times: [0, 0.6, 1] }}
            >
              <Check className="w-3 h-3" style={{ color }} />
            </motion.div>
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Screened':
        return (
          <motion.div
            className="relative flex items-center gap-2 px-3 py-1 rounded-full overflow-hidden"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div
              className="absolute inset-0"
              animate={{ x: ['-100%', '100%'] }}
              transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}
              style={{
                background: `linear-gradient(90deg, transparent, ${color}40, transparent)`,
                width: '30%',
              }}
            />
            <span className="text-xs font-mono relative z-10" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Approved':
        return (
          <motion.div
            className="flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div
              animate={{ rotate: 360, scale: [1, 1.2, 1] }}
              transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}
            >
              <Star className="w-3 h-3 fill-current" style={{ color: '#d4af37' }} />
            </motion.div>
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Queued':
        return (
          <motion.div
            className="flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <div className="flex gap-0.5">
              {[0, 1, 2].map((i) => (
                <motion.div
                  key={i}
                  className="w-1 h-1 rounded-full"
                  style={{ backgroundColor: color }}
                  animate={{ opacity: [0.3, 1, 0.3] }}
                  transition={{
                    duration: 1.5,
                    repeat: Infinity,
                    delay: i * 0.2,
                  }}
                />
              ))}
            </div>
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Settling':
        return (
          <motion.div
            className="flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div animate={{ rotate: 360 }} transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}>
              <Loader2 className="w-3 h-3" style={{ color }} />
            </motion.div>
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Settled':
        return (
          <motion.div
            className="relative flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div
              className="absolute inset-0 rounded-full"
              animate={{ boxShadow: [`0 0 0px ${color}00`, `0 0 15px ${color}60`, `0 0 0px ${color}00`] }}
              transition={{ duration: 2, repeat: Infinity }}
            />
            <Check className="w-3 h-3 relative z-10" style={{ color }} />
            <span className="text-xs font-mono relative z-10" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Completed':
        return (
          <motion.div
            className="relative flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            {/* Confetti particles */}
            {[...Array(5)].map((_, i) => (
              <motion.div
                key={i}
                className="absolute w-1 h-1 rounded-full"
                style={{
                  backgroundColor: '#d4af37',
                  left: '50%',
                  top: '50%',
                }}
                animate={{
                  x: [0, (i - 2) * 15],
                  y: [0, -20 - i * 3],
                  opacity: [1, 0],
                  scale: [1, 0],
                }}
                transition={{
                  duration: 1,
                  repeat: Infinity,
                  delay: i * 0.1,
                  repeatDelay: 2,
                }}
              />
            ))}
            <Check className="w-3 h-3" style={{ color }} />
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Rejected':
        return (
          <motion.div
            className="flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
            animate={{ rotate: [-2, 2, -2] }}
            transition={{ duration: 0.5, repeat: 3, repeatDelay: 2 }}
          >
            <X className="w-3 h-3" style={{ color }} />
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Failed':
        return (
          <motion.div
            className="flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div
              animate={{ opacity: [1, 0, 1, 0, 1] }}
              transition={{ duration: 0.8, times: [0, 0.2, 0.4, 0.6, 1], repeat: Infinity, repeatDelay: 3 }}
            >
              <Zap className="w-3 h-3 fill-current" style={{ color }} />
            </motion.div>
            <span className="text-xs font-mono" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      case 'Cancelled':
        return (
          <motion.div
            className="relative flex items-center gap-2 px-3 py-1 rounded-full"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40` }}
          >
            <motion.div
              className="absolute inset-0 flex items-center justify-center"
              initial={{ scaleX: 0 }}
              animate={{ scaleX: 1 }}
              transition={{ duration: 0.5 }}
            >
              <div
                className="h-[1px] w-full"
                style={{ backgroundColor: color, transform: 'rotate(-10deg)' }}
              />
            </motion.div>
            <span className="text-xs font-mono relative z-10" style={{ color }}>
              {status}
            </span>
          </motion.div>
        )

      default:
        return (
          <div
            className="px-3 py-1 rounded-full text-xs font-mono"
            style={{ backgroundColor: `${color}20`, border: `1px solid ${color}40`, color }}
          >
            {status}
          </div>
        )
    }
  }

  return getStatusAnimation()
}
