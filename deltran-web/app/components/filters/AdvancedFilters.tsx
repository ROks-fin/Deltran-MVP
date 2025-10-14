'use client'

import { motion, AnimatePresence } from 'framer-motion'
import { Filter, X, Search, Calendar, DollarSign, Building2, RefreshCcw } from 'lucide-react'
import { useState } from 'react'
import { PaymentStatus, Currency } from '@/app/types/transaction'

export interface FilterParams {
  status?: PaymentStatus | ''
  currency?: Currency | ''
  senderBIC?: string
  receiverBIC?: string
  dateFrom?: string
  dateTo?: string
  minAmount?: string
  maxAmount?: string
  reference?: string
}

interface AdvancedFiltersProps {
  filters: FilterParams
  onChange: (filters: FilterParams) => void
  onReset: () => void
}

const statusOptions: (PaymentStatus | '')[] = [
  '',
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

const currencyOptions: (Currency | '')[] = ['', 'USD', 'EUR', 'GBP', 'AED', 'INR', 'PKR', 'ILS']

export function AdvancedFilters({ filters, onChange, onReset }: AdvancedFiltersProps) {
  const [isExpanded, setIsExpanded] = useState(false)

  const updateFilter = (key: keyof FilterParams, value: string) => {
    onChange({ ...filters, [key]: value })
  }

  const hasActiveFilters = Object.values(filters).some((v) => v !== '' && v !== undefined)

  const activeFilterCount = Object.values(filters).filter(
    (v) => v !== '' && v !== undefined
  ).length

  return (
    <div className="rounded-xl bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 overflow-hidden">
      {/* Header - Always Visible */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full p-4 flex items-center justify-between hover:bg-zinc-800/30 transition-colors"
      >
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-deltran-gold/10 border border-deltran-gold/30">
            <Filter className="w-5 h-5 text-deltran-gold" />
          </div>
          <div className="text-left">
            <h3 className="text-lg font-semibold text-white">Advanced Filters</h3>
            <p className="text-xs text-zinc-500">
              {hasActiveFilters ? `${activeFilterCount} filter${activeFilterCount > 1 ? 's' : ''} active` : 'No filters applied'}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {hasActiveFilters && (
            <motion.button
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              onClick={(e) => {
                e.stopPropagation()
                onReset()
              }}
              className="px-3 py-1.5 rounded-lg bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-400 text-xs font-medium transition-colors flex items-center gap-1"
            >
              <X className="w-3 h-3" />
              Clear
            </motion.button>
          )}

          <motion.div
            animate={{ rotate: isExpanded ? 180 : 0 }}
            transition={{ duration: 0.2 }}
            className="text-zinc-400"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
            </svg>
          </motion.div>
        </div>
      </button>

      {/* Filter Panel - Expandable */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: 'easeInOut' }}
            className="overflow-hidden border-t border-zinc-800"
          >
            <div className="p-6 space-y-6">
              {/* Row 1: Status & Currency */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <FilterSelect
                  label="Status"
                  value={filters.status || ''}
                  onChange={(value) => updateFilter('status', value)}
                  options={statusOptions.map((s) => ({ value: s, label: s || 'All Statuses' }))}
                  icon={RefreshCcw}
                />

                <FilterSelect
                  label="Currency"
                  value={filters.currency || ''}
                  onChange={(value) => updateFilter('currency', value)}
                  options={currencyOptions.map((c) => ({ value: c, label: c || 'All Currencies' }))}
                  icon={DollarSign}
                />
              </div>

              {/* Row 2: BIC Codes */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <FilterInput
                  label="Sender BIC"
                  value={filters.senderBIC || ''}
                  onChange={(value) => updateFilter('senderBIC', value)}
                  placeholder="e.g. CHASUS33XXX"
                  icon={Building2}
                />

                <FilterInput
                  label="Receiver BIC"
                  value={filters.receiverBIC || ''}
                  onChange={(value) => updateFilter('receiverBIC', value)}
                  placeholder="e.g. HSBCGB2LXXX"
                  icon={Building2}
                />
              </div>

              {/* Row 3: Date Range */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <FilterInput
                  label="Date From"
                  type="date"
                  value={filters.dateFrom || ''}
                  onChange={(value) => updateFilter('dateFrom', value)}
                  icon={Calendar}
                />

                <FilterInput
                  label="Date To"
                  type="date"
                  value={filters.dateTo || ''}
                  onChange={(value) => updateFilter('dateTo', value)}
                  icon={Calendar}
                />
              </div>

              {/* Row 4: Amount Range */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <FilterInput
                  label="Min Amount"
                  type="number"
                  value={filters.minAmount || ''}
                  onChange={(value) => updateFilter('minAmount', value)}
                  placeholder="0.00"
                  icon={DollarSign}
                />

                <FilterInput
                  label="Max Amount"
                  type="number"
                  value={filters.maxAmount || ''}
                  onChange={(value) => updateFilter('maxAmount', value)}
                  placeholder="999999.99"
                  icon={DollarSign}
                />
              </div>

              {/* Row 5: Reference Search */}
              <FilterInput
                label="Reference / Payment ID"
                value={filters.reference || ''}
                onChange={(value) => updateFilter('reference', value)}
                placeholder="Search by reference or payment ID..."
                icon={Search}
              />

              {/* Active Filters Summary */}
              {hasActiveFilters && (
                <motion.div
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="pt-4 border-t border-zinc-800"
                >
                  <p className="text-xs text-zinc-500 mb-3 uppercase tracking-wider">Active Filters</p>
                  <div className="flex flex-wrap gap-2">
                    {Object.entries(filters).map(([key, value]) => {
                      if (!value) return null
                      return (
                        <FilterTag
                          key={key}
                          label={formatFilterLabel(key)}
                          value={String(value)}
                          onRemove={() => updateFilter(key as keyof FilterParams, '')}
                        />
                      )
                    })}
                  </div>
                </motion.div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}

// Filter Input Component
function FilterInput({
  label,
  value,
  onChange,
  placeholder,
  icon: Icon,
  type = 'text',
}: {
  label: string
  value: string
  onChange: (value: string) => void
  placeholder?: string
  icon: React.ElementType
  type?: string
}) {
  return (
    <div>
      <label className="block text-xs text-zinc-400 mb-2 uppercase tracking-wider">
        {label}
      </label>
      <div className="relative">
        <div className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500">
          <Icon className="w-4 h-4" />
        </div>
        <input
          type={type}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="w-full pl-10 pr-4 py-2.5 bg-zinc-800/50 border border-zinc-700 rounded-lg text-sm text-white placeholder:text-zinc-600 focus:outline-none focus:border-deltran-gold/50 focus:bg-zinc-800 transition-all"
        />
      </div>
    </div>
  )
}

// Filter Select Component
function FilterSelect({
  label,
  value,
  onChange,
  options,
  icon: Icon,
}: {
  label: string
  value: string
  onChange: (value: string) => void
  options: Array<{ value: string; label: string }>
  icon: React.ElementType
}) {
  return (
    <div>
      <label className="block text-xs text-zinc-400 mb-2 uppercase tracking-wider">
        {label}
      </label>
      <div className="relative">
        <div className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500 pointer-events-none">
          <Icon className="w-4 h-4" />
        </div>
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="w-full pl-10 pr-10 py-2.5 bg-zinc-800/50 border border-zinc-700 rounded-lg text-sm text-white focus:outline-none focus:border-deltran-gold/50 focus:bg-zinc-800 transition-all appearance-none cursor-pointer"
        >
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <div className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 pointer-events-none">
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
    </div>
  )
}

// Filter Tag Component
function FilterTag({
  label,
  value,
  onRemove,
}: {
  label: string
  value: string
  onRemove: () => void
}) {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.9 }}
      className="inline-flex items-center gap-2 px-3 py-1.5 bg-deltran-gold/10 border border-deltran-gold/30 rounded-lg text-xs"
    >
      <span className="text-zinc-400">{label}:</span>
      <span className="text-deltran-gold font-medium">{value}</span>
      <button
        onClick={onRemove}
        className="ml-1 text-zinc-500 hover:text-red-400 transition-colors"
      >
        <X className="w-3 h-3" />
      </button>
    </motion.div>
  )
}

// Helper function to format filter labels
function formatFilterLabel(key: string): string {
  const labels: Record<string, string> = {
    status: 'Status',
    currency: 'Currency',
    senderBIC: 'Sender BIC',
    receiverBIC: 'Receiver BIC',
    dateFrom: 'From',
    dateTo: 'To',
    minAmount: 'Min Amount',
    maxAmount: 'Max Amount',
    reference: 'Reference',
  }
  return labels[key] || key
}
