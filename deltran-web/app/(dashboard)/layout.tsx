'use client';

import { GoldenCompassNav } from '../components/premium/GoldenCompassNav';
import { PageTransition } from '../components/premium/PageTransition';

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen bg-gradient-to-br from-deltran-dark-midnight via-deltran-dark-obsidian to-deltran-dark-midnight">
      {/* Premium Sidebar Navigation */}
      <GoldenCompassNav />

      {/* Main Content with Page Transitions */}
      <PageTransition>
        <main className="min-h-screen">
          {children}
        </main>
      </PageTransition>
    </div>
  );
}
