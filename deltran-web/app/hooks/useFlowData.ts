'use client'

import { useMemo } from 'react'
import { Transaction, PaymentStatus } from '../types/transaction'

export interface FlowNode {
  status: PaymentStatus
  count: number
  volume: number
  avgProcessingTime: number
  transactions: Transaction[]
}

export interface FlowData {
  nodes: FlowNode[]
  flowRate: number
  bottleneck?: PaymentStatus
  totalVolume: number
}

const ALL_STATUSES: PaymentStatus[] = [
  'Initiated',
  'Validated',
  'Screened',
  'Approved',
  'Queued',
  'Settling',
  'Settled',
  'Completed',
  'Rejected',
  'Failed',
  'Cancelled',
]

export function useFlowData(transactions: Transaction[] = []): FlowData {
  return useMemo(() => {
    // Group transactions by status
    const statusMap = new Map<PaymentStatus, Transaction[]>()

    ALL_STATUSES.forEach((status) => {
      statusMap.set(status, [])
    })

    transactions.forEach((tx) => {
      const existing = statusMap.get(tx.status) || []
      statusMap.set(tx.status, [...existing, tx])
    })

    // Calculate metrics for each node
    const nodes: FlowNode[] = ALL_STATUSES.map((status) => {
      const statusTransactions = statusMap.get(status) || []
      const volume = statusTransactions.reduce(
        (sum, tx) => sum + parseFloat(tx.amount),
        0
      )
      const avgTime =
        statusTransactions.reduce(
          (sum, tx) => sum + (tx.processing_time || 0),
          0
        ) / (statusTransactions.length || 1)

      return {
        status,
        count: statusTransactions.length,
        volume,
        avgProcessingTime: avgTime,
        transactions: statusTransactions.slice(0, 5),
      }
    })

    // Find bottleneck (status with most transactions)
    const bottleneckNode = nodes.reduce((max, node) =>
      node.count > max.count ? node : max
    )
    const bottleneck =
      bottleneckNode.count > 3 ? bottleneckNode.status : undefined

    // Calculate flow rate (transactions per second - mock for now)
    const flowRate = transactions.length / 60 // assuming 1 minute window

    // Total volume
    const totalVolume = nodes.reduce((sum, node) => sum + node.volume, 0)

    return {
      nodes,
      flowRate,
      bottleneck,
      totalVolume,
    }
  }, [transactions])
}
