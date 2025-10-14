'use client'

import { motion } from 'framer-motion'
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getPaginationRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
} from '@tanstack/react-table'
import { useState } from 'react'
import { Transaction } from '@/app/types/transaction'
import { StatusBadge } from './StatusBadge'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { PaymentDetailsModal } from '../modals/PaymentDetailsModal'

interface TransactionsTableProps {
  transactions?: Transaction[]
  isLoading?: boolean
}

const columnHelper = createColumnHelper<Transaction>()

const columns = [
  columnHelper.accessor('payment_id', {
    header: 'Payment ID',
    cell: (info) => (
      <span className="font-mono text-xs text-zinc-300">
        {info.getValue()}
      </span>
    ),
  }),
  columnHelper.accessor('amount', {
    header: 'Amount',
    cell: (info) => {
      const amount = parseFloat(info.getValue())
      const currency = info.row.original.currency
      return (
        <span className="font-semibold text-white">
          {new Intl.NumberFormat('en-US', {
            style: 'currency',
            currency,
            minimumFractionDigits: 2,
          }).format(amount)}
        </span>
      )
    },
  }),
  columnHelper.accessor('status', {
    header: 'Status',
    cell: (info) => <StatusBadge status={info.getValue()} />,
  }),
  columnHelper.accessor('sender_account', {
    header: 'From',
    cell: (info) => (
      <span className="font-mono text-xs text-zinc-400">
        {info.getValue()}
      </span>
    ),
  }),
  columnHelper.accessor('recipient_account', {
    header: 'To',
    cell: (info) => (
      <span className="font-mono text-xs text-zinc-400">
        {info.getValue()}
      </span>
    ),
  }),
  columnHelper.accessor('created_at', {
    header: 'Time',
    cell: (info) => {
      const date = new Date(info.getValue())
      const now = new Date()
      const diff = now.getTime() - date.getTime()
      const minutes = Math.floor(diff / 60000)

      if (minutes < 1) return <span className="text-xs text-zinc-500">Just now</span>
      if (minutes < 60) return <span className="text-xs text-zinc-500">{minutes}m ago</span>
      const hours = Math.floor(minutes / 60)
      return <span className="text-xs text-zinc-500">{hours}h ago</span>
    },
  }),
  columnHelper.accessor('risk_score', {
    header: 'Risk',
    cell: (info) => {
      const score = info.getValue() ?? 0
      const color = score > 70 ? '#f87171' : score > 30 ? '#f59e0b' : '#4ade80'
      return (
        <div className="flex items-center gap-2">
          <div className="w-12 h-1.5 bg-zinc-800 rounded-full overflow-hidden">
            <div
              className="h-full transition-all duration-500"
              style={{ width: `${score}%`, backgroundColor: color }}
            />
          </div>
          <span className="text-xs text-zinc-500">{score}</span>
        </div>
      )
    },
  }),
]

export function TransactionsTable({ transactions = [], isLoading }: TransactionsTableProps) {
  const [sorting, setSorting] = useState<SortingState>([])
  const [selectedPaymentId, setSelectedPaymentId] = useState<string | null>(null)

  const table = useReactTable({
    data: transactions,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    initialState: {
      pagination: {
        pageSize: 10,
      },
    },
  })

  if (isLoading) {
    return (
      <div className="rounded-xl p-6 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-xl font-semibold text-white">Recent Transactions</h3>
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-zinc-600 animate-pulse" />
            <span className="text-sm text-zinc-500">Loading...</span>
          </div>
        </div>
        {/* Skeleton */}
        <div className="space-y-3">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-12 bg-zinc-800/50 rounded animate-pulse" />
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="rounded-xl p-6 bg-gradient-to-br from-zinc-900 to-black border border-zinc-800">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-xl font-semibold text-white">Recent Transactions</h3>
        <div className="flex items-center gap-2">
          <motion.div
            animate={{ scale: [1, 1.2, 1], opacity: [0.5, 1, 0.5] }}
            transition={{ duration: 2, repeat: Infinity }}
            className="w-2 h-2 rounded-full bg-green-400"
          />
          <span className="text-sm text-zinc-400">Live</span>
        </div>
      </div>

      {/* Table */}
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="border-b border-zinc-800">
              {table.getHeaderGroups().map((headerGroup) =>
                headerGroup.headers.map((header) => (
                  <th
                    key={header.id}
                    className="text-left py-3 px-4 text-xs font-semibold text-zinc-500 uppercase tracking-wider"
                  >
                    {flexRender(header.column.columnDef.header, header.getContext())}
                  </th>
                ))
              )}
            </tr>
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row, index) => (
              <motion.tr
                key={row.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.05, duration: 0.3 }}
                onClick={() => setSelectedPaymentId(row.original.payment_id)}
                className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors group cursor-pointer"
              >
                {/* Gold highlight on hover */}
                <td className="relative">
                  <div className="absolute left-0 top-0 bottom-0 w-0.5 bg-deltran-gold opacity-0 group-hover:opacity-100 transition-opacity" />
                </td>
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="py-4 px-4">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </motion.tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between mt-6 pt-4 border-t border-zinc-800">
        <div className="text-sm text-zinc-500">
          Showing {table.getState().pagination.pageIndex * table.getState().pagination.pageSize + 1} to{' '}
          {Math.min(
            (table.getState().pagination.pageIndex + 1) * table.getState().pagination.pageSize,
            transactions.length
          )}{' '}
          of {transactions.length} transactions
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
            className="p-2 rounded-lg bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>
          <button
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
            className="p-2 rounded-lg bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            <ChevronRight className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Payment Details Modal */}
      {selectedPaymentId && (
        <PaymentDetailsModal
          paymentId={selectedPaymentId}
          isOpen={!!selectedPaymentId}
          onClose={() => setSelectedPaymentId(null)}
        />
      )}
    </div>
  )
}
