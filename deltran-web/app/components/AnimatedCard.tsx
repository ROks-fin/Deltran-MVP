'use client'

import { motion } from 'framer-motion'
import { LucideIcon } from 'lucide-react'
import { AnimatedCounter } from './analytics/AnimatedCounter'

interface AnimatedCardProps {
  title: string
  value: string | number
  change: string
  icon: LucideIcon
  delay?: number
  numericValue?: number
  format?: 'currency' | 'number' | 'percentage'
  sparklineData?: number[]
}

export function AnimatedCard({
  title,
  value,
  change,
  icon: Icon,
  delay = 0,
  numericValue,
  format = 'number',
  sparklineData
}: AnimatedCardProps) {
  const isPositive = change.startsWith('↑')
  const useAnimatedCounter = numericValue !== undefined

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        duration: 0.5,
        delay,
        type: 'spring',
        stiffness: 100
      }}
      whileHover={{
        scale: 1.02,
        transition: { type: 'spring', stiffness: 300, damping: 20 }
      }}
      className="group relative overflow-hidden rounded-xl p-6 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 hover:border-deltran-gold/50 transition-all duration-300"
    >
      {/* Gold glow on hover */}
      <div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300">
        <div className="absolute inset-0 bg-gradient-to-br from-deltran-gold/10 via-transparent to-transparent" />
      </div>

      {/* Content */}
      <div className="relative z-10">
        <div className="flex justify-between mb-4">
          <h3 className="text-sm text-zinc-400 uppercase tracking-wider">{title}</h3>
          <motion.div
            whileHover={{ rotate: 360 }}
            transition={{ duration: 0.6, ease: 'easeInOut' }}
            className="p-2 rounded-lg bg-zinc-800/50 group-hover:bg-deltran-gold/10 transition-colors"
          >
            <Icon className="w-5 h-5 text-zinc-400 group-hover:text-deltran-gold transition-colors" />
          </motion.div>
        </div>

        <motion.div
          initial={{ scale: 0.8, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={{ delay: delay + 0.2, duration: 0.4 }}
          className="text-3xl font-bold text-white mb-2"
        >
          {useAnimatedCounter ? (
            <AnimatedCounter
              value={numericValue}
              format={format}
              highlightChange={true}
              decimals={format === 'currency' ? 1 : 0}
            />
          ) : (
            value
          )}
        </motion.div>

        <motion.div
          initial={{ opacity: 0, x: -10 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: delay + 0.3, duration: 0.3 }}
          className={`text-sm ${isPositive ? 'text-green-400' : 'text-red-400'}`}
        >
          {change}
        </motion.div>
      </div>

      {/* Pulsating gold line for positive changes */}
      {isPositive && (
        <motion.div
          animate={{
            opacity: [0.3, 0.6, 0.3],
          }}
          transition={{
            duration: 2,
            repeat: Infinity,
            ease: 'easeInOut',
          }}
          className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-transparent via-deltran-gold to-transparent"
        />
      )}
    </motion.div>
  )
}
