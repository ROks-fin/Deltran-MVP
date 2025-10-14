'use client';

import React, { useState } from 'react';
import { Download, FileText, Shield, Database, Calendar } from 'lucide-react';
import toast from 'react-hot-toast';

interface ExportRequest {
  report_type: 'audit_trail' | 'transaction_ledger' | 'reconciliation';
  start_date: string;
  end_date: string;
  compliance_type?: string;
  entity_type?: string;
  format: 'csv' | 'xlsx' | 'json';
  include_metadata: boolean;
}

export default function AuditReportsPage() {
  const [loading, setLoading] = useState(false);
  const [reportType, setReportType] = useState<'audit_trail' | 'transaction_ledger' | 'reconciliation'>('audit_trail');
  const [startDate, setStartDate] = useState(new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]);
  const [endDate, setEndDate] = useState(new Date().toISOString().split('T')[0]);
  const [complianceType, setComplianceType] = useState('SOX');
  const [format, setFormat] = useState<'csv' | 'xlsx' | 'json'>('xlsx');
  const [includeMetadata, setIncludeMetadata] = useState(true);

  const exportReport = async () => {
    setLoading(true);

    try {
      const token = localStorage.getItem('access_token');

      const request: ExportRequest = {
        report_type: reportType,
        start_date: `${startDate}T00:00:00Z`,
        end_date: `${endDate}T23:59:59Z`,
        format,
        include_metadata: includeMetadata,
      };

      if (reportType === 'audit_trail') {
        request.compliance_type = complianceType;
      }

      const endpoint = reportType === 'audit_trail'
        ? '/api/v1/audit/export/trail'
        : reportType === 'transaction_ledger'
        ? '/api/v1/audit/export/ledger'
        : '/api/v1/audit/export/reconciliation';

      const response = await fetch(`http://localhost:8080${endpoint}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result = await response.json();

      toast.success(
        <div>
          <p className="font-semibold">Report Generated Successfully!</p>
          <p className="text-sm text-gray-600 mt-1">
            {result.record_count} records exported
          </p>
          <p className="text-xs text-gray-500 mt-1">
            File: {result.file_path}
          </p>
          <p className="text-xs text-gray-500">
            Reference: {result.compliance_ref}
          </p>
        </div>,
        { duration: 8000 }
      );

      // Log export event to console
      console.log('Export completed:', result);

    } catch (error) {
      console.error('Export failed:', error);
      toast.error('Failed to generate report. Please check console for details.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 p-8">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <div className="flex items-center gap-3 mb-2">
            <Shield className="w-8 h-8 text-blue-600" />
            <h1 className="text-3xl font-bold text-gray-900">Audit Reports & Compliance</h1>
          </div>
          <p className="text-gray-600">
            Big Four compliant audit trail exports - SOX, IFRS 9, Basel III, PCI DSS Level 1
          </p>
        </div>

        {/* Main Export Card */}
        <div className="bg-white rounded-xl shadow-lg border border-gray-200 overflow-hidden">
          <div className="bg-gradient-to-r from-blue-600 to-indigo-600 text-white px-6 py-4">
            <h2 className="text-xl font-semibold flex items-center gap-2">
              <FileText className="w-5 h-5" />
              Generate Compliance Report
            </h2>
          </div>

          <div className="p-6 space-y-6">
            {/* Report Type Selection */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-3">
                Report Type
              </label>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <button
                  onClick={() => setReportType('audit_trail')}
                  className={`p-4 rounded-lg border-2 transition-all ${
                    reportType === 'audit_trail'
                      ? 'border-blue-600 bg-blue-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <Shield className="w-6 h-6 mb-2 text-blue-600" />
                  <div className="text-left">
                    <p className="font-semibold text-gray-900">Audit Trail</p>
                    <p className="text-xs text-gray-600 mt-1">Complete user activity log</p>
                  </div>
                </button>

                <button
                  onClick={() => setReportType('transaction_ledger')}
                  className={`p-4 rounded-lg border-2 transition-all ${
                    reportType === 'transaction_ledger'
                      ? 'border-blue-600 bg-blue-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <Database className="w-6 h-6 mb-2 text-green-600" />
                  <div className="text-left">
                    <p className="font-semibold text-gray-900">Transaction Ledger</p>
                    <p className="text-xs text-gray-600 mt-1">Immutable financial records</p>
                  </div>
                </button>

                <button
                  onClick={() => setReportType('reconciliation')}
                  className={`p-4 rounded-lg border-2 transition-all ${
                    reportType === 'reconciliation'
                      ? 'border-blue-600 bg-blue-50'
                      : 'border-gray-200 hover:border-gray-300'
                  }`}
                >
                  <FileText className="w-6 h-6 mb-2 text-purple-600" />
                  <div className="text-left">
                    <p className="font-semibold text-gray-900">Reconciliation</p>
                    <p className="text-xs text-gray-600 mt-1">Daily settlement reports</p>
                  </div>
                </button>
              </div>
            </div>

            {/* Date Range */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  <Calendar className="w-4 h-4 inline mr-1" />
                  Start Date
                </label>
                <input
                  type="date"
                  value={startDate}
                  onChange={(e) => setStartDate(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  <Calendar className="w-4 h-4 inline mr-1" />
                  End Date
                </label>
                <input
                  type="date"
                  value={endDate}
                  onChange={(e) => setEndDate(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
              </div>
            </div>

            {/* Compliance Type (only for audit trail) */}
            {reportType === 'audit_trail' && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Compliance Standard
                </label>
                <select
                  value={complianceType}
                  onChange={(e) => setComplianceType(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                >
                  <option value="SOX">SOX (Sarbanes-Oxley)</option>
                  <option value="IFRS-9">IFRS 9 (Financial Instruments)</option>
                  <option value="Basel-III">Basel III (Banking Regulation)</option>
                  <option value="PCI-DSS">PCI DSS (Payment Card Industry)</option>
                  <option value="GDPR">GDPR (Data Protection)</option>
                </select>
              </div>
            )}

            {/* Export Format */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Export Format
              </label>
              <div className="flex gap-3">
                <button
                  onClick={() => setFormat('xlsx')}
                  className={`px-6 py-2 rounded-lg font-medium transition-all ${
                    format === 'xlsx'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  Excel (.xlsx)
                </button>
                <button
                  onClick={() => setFormat('csv')}
                  className={`px-6 py-2 rounded-lg font-medium transition-all ${
                    format === 'csv'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  CSV
                </button>
                <button
                  onClick={() => setFormat('json')}
                  className={`px-6 py-2 rounded-lg font-medium transition-all ${
                    format === 'json'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  JSON
                </button>
              </div>
            </div>

            {/* Include Metadata */}
            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                id="metadata"
                checked={includeMetadata}
                onChange={(e) => setIncludeMetadata(e.target.checked)}
                className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
              />
              <label htmlFor="metadata" className="text-sm font-medium text-gray-700">
                Include detailed metadata and change history
              </label>
            </div>

            {/* Export Button */}
            <button
              onClick={exportReport}
              disabled={loading}
              className="w-full bg-gradient-to-r from-blue-600 to-indigo-600 text-white py-3 px-6 rounded-lg font-semibold hover:from-blue-700 hover:to-indigo-700 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {loading ? (
                <>
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                  Generating Report...
                </>
              ) : (
                <>
                  <Download className="w-5 h-5" />
                  Generate & Export Report
                </>
              )}
            </button>
          </div>
        </div>

        {/* Info Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mt-8">
          <div className="bg-white rounded-lg shadow-md p-6 border-l-4 border-blue-600">
            <h3 className="font-semibold text-gray-900 mb-2">Audit Trail</h3>
            <p className="text-sm text-gray-600">
              Complete user activity log with old/new values, IP tracking, and MFA verification status
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-md p-6 border-l-4 border-green-600">
            <h3 className="font-semibold text-gray-900 mb-2">Transaction Ledger</h3>
            <p className="text-sm text-gray-600">
              Immutable financial records with SHA-256 hashes and Ed25519 digital signatures
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-md p-6 border-l-4 border-purple-600">
            <h3 className="font-semibold text-gray-900 mb-2">Reconciliation</h3>
            <p className="text-sm text-gray-600">
              Daily/Monthly settlement reports with variance analysis and external audit references
            </p>
          </div>
        </div>

        {/* Compliance Badges */}
        <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg p-6 mt-8 border border-blue-200">
          <h3 className="font-semibold text-gray-900 mb-4">Compliance Standards</h3>
          <div className="flex flex-wrap gap-3">
            <span className="px-4 py-2 bg-white rounded-full text-sm font-medium text-blue-700 border border-blue-300">
              SOX Compliant
            </span>
            <span className="px-4 py-2 bg-white rounded-full text-sm font-medium text-green-700 border border-green-300">
              IFRS 9 Ready
            </span>
            <span className="px-4 py-2 bg-white rounded-full text-sm font-medium text-purple-700 border border-purple-300">
              Basel III
            </span>
            <span className="px-4 py-2 bg-white rounded-full text-sm font-medium text-red-700 border border-red-300">
              PCI DSS Level 1
            </span>
            <span className="px-4 py-2 bg-white rounded-full text-sm font-medium text-indigo-700 border border-indigo-300">
              7-Year Retention
            </span>
            <span className="px-4 py-2 bg-white rounded-full text-sm font-medium text-orange-700 border border-orange-300">
              Cryptographic Proof
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
