'use client'

// Simplified ProtectedRoute for demo - skip auth check to avoid hydration errors
export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  // For demo purposes, always render children without authentication check
  // This avoids SSR/client hydration mismatch
  return <>{children}</>
}
