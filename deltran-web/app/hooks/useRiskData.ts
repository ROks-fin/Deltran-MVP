'use client'

import { useMemo } from 'react'
import { Transaction } from '../types/transaction'

export interface RiskCell {
  hour: number // 0-23
  day: number // 0-6 (0 = Monday)
  score: number // 0-100
  transactionCount: number
  totalVolume: number
}

export interface RiskHeatmapData {
  cells: RiskCell[]
  maxScore: number
  maxVolume: number
  criticalCells: RiskCell[]
}

// Generate mock risk data for heatmap (7 days x 24 hours)
const generateMockRiskData = (): RiskCell[] => {
  const cells: RiskCell[] = []
  const now = new Date()

  for (let day = 0; day < 7; day++) {
    for (let hour = 0; hour < 24; hour++) {
      // Simulate higher risk during business hours (9-17) and peak times
      const isBusinessHours = hour >= 9 && hour <= 17
      const isPeakTime = hour === 12 || hour === 15

      const baseScore = Math.random() * 30
      const businessBonus = isBusinessHours ? Math.random() * 30 : 0
      const peakBonus = isPeakTime ? Math.random() * 40 : 0

      const score = Math.min(100, baseScore + businessBonus + peakBonus)
      const transactionCount = Math.floor((score / 100) * 50)
      const totalVolume = transactionCount * (Math.random() * 100000 + 10000)

      cells.push({
        hour,
        day,
        score: Math.floor(score),
        transactionCount,
        totalVolume,
      })
    }
  }

  return cells
}

export function useRiskData(transactions: Transaction[] = []): RiskHeatmapData {
  return useMemo(() => {
    // For now, use mock data. Later, aggregate real transactions by time
    const cells = generateMockRiskData()

    const maxScore = Math.max(...cells.map(c => c.score))
    const maxVolume = Math.max(...cells.map(c => c.totalVolume))
    const criticalCells = cells.filter(c => c.score > 75)

    return {
      cells,
      maxScore,
      maxVolume,
      criticalCells,
    }
  }, [transactions])
}
