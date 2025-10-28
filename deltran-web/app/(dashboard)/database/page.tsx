'use client';

import { Database } from 'lucide-react';

export default function DatabasePage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 p-8">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center gap-3 mb-8">
          <Database className="w-8 h-8 text-slate-600" />
          <h1 className="text-3xl font-bold">Database Explorer</h1>
        </div>
        <div className="bg-white rounded-xl shadow-lg p-6">
          <p className="text-gray-600">Direct database access and query interface</p>
        </div>
      </div>
    </div>
  );
}
