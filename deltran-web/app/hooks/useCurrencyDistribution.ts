'use client'

import { useMemo } from 'react'
import { Transaction, Currency } from '../types/transaction'

export interface CurrencyData {
  currency: Currency
  value: number
  percentage: number
  count: number
  trend: 'up' | 'down' | 'stable'
}

export interface CurrencyDistributionData {
  currencies: CurrencyData[]
  totalVolume: number
  dominantCurrency: Currency
}

export function useCurrencyDistribution(transactions: Transaction[] = []): CurrencyDistributionData {
  return useMemo(() => {
    if (transactions.length === 0) {
      // Mock data when no transactions
      const mockData: CurrencyData[] = [
        { currency: 'USD', value: 8500000, percentage: 56, count: 89, trend: 'up' },
        { currency: 'EUR', value: 3200000, percentage: 21, count: 34, trend: 'up' },
        { currency: 'GBP', value: 1800000, percentage: 12, count: 18, trend: 'stable' },
        { currency: 'AED', value: 950000, percentage: 6, count: 12, trend: 'down' },
        { currency: 'INR', value: 450000, percentage: 3, count: 8, trend: 'up' },
        { currency: 'PKR', value: 200000, percentage: 1, count: 5, trend: 'stable' },
        { currency: 'ILS', value: 100000, percentage: 1, count: 3, trend: 'stable' },
      ]

      return {
        currencies: mockData,
        totalVolume: mockData.reduce((sum, c) => sum + c.value, 0),
        dominantCurrency: 'USD' as Currency,
      }
    }

    // Aggregate real transactions by currency
    const currencyMap = new Map<Currency, { value: number; count: number }>()

    transactions.forEach((tx) => {
      const existing = currencyMap.get(tx.currency) || { value: 0, count: 0 }
      currencyMap.set(tx.currency, {
        value: existing.value + parseFloat(tx.amount),
        count: existing.count + 1,
      })
    })

    const totalVolume = Array.from(currencyMap.values()).reduce((sum, c) => sum + c.value, 0)

    const currencies: CurrencyData[] = Array.from(currencyMap.entries())
      .map(([currency, data]) => ({
        currency,
        value: data.value,
        percentage: (data.value / totalVolume) * 100,
        count: data.count,
        trend: Math.random() > 0.5 ? ('up' as const) : ('stable' as const), // Mock trend for now
      }))
      .sort((a, b) => b.value - a.value)

    const dominantCurrency = currencies[0]?.currency || ('USD' as Currency)

    return {
      currencies,
      totalVolume,
      dominantCurrency,
    }
  }, [transactions])
}
