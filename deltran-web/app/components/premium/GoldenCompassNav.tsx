'use client';

import React, { useState } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { motion, AnimatePresence } from 'framer-motion';
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
  Menu,
  X,
  ChevronRight,
} from 'lucide-react';
import { slideFromLeft, liquidGoldFlow, DURATION, EASE } from '@/app/lib/animations';

interface NavItem {
  name: string;
  path: string;
  icon: React.ElementType;
  badge?: string | number;
  submenu?: NavItem[];
}

const NAV_ITEMS: NavItem[] = [
  {
    name: 'Dashboard',
    path: '/',
    icon: LayoutDashboard,
  },
  {
    name: 'Payments',
    path: '/payments',
    icon: Wallet,
    badge: 142,
  },
  {
    name: 'Banks',
    path: '/banks',
    icon: Building2,
  },
  {
    name: 'Compliance',
    path: '/compliance',
    icon: Shield,
    badge: 5,
  },
  {
    name: 'Audit Reports',
    path: '/audit',
    icon: FileText,
  },
  {
    name: 'Reports',
    path: '/reports',
    icon: BarChart3,
  },
  {
    name: 'Analytics',
    path: '/analytics',
    icon: TrendingUp,
  },
  {
    name: 'Transactions',
    path: '/transactions',
    icon: Activity,
  },
  {
    name: 'Users',
    path: '/users',
    icon: Users,
  },
  {
    name: 'Network',
    path: '/network',
    icon: Globe,
  },
  {
    name: 'Database',
    path: '/database',
    icon: Database,
  },
  {
    name: 'Settings',
    path: '/settings',
    icon: Settings,
  },
];

