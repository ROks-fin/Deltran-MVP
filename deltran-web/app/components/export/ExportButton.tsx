'use client'

import { motion } from 'framer-motion'
import { Download, Loader2 } from 'lucide-react'
import { useState } from 'react'
import toast from 'react-hot-toast'
import { FilterParams } from '../filters/AdvancedFilters'

interface ExportButtonProps {
  filters: FilterParams
}

export function ExportButton({ filters }: ExportButtonProps) {
  const [isExporting, setIsExporting] = useState(false)

  const handleExport = async () => {
    setIsExporting(true)

    try {
      // Build query params
      const params = new URLSearchParams()

      if (filters.status) params.append('status', filters.status.toLowerCase())
      if (filters.currency) params.append('currency', filters.currency)
      if (filters.senderBIC) params.append('sender_bic', filters.senderBIC)
      if (filters.receiverBIC) params.append('receiver_bic', filters.receiverBIC)
      if (filters.dateFrom) params.append('date_from', filters.dateFrom)
      if (filters.dateTo) params.append('date_to', filters.dateTo)

      const queryString = params.toString()
      const url = `/api/v1/export/payments${queryString ? `?${queryString}` : ''}`

      // Fetch CSV
      const response = await fetch(url)

      if (!response.ok) {
        throw new Error('Export failed')
      }

      // Get filename from Content-Disposition header or generate one
      const contentDisposition = response.headers.get('Content-Disposition')
      let filename = 'payments_export.csv'
      if (contentDisposition) {
        const filenameMatch = contentDisposition.match(/filename="?(.+)"?/)
        if (filenameMatch) {
          filename = filenameMatch[1]
        }
      }

      // Create blob and download
      const blob = await response.blob()
      const downloadUrl = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = downloadUrl
      link.download = filename
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      window.URL.revokeObjectURL(downloadUrl)

      // Success toast
      toast.success('CSV exported successfully!')
    } catch (error) {
      console.error('Export failed:', error)
      toast.error('Failed to export payments. Please try again.')
    } finally {
      setIsExporting(false)
    }
  }

  return (
    <motion.button
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={handleExport}
      disabled={isExporting}
      className="px-4 py-2.5 rounded-lg bg-gradient-to-r from-deltran-gold to-deltran-gold-light hover:from-deltran-gold-light hover:to-deltran-gold text-black font-semibold text-sm flex items-center gap-2 transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-lg hover:shadow-deltran-gold/20"
    >
      {isExporting ? (
        <>
          <Loader2 className="w-4 h-4 animate-spin" />
          Exporting...
        </>
      ) : (
        <>
          <Download className="w-4 h-4" />
          Export CSV
        </>
      )}
    </motion.button>
  )
}
