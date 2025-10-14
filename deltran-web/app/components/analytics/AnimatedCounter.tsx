'use client'

import { useEffect, useState } from 'react'
import { motion, useSpring, useMotionValue } from 'framer-motion'

export interface AnimatedCounterProps {
  value: number
  duration?: number
  format?: 'currency' | 'number' | 'percentage'
  prefix?: string
  suffix?: string
  highlightChange?: boolean
  decimals?: number
}

export function AnimatedCounter({
  value,
  duration = 800,
  format = 'number',
  prefix = '',
  suffix = '',
  highlightChange = true,
  decimals = 2,
}: AnimatedCounterProps) {
  const [prevValue, setPrevValue] = useState(value)
  const [isIncreasing, setIsIncreasing] = useState<boolean | null>(null)
  const [displayValue, setDisplayValue] = useState(value)

  // Framer Motion spring animation
  const spring = useSpring(value, {
    stiffness: 100,
    damping: 30,
    mass: 0.5,
  })

  // Update display value as spring changes
  useEffect(() => {
    const unsubscribe = spring.on('change', (latest) => {
      setDisplayValue(latest)
    })
    return () => unsubscribe()
  }, [spring])

  const formatValue = (num: number): string => {
    const rounded = Number(num.toFixed(decimals))

    switch (format) {
      case 'currency':
        return new Intl.NumberFormat('en-US', {
          style: 'currency',
          currency: 'USD',
          minimumFractionDigits: decimals,
          maximumFractionDigits: decimals,
        }).format(rounded)
      case 'percentage':
        return `${rounded.toFixed(decimals)}%`
      case 'number':
      default:
        return new Intl.NumberFormat('en-US', {
          minimumFractionDigits: decimals,
          maximumFractionDigits: decimals,
        }).format(rounded)
    }
  }

  useEffect(() => {
    if (value !== prevValue) {
      setIsIncreasing(value > prevValue)
      spring.set(value)
      setPrevValue(value)

      // Reset highlight after animation
      if (highlightChange) {
        const timer = setTimeout(() => setIsIncreasing(null), duration + 200)
        return () => clearTimeout(timer)
      }
    }
  }, [value, prevValue, spring, duration, highlightChange])

  return (
    <div className="relative inline-block">
      <motion.span
        className="relative z-10"
        animate={
          highlightChange && isIncreasing !== null
            ? {
                color: isIncreasing
                  ? ['#ffffff', '#d4af37', '#ffffff']
                  : ['#ffffff', '#ef4444', '#ffffff'],
              }
            : {}
        }
        transition={{ duration: 0.6 }}
      >
        {prefix}
        <span>{formatValue(displayValue)}</span>
        {suffix}
      </motion.span>

      {/* Glow effect on increase */}
      {highlightChange && isIncreasing === true && (
        <motion.div
          initial={{ opacity: 0, scale: 0.8 }}
          animate={{ opacity: [0, 0.6, 0], scale: [0.8, 1.2, 1.4] }}
          transition={{ duration: 0.8 }}
          className="absolute inset-0 rounded-lg blur-xl bg-deltran-gold -z-10"
        />
      )}

      {/* Flash effect on decrease */}
      {highlightChange && isIncreasing === false && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: [0, 0.4, 0] }}
          transition={{ duration: 0.6 }}
          className="absolute inset-0 rounded-lg blur-lg bg-red-500 -z-10"
        />
      )}
    </div>
  )
}
