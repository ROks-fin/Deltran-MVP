'use client';

import { Settings } from 'lucide-react';

export default function SettingsPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 p-8">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center gap-3 mb-8">
          <Settings className="w-8 h-8 text-gray-600" />
          <h1 className="text-3xl font-bold">System Settings</h1>
        </div>
        <div className="bg-white rounded-xl shadow-lg p-6">
          <p className="text-gray-600">Configure system parameters and preferences</p>
        </div>
      </div>
    </div>
  );
}
