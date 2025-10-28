'use client';

import { Activity } from 'lucide-react';

export default function TransactionsPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 p-8">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center gap-3 mb-8">
          <Activity className="w-8 h-8 text-cyan-600" />
          <h1 className="text-3xl font-bold">Transaction History</h1>
        </div>
        <div className="bg-white rounded-xl shadow-lg p-6">
          <p className="text-gray-600">Complete transaction ledger and history</p>
        </div>
      </div>
    </div>
  );
}