export function GoldenCompassNav() {
  const pathname = usePathname();
  const router = useRouter();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const [expandedSubmenu, setExpandedSubmenu] = useState<number | null>(null);

  const handleLogout = () => {
    localStorage.removeItem('access_token');
    router.push('/login');
  };

  const handleNavigation = (path: string, index: number) => {
    router.push(path);
  };

  return (
    <>
      {/* Sidebar Navigation */}
      <motion.aside
        initial={{ x: -280 }}
        animate={{ x: 0, width: isCollapsed ? 80 : 280 }}
        transition={{ duration: 0.5, ease: EASE.premium }}
        className="fixed left-0 top-0 h-screen z-50 glass border-r border-white/10"
      >
        {/* Background gradient */}
        <div className="absolute inset-0 bg-gradient-to-b from-deltran-dark-midnight via-deltran-dark-obsidian to-deltran-dark-midnight" />

        {/* Frosted glass overlay */}
        <div className="absolute inset-0 backdrop-blur-3xl bg-white/[0.02]" />

        {/* Golden accent line */}
        <motion.div
          className="absolute right-0 top-0 bottom-0 w-[1px]"
          animate={{
            background: [
              'linear-gradient(180deg, transparent 0%, rgba(212, 175, 55, 0.3) 50%, transparent 100%)',
              'linear-gradient(180deg, transparent 30%, rgba(212, 175, 55, 0.5) 50%, transparent 70%)',
              'linear-gradient(180deg, transparent 0%, rgba(212, 175, 55, 0.3) 50%, transparent 100%)',
            ],
          }}
          transition={{ duration: 3, repeat: Infinity, ease: 'linear' }}
        />

        {/* Content */}
        <div className="relative h-full flex flex-col">
          {/* Header */}
          <div className="p-6 border-b border-white/5">
            <div className="flex items-center justify-between">
              <AnimatePresence mode="wait">
                {!isCollapsed && (
                  <motion.div
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: -20 }}
                    transition={{ duration: 0.3 }}
                    className="flex items-center gap-3"
                  >
                    {/* Logo */}
                    <motion.div
                      animate={{
                        rotate: [0, 5, -5, 0],
                      }}
                      transition={{
                        duration: 4,
                        repeat: Infinity,
                        ease: 'easeInOut',
                      }}
                      className="relative"
                    >
                      <div className="absolute inset-0 bg-deltran-gold rounded-xl blur-lg opacity-50" />
                      <div className="relative p-2 rounded-xl bg-gradient-to-br from-deltran-gold to-deltran-gold-dark">
                        <Wallet className="w-6 h-6 text-white" />
                      </div>
                    </motion.div>

                    {/* Brand */}
                    <div>
                      <h1 className="text-xl font-bold text-gradient-gold">DelTran</h1>
                      <p className="text-[10px] text-zinc-500 uppercase tracking-widest">
                        Premium Gateway
                      </p>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Toggle Button */}
              <motion.button
                onClick={() => setIsCollapsed(!isCollapsed)}
                whileHover={{ scale: 1.1 }}
                whileTap={{ scale: 0.95 }}
                className="p-2 rounded-lg hover:bg-white/5 text-zinc-400 hover:text-deltran-gold transition-colors"
              >
                {isCollapsed ? <Menu size={20} /> : <X size={20} />}
              </motion.button>
            </div>
          </div>

          {/* Navigation Items */}
          <nav className="flex-1 overflow-y-auto py-6 px-3 space-y-1 scrollbar-thin scrollbar-thumb-deltran-gold/30">
            {NAV_ITEMS.map((item, index) => {
              const Icon = item.icon;
              const isActive = pathname === item.path;
              const isHovered = hoveredIndex === index;

              return (
                <motion.div
                  key={item.path}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.05, duration: 0.3 }}
                >
                  <motion.button
                    onClick={() => handleNavigation(item.path, index)}
                    onMouseEnter={() => setHoveredIndex(index)}
                    onMouseLeave={() => setHoveredIndex(null)}
                    whileHover={{ x: 5 }}
                    className={`
                      relative w-full group
                      flex items-center gap-3
                      px-4 py-3
                      rounded-xl
                      transition-all duration-300
                      ${
                        isActive
                          ? 'text-white bg-gradient-to-r from-deltran-gold/20 to-transparent'
                          : 'text-zinc-400 hover:text-white hover:bg-white/5'
                      }
                    `}
                  >
                    {/* Golden accent line for active item */}
                    {isActive && (
                      <motion.div
                        layoutId="activeIndicator"
                        className="absolute left-0 top-0 bottom-0 w-1 bg-gradient-to-b from-deltran-gold via-deltran-gold-light to-deltran-gold rounded-r-full"
                        transition={{ duration: 0.3, ease: EASE.premium }}
                      />
                    )}

                    {/* Icon with glow effect */}
                    <motion.div
                      animate={{
                        rotate: isActive ? [0, 5, -5, 0] : 0,
                        scale: isHovered ? 1.1 : 1,
                      }}
                      transition={{ duration: 0.3 }}
                      className={`
                        relative
                        ${isActive ? 'text-deltran-gold' : ''}
                      `}
                    >
                      {isActive && (
                        <motion.div
                          className="absolute inset-0 bg-deltran-gold rounded-lg blur-lg opacity-50"
                          animate={{ opacity: [0.3, 0.6, 0.3] }}
                          transition={{ duration: 2, repeat: Infinity }}
                        />
                      )}
                      <Icon size={20} className="relative" />
                    </motion.div>

                    {/* Label */}
                    <AnimatePresence mode="wait">
                      {!isCollapsed && (
                        <motion.span
                          initial={{ opacity: 0, width: 0 }}
                          animate={{ opacity: 1, width: 'auto' }}
                          exit={{ opacity: 0, width: 0 }}
                          transition={{ duration: 0.3 }}
                          className={`
                            text-sm font-medium whitespace-nowrap
                            ${isActive ? 'font-semibold' : ''}
                          `}
                        >
                          {item.name}
                        </motion.span>
                      )}
                    </AnimatePresence>

                    {/* Badge */}
                    {item.badge && !isCollapsed && (
                      <motion.span
                        initial={{ scale: 0 }}
                        animate={{ scale: 1 }}
                        className="ml-auto px-2 py-0.5 text-xs font-bold bg-deltran-gold text-deltran-dark-midnight rounded-full"
                      >
                        {item.badge}
                      </motion.span>
                    )}

                    {/* Hover shine effect */}
                    {isHovered && (
                      <motion.div
                        className="absolute inset-0 rounded-xl bg-gradient-to-r from-transparent via-white/5 to-transparent"
                        initial={{ x: '-100%' }}
                        animate={{ x: '100%' }}
                        transition={{ duration: 0.6, ease: 'linear' }}
                      />
                    )}
                  </motion.button>
                </motion.div>
              );
            })}
          </nav>

          {/* Footer - Logout */}
          <div className="p-4 border-t border-white/5">
            <motion.button
              onClick={handleLogout}
              whileHover={{ scale: 1.02, x: 5 }}
              whileTap={{ scale: 0.98 }}
              className={`
                w-full
                flex items-center gap-3
                px-4 py-3
                rounded-xl
                text-zinc-400 hover:text-red-400
                hover:bg-red-500/10
                transition-all duration-300
                group
              `}
            >
              <LogOut size={20} />
              <AnimatePresence mode="wait">
                {!isCollapsed && (
                  <motion.span
                    initial={{ opacity: 0, width: 0 }}
                    animate={{ opacity: 1, width: 'auto' }}
                    exit={{ opacity: 0, width: 0 }}
                    className="text-sm font-medium"
                  >
                    Logout
                  </motion.span>
                )}
              </AnimatePresence>
            </motion.button>
          </div>
        </div>
      </motion.aside>

      {/* Main content offset */}
      <motion.div
        animate={{ marginLeft: isCollapsed ? 80 : 280 }}
        transition={{ duration: 0.5, ease: EASE.premium }}
        className="min-h-screen"
      />
    </>
  );
}
