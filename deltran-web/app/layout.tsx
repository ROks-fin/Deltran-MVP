import type { Metadata } from 'next'
import './globals.css'
import { QueryProvider } from './providers/QueryProvider'
import { ToastProvider } from './providers/ToastProvider'
import { PremiumToastContainer } from './components/premium/PremiumToast'
import { CommandPalette } from './components/premium/CommandPalette'

export const metadata: Metadata = {
  title: 'DelTran Premium - Digital Luxury Finance',
  description: 'Ultra-Premium Payment Gateway with Swiss Bank-Level Interface',
  icons: {
    icon: '/favicon.ico',
  },
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" className="dark">
      <body className="bg-deltran-dark-midnight">
        <QueryProvider>
          {children}
          <ToastProvider />

          {/* Premium Global Components */}
          <PremiumToastContainer />
          <CommandPalette />
        </QueryProvider>
      </body>
    </html>
  )
}
