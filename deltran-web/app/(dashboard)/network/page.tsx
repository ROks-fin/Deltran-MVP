'use client';

import { Globe } from 'lucide-react';

export default function NetworkPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 p-8">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center gap-3 mb-8">
          <Globe className="w-8 h-8 text-violet-600" />
          <h1 className="text-3xl font-bold">Network Topology</h1>
        </div>
        <div className="bg-white rounded-xl shadow-lg p-6">
          <p className="text-gray-600">Inter-bank network visualization and status</p>
        </div>
      </div>
    </div>
  );
}
