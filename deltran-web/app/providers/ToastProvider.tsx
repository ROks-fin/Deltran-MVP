'use client'

import { Toaster } from 'react-hot-toast'

export function ToastProvider() {
  return (
    <Toaster
      position="top-right"
      toastOptions={{
        // Default options
        duration: 3000,
        style: {
          background: '#18181b',
          color: '#fff',
          border: '1px solid #27272a',
          borderRadius: '12px',
          padding: '16px',
          fontSize: '14px',
          boxShadow: '0 10px 40px rgba(0, 0, 0, 0.5)',
        },
        // Success
        success: {
          duration: 3000,
          iconTheme: {
            primary: '#10b981',
            secondary: '#18181b',
          },
          style: {
            border: '1px solid rgba(16, 185, 129, 0.3)',
            background: 'linear-gradient(to bottom right, #18181b, #0c0c0c)',
          },
        },
        // Error
        error: {
          duration: 4000,
          iconTheme: {
            primary: '#ef4444',
            secondary: '#18181b',
          },
          style: {
            border: '1px solid rgba(239, 68, 68, 0.3)',
            background: 'linear-gradient(to bottom right, #18181b, #0c0c0c)',
          },
        },
        // Loading
        loading: {
          iconTheme: {
            primary: '#d4af37',
            secondary: '#18181b',
          },
          style: {
            border: '1px solid rgba(212, 175, 55, 0.3)',
            background: 'linear-gradient(to bottom right, #18181b, #0c0c0c)',
          },
        },
      }}
    />
  )
}
