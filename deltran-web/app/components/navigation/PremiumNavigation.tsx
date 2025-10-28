'use client';

import React, { useState } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import {
  LayoutDashboard,
  Wallet,
  Building2,
  Shield,
  FileText,
  TrendingUp,
  Users,
  Settings,
  Database,
  Activity,
  Globe,
  BarChart3,
  LogOut,
} from 'lucide-react';

interface NavItem {
  name: string;
  path: string;
  icon: React.ElementType;
  color: string;
  badge?: string;
}

const NAV_ITEMS: NavItem[] = [
  {
    name: 'Dashboard',
    path: '/',
    icon: LayoutDashboard,
    color: 'from-blue-500 to-cyan-500',
  },
  {
    name: 'Payments',
    path: '/payments',
    icon: Wallet,
    color: 'from-emerald-500 to-teal-500',
    badge: '142',
  },
  {
    name: 'Banks',
    path: '/banks',
    icon: Building2,
    color: 'from-purple-500 to-pink-500',
  },
  {
    name: 'Compliance',
    path: '/compliance',
    icon: Shield,
    color: 'from-orange-500 to-red-500',
    badge: '5',
  },
  {
    name: 'Audit Reports',
    path: '/audit',
    icon: FileText,
    color: 'from-indigo-500 to-blue-500',
  },
  {
    name: 'Reports',
    path: '/reports',
    icon: BarChart3,
    color: 'from-yellow-500 to-orange-500',
  },
  {
    name: 'Analytics',
    path: '/analytics',
    icon: TrendingUp,
    color: 'from-green-500 to-emerald-500',
  },
  {
    name: 'Transactions',
    path: '/transactions',
    icon: Activity,
    color: 'from-cyan-500 to-blue-500',
  },
  {
    name: 'Users',
    path: '/users',
    icon: Users,
    color: 'from-pink-500 to-rose-500',
  },
  {
    name: 'Network',
    path: '/network',
    icon: Globe,
    color: 'from-violet-500 to-purple-500',
  },
  {
    name: 'Database',
    path: '/database',
    icon: Database,
    color: 'from-slate-500 to-zinc-500',
  },
  {
    name: 'Settings',
    path: '/settings',
    icon: Settings,
    color: 'from-gray-500 to-slate-500',
  },
];

export default function PremiumNavigation() {
  const pathname = usePathname();
  const router = useRouter();
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const handleLogout = () => {
    localStorage.removeItem('access_token');
    router.push('/login');
  };

  return (
    <header className="sticky top-0 z-50 border-b border-zinc-800/50 backdrop-blur-xl">
      {/* Premium gradient overlay */}
      <div className="absolute inset-0 bg-gradient-to-r from-zinc-900/95 via-black/95 to-zinc-900/95 backdrop-blur-xl" />

      {/* Animated border gradient */}
      <div className="absolute bottom-0 left-0 right-0 h-[1px] bg-gradient-to-r from-transparent via-cyan-500/50 to-transparent" />

      <div className="relative container mx-auto px-6">
        <div className="flex items-center justify-between py-4">
          {/* Logo Section */}
          <div className="flex items-center gap-4">
            <div className="relative group">
              <div className="absolute inset-0 bg-gradient-to-r from-cyan-500 to-blue-500 rounded-xl blur-lg opacity-50 group-hover:opacity-75 transition-opacity" />
              <div className="relative p-3 rounded-xl bg-gradient-to-br from-cyan-500 to-blue-600 shadow-xl">
                <Wallet className="w-6 h-6 text-white" />
              </div>
            </div>

            <div>
              <h1 className="text-2xl font-bold bg-gradient-to-r from-cyan-400 via-blue-400 to-purple-400 bg-clip-text text-transparent">
                DelTran
              </h1>
              <p className="text-xs text-zinc-400 uppercase tracking-wider">
                Premium Gateway
              </p>
            </div>
          </div>

          {/* Navigation Buttons - 13 Items */}
          <nav className="flex items-center gap-2">
            {NAV_ITEMS.map((item, index) => {
              const Icon = item.icon;
              const isActive = pathname === item.path;
              const isHovered = hoveredIndex === index;

              return (
                <button
                  key={item.path}
                  onClick={() => router.push(item.path)}
                  onMouseEnter={() => setHoveredIndex(index)}
                  onMouseLeave={() => setHoveredIndex(null)}
                  className={`
                    relative group px-4 py-2.5 rounded-lg font-medium transition-all duration-300
                    ${isActive
                      ? 'text-white shadow-xl'
                      : 'text-zinc-400 hover:text-white'
                    }
                  `}
                  style={{
                    transform: isHovered ? 'translateY(-2px)' : 'translateY(0)',
                  }}
                >
                  {/* Animated background for active/hover state */}
                  {(isActive || isHovered) && (
                    <div
                      className={`
                        absolute inset-0 rounded-lg opacity-90 transition-opacity duration-300
                        bg-gradient-to-r ${item.color}
                      `}
                      style={{
                        opacity: isActive ? 0.9 : isHovered ? 0.6 : 0,
                      }}
                    />
                  )}

                  {/* Glow effect */}
                  {isActive && (
                    <div
                      className={`
                        absolute inset-0 rounded-lg blur-xl opacity-50
                        bg-gradient-to-r ${item.color}
                      `}
                    />
                  )}

                  {/* Content */}
                  <div className="relative flex items-center gap-2">
                    <Icon className={`w-4 h-4 ${isActive ? 'animate-pulse' : ''}`} />
                    <span className="text-sm font-semibold">{item.name}</span>

                    {/* Badge */}
                    {item.badge && (
                      <span className="px-2 py-0.5 text-[10px] font-bold bg-red-500 text-white rounded-full animate-pulse">
                        {item.badge}
                      </span>
                    )}
                  </div>

                  {/* Shine effect on hover */}
                  <div
                    className={`
                      absolute inset-0 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity duration-500
                      bg-gradient-to-r from-transparent via-white/20 to-transparent
                    `}
                    style={{
                      transform: 'translateX(-100%)',
                      animation: isHovered ? 'shine 1.5s infinite' : 'none',
                    }}
                  />
                </button>
              );
            })}

            {/* Logout Button */}
            <button
              onClick={handleLogout}
              className="relative group ml-2 px-4 py-2.5 rounded-lg font-medium text-zinc-400 hover:text-white transition-all duration-300"
              onMouseEnter={() => setHoveredIndex(99)}
              onMouseLeave={() => setHoveredIndex(null)}
            >
              <div
                className={`
                  absolute inset-0 rounded-lg bg-gradient-to-r from-red-500 to-rose-600 opacity-0 group-hover:opacity-60 transition-opacity duration-300
                `}
              />
              <div className="relative flex items-center gap-2">
                <LogOut className="w-4 h-4" />
                <span className="text-sm font-semibold">Logout</span>
              </div>
            </button>
          </nav>
        </div>
      </div>

      <style jsx>{`
        @keyframes shine {
          0% {
            transform: translateX(-100%);
          }
          100% {
            transform: translateX(200%);
          }
        }
      `}</style>
    </header>
  );
}
