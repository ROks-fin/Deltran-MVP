'use client'

import { useState, useEffect } from 'react'
import { Card, CardHeader, CardTitle, CardContent } from '@/app/components/ui/card'
import { Button } from '@/app/components/ui/button'
import { Badge } from '@/app/components/ui/badge'

interface Report {
  report_id: string
  report_type: string
  format: string
  status: string
  generated_at: string
  period_start: string
  period_end: string
  file_size: number
  download_url: string
  generated_by: string
}

interface ReportStats {
  total_reports: number
  by_type: Record<string, number>
  by_status: Record<string, number>
  total_size_bytes: number
  oldest_report?: string
  latest_report?: string
}

export default function ReportsPage() {
  const [reports, setReports] = useState<Report[]>([])
  const [stats, setStats] = useState<ReportStats | null>(null)
  const [loading, setLoading] = useState(false)
  const [generating, setGenerating] = useState(false)
  const [selectedType, setSelectedType] = useState<string>('AML_ANNUAL')
  const [selectedFormat, setSelectedFormat] = useState<string>('excel')

  const reportTypes = [
    { value: 'AML_ANNUAL', label: 'AML Annual Return (UAE FIU)' },
    { value: 'PRU_MONTHLY', label: 'Prudential Monthly (FSRA)' },
    { value: 'SAFEGUARDING', label: 'Client Money Safeguarding' },
    { value: 'PAYMENT_STATS', label: 'Payment Statistics (Quarterly)' },
    { value: 'CTR', label: 'Currency Transaction Report' },
    { value: 'TECH_RISK', label: 'Technology Risk Report' },
    { value: 'MODEL_VALIDATION', label: 'Risk Model Validation' },
    { value: 'AUDIT_TRAIL', label: 'Audit Trail Export' },
    { value: 'TRANSACTION_LOG', label: 'Transaction Log' },
  ]

  const formats = [
    { value: 'excel', label: 'Excel (.xlsx)', icon: '📊' },
    { value: 'pdf', label: 'PDF (.pdf)', icon: '📄' },
    { value: 'csv', label: 'CSV (.csv)', icon: '📋' },
    { value: 'json', label: 'JSON (.json)', icon: '🔢' },
    { value: 'xml', label: 'XML (.xml)', icon: '📝' },
  ]

  useEffect(() => {
    loadReports()
    loadStats()
  }, [])

  const loadReports = async () => {
    try {
      setLoading(true)
      const response = await fetch('http://localhost:8080/api/v1/compliance/reports')
      if (response.ok) {
        const data = await response.json()
        setReports(data.reports || [])
      }
    } catch (error) {
      console.error('Failed to load reports:', error)
    } finally {
      setLoading(false)
    }
  }

  const loadStats = async () => {
    try {
      const response = await fetch('http://localhost:8080/api/v1/compliance/reports/stats')
      if (response.ok) {
        const data = await response.json()
        setStats(data)
      }
    } catch (error) {
      console.error('Failed to load stats:', error)
    }
  }

  const generateReport = async () => {
    try {
      setGenerating(true)

      // Calculate date range (last 30 days)
      const endDate = new Date()
      const startDate = new Date()
      startDate.setDate(startDate.getDate() - 30)

      const request = {
        report_type: selectedType,
        format: selectedFormat,
        start_date: startDate.toISOString(),
        end_date: endDate.toISOString(),
        generated_by: 'web_user',
      }

      const response = await fetch('http://localhost:8080/api/v1/compliance/reports/generate', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
      })

      if (response.ok) {
        const newReport = await response.json()
        setReports([newReport, ...reports])
        await loadStats()
        alert('Report generated successfully!')
      } else {
        alert('Failed to generate report')
      }
    } catch (error) {
      console.error('Failed to generate report:', error)
      alert('Error generating report')
    } finally {
      setGenerating(false)
    }
  }

  const downloadReport = async (reportId: string, filename: string) => {
    try {
      const response = await fetch(`http://localhost:8080/api/v1/compliance/reports/${reportId}/download`)
      if (response.ok) {
        const blob = await response.blob()
        const url = window.URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = filename
        document.body.appendChild(a)
        a.click()
        window.URL.revokeObjectURL(url)
        document.body.removeChild(a)
      } else {
        alert('Failed to download report')
      }
    } catch (error) {
      console.error('Failed to download report:', error)
      alert('Error downloading report')
    }
  }

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'READY':
        return 'bg-green-500'
      case 'GENERATING':
        return 'bg-yellow-500'
      case 'FAILED':
        return 'bg-red-500'
      default:
        return 'bg-gray-500'
    }
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold text-white">Regulatory Reports</h1>
          <p className="text-gray-400 mt-1">Big 4 Compliance Level Reporting</p>
        </div>
      </div>

      {/* Statistics Cards */}
      {stats && (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <Card className="bg-slate-800/50 border-slate-700">
            <CardContent className="p-6">
              <div className="text-sm text-gray-400">Total Reports</div>
              <div className="text-3xl font-bold text-white mt-2">{stats.total_reports}</div>
            </CardContent>
          </Card>

          <Card className="bg-slate-800/50 border-slate-700">
            <CardContent className="p-6">
              <div className="text-sm text-gray-400">Total Size</div>
              <div className="text-3xl font-bold text-white mt-2">
                {formatFileSize(stats.total_size_bytes)}
              </div>
            </CardContent>
          </Card>

          <Card className="bg-slate-800/50 border-slate-700">
            <CardContent className="p-6">
              <div className="text-sm text-gray-400">Ready Reports</div>
              <div className="text-3xl font-bold text-green-400 mt-2">
                {stats.by_status.READY || 0}
              </div>
            </CardContent>
          </Card>

          <Card className="bg-slate-800/50 border-slate-700">
            <CardContent className="p-6">
              <div className="text-sm text-gray-400">Latest Report</div>
              <div className="text-sm font-medium text-white mt-2">
                {stats.latest_report ? formatDate(stats.latest_report) : 'N/A'}
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Generate Report Section */}
      <Card className="bg-slate-800/50 border-slate-700">
        <CardHeader>
          <CardTitle className="text-xl text-white">Generate New Report</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-2">
                Report Type
              </label>
              <select
                value={selectedType}
                onChange={(e) => setSelectedType(e.target.value)}
                className="w-full bg-slate-700 border-slate-600 text-white rounded-lg px-4 py-2"
              >
                {reportTypes.map((type) => (
                  <option key={type.value} value={type.value}>
                    {type.label}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-400 mb-2">
                Format
              </label>
              <select
                value={selectedFormat}
                onChange={(e) => setSelectedFormat(e.target.value)}
                className="w-full bg-slate-700 border-slate-600 text-white rounded-lg px-4 py-2"
              >
                {formats.map((format) => (
                  <option key={format.value} value={format.value}>
                    {format.icon} {format.label}
                  </option>
                ))}
              </select>
            </div>

            <div className="flex items-end">
              <Button
                onClick={generateReport}
                disabled={generating}
                className="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-lg"
              >
                {generating ? 'Generating...' : '🎯 Generate Report'}
              </Button>
            </div>
          </div>

          <p className="text-sm text-gray-400 mt-4">
            📅 Report will cover the last 30 days of transaction data
          </p>
        </CardContent>
      </Card>

      {/* Reports List */}
      <Card className="bg-slate-800/50 border-slate-700">
        <CardHeader>
          <CardTitle className="text-xl text-white">Available Reports</CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="text-center py-8 text-gray-400">Loading reports...</div>
          ) : reports.length === 0 ? (
            <div className="text-center py-8 text-gray-400">
              No reports available. Generate your first report above.
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-slate-700">
                    <th className="text-left py-3 px-4 text-gray-400 font-medium">Type</th>
                    <th className="text-left py-3 px-4 text-gray-400 font-medium">Period</th>
                    <th className="text-left py-3 px-4 text-gray-400 font-medium">Format</th>
                    <th className="text-left py-3 px-4 text-gray-400 font-medium">Generated</th>
                    <th className="text-left py-3 px-4 text-gray-400 font-medium">Size</th>
                    <th className="text-left py-3 px-4 text-gray-400 font-medium">Status</th>
                    <th className="text-right py-3 px-4 text-gray-400 font-medium">Action</th>
                  </tr>
                </thead>
                <tbody>
                  {reports.map((report) => (
                    <tr key={report.report_id} className="border-b border-slate-700/50 hover:bg-slate-700/30">
                      <td className="py-3 px-4 text-white font-medium">{report.report_type}</td>
                      <td className="py-3 px-4 text-gray-300 text-sm">
                        {new Date(report.period_start).toLocaleDateString()} -{' '}
                        {new Date(report.period_end).toLocaleDateString()}
                      </td>
                      <td className="py-3 px-4">
                        <Badge className="bg-slate-700 text-white">{report.format.toUpperCase()}</Badge>
                      </td>
                      <td className="py-3 px-4 text-gray-300 text-sm">
                        {formatDate(report.generated_at)}
                      </td>
                      <td className="py-3 px-4 text-gray-300 text-sm">
                        {formatFileSize(report.file_size)}
                      </td>
                      <td className="py-3 px-4">
                        <Badge className={getStatusColor(report.status)}>
                          {report.status}
                        </Badge>
                      </td>
                      <td className="py-3 px-4 text-right">
                        <Button
                          onClick={() => downloadReport(report.report_id, `${report.report_type}.${report.format}`)}
                          className="bg-green-600 hover:bg-green-700 text-white text-sm px-3 py-1 rounded"
                        >
                          ⬇️ Download
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
