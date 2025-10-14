import type { Metadata } from 'next'
import './globals.css'
import { QueryProvider } from './providers/QueryProvider'
import { ToastProvider } from './providers/ToastProvider'

export const metadata: Metadata = {
  title: 'DelTran Premium',
  description: 'Premium Payment Gateway',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>
        <QueryProvider>
          {children}
          <ToastProvider />
        </QueryProvider>
      </body>
    </html>
  )
}
