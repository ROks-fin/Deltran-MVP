'use client'

import { motion } from 'framer-motion'
import { Currency } from '@/app/types/transaction'

interface FlowParticleProps {
  currency: Currency
  amount: number
  delay: number
  duration: number
}

// Currency color mapping
const CURRENCY_COLORS: Record<Currency, string> = {
  USD: '#4ade80', // зеленый
  EUR: '#60a5fa', // синий
  GBP: '#a78bfa', // фиолетовый
  AED: '#fbbf24', // желтый
  INR: '#fb923c', // оранжевый
  PKR: '#f87171', // красный
  ILS: '#60a5fa', // голубой
}

export function FlowParticle({ currency, amount, delay, duration }: FlowParticleProps) {
  const color = CURRENCY_COLORS[currency]

  // Size based on amount (log scale for balance)
  const size = Math.min(Math.max(Math.log10(amount) * 2, 4), 12)

  return (
    <motion.div
      className="absolute rounded-full"
      style={{
        width: size,
        height: size,
        backgroundColor: color,
        boxShadow: `0 0 ${size * 2}px ${color}80`,
        top: '50%',
        transform: 'translateY(-50%)',
      }}
      initial={{ x: 0, opacity: 0 }}
      animate={{
        x: [0, 120, 240, 360, 480, 600, 720, 840, 960, 1080, 1200],
        opacity: [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
      }}
      transition={{
        duration,
        delay,
        repeat: Infinity,
        ease: 'linear',
        repeatDelay: 2,
      }}
    >
      {/* Trail effect for fast particles */}
      <motion.div
        className="absolute inset-0 rounded-full"
        style={{
          backgroundColor: color,
          filter: 'blur(4px)',
        }}
        animate={{
          scale: [1, 1.5, 1],
          opacity: [0.5, 0.2, 0.5],
        }}
        transition={{
          duration: 0.5,
          repeat: Infinity,
        }}
      />
    </motion.div>
  )
}
