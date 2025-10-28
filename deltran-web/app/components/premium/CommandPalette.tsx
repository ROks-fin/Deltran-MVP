'use client';

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useRouter } from 'next/navigation';
import {
  Search,
  ArrowRight,
  Command,
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
  Clock,
} from 'lucide-react';
import { modalOverlay, modalContent, EASE } from '@/app/lib/animations';

interface CommandItem {
  id: string;
  title: string;
  description?: string;
  icon: React.ElementType;
  action: () => void;
  category: 'navigation' | 'action' | 'recent';
  keywords?: string[];
}

export function CommandPalette() {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [recentCommands, setRecentCommands] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const router = useRouter();

  // Command items
  const commands: CommandItem[] = [
    {
      id: 'dashboard',
      title: 'Dashboard',
      description: 'View main dashboard',
      icon: LayoutDashboard,
      action: () => router.push('/'),
      category: 'navigation',
      keywords: ['home', 'overview'],
    },
    {
      id: 'payments',
      title: 'Payments',
      description: 'Manage payment transactions',
      icon: Wallet,
      action: () => router.push('/payments'),
      category: 'navigation',
      keywords: ['transactions', 'money'],
    },
    {
      id: 'banks',
      title: 'Banks',
      description: 'View connected banks',
      icon: Building2,
      action: () => router.push('/banks'),
      category: 'navigation',
      keywords: ['financial', 'institutions'],
    },
    {
      id: 'compliance',
      title: 'Compliance',
      description: 'Review compliance queue',
      icon: Shield,
      action: () => router.push('/compliance'),
      category: 'navigation',
      keywords: ['security', 'audit'],
    },
    {
      id: 'audit',
      title: 'Audit Reports',
      description: 'Access audit logs',
      icon: FileText,
      action: () => router.push('/audit'),
      category: 'navigation',
      keywords: ['logs', 'reports'],
    },
    {
      id: 'reports',
      title: 'Reports',
      description: 'Generate reports',
      icon: BarChart3,
      action: () => router.push('/reports'),
      category: 'navigation',
      keywords: ['analytics', 'data'],
    },
    {
      id: 'analytics',
      title: 'Analytics',
      description: 'View analytics dashboard',
      icon: TrendingUp,
      action: () => router.push('/analytics'),
      category: 'navigation',
      keywords: ['metrics', 'stats'],
    },
    {
      id: 'transactions',
      title: 'Transactions',
      description: 'Browse all transactions',
      icon: Activity,
      action: () => router.push('/transactions'),
      category: 'navigation',
      keywords: ['payments', 'history'],
    },
    {
      id: 'users',
      title: 'Users',
      description: 'Manage users',
      icon: Users,
      action: () => router.push('/users'),
      category: 'navigation',
      keywords: ['accounts', 'team'],
    },
    {
      id: 'network',
      title: 'Network',
      description: 'View network status',
      icon: Globe,
      action: () => router.push('/network'),
      category: 'navigation',
      keywords: ['connectivity', 'status'],
    },
    {
      id: 'database',
      title: 'Database',
      description: 'Database management',
      icon: Database,
      action: () => router.push('/database'),
      category: 'navigation',
      keywords: ['data', 'storage'],
    },
    {
      id: 'settings',
      title: 'Settings',
      description: 'Application settings',
      icon: Settings,
      action: () => router.push('/settings'),
      category: 'navigation',
      keywords: ['preferences', 'config'],
    },
  ];

  // Filter commands based on search
  const filteredCommands = commands.filter((cmd) => {
    const searchLower = search.toLowerCase();
    return (
      cmd.title.toLowerCase().includes(searchLower) ||
      cmd.description?.toLowerCase().includes(searchLower) ||
      cmd.keywords?.some((k) => k.includes(searchLower))
    );
  });

  // Add recent commands
  const recentItems: CommandItem[] = recentCommands
    .map((id) => commands.find((cmd) => cmd.id === id))
    .filter(Boolean) as CommandItem[];

  const displayCommands = search ? filteredCommands : [...recentItems, ...commands].slice(0, 8);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Open palette with Cmd+K or Ctrl+K
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setIsOpen(true);
      }

      // Close with Escape
      if (e.key === 'Escape') {
        setIsOpen(false);
        setSearch('');
        setSelectedIndex(0);
      }

      // Navigate with arrow keys
      if (isOpen) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          setSelectedIndex((prev) => (prev + 1) % displayCommands.length);
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          setSelectedIndex((prev) => (prev - 1 + displayCommands.length) % displayCommands.length);
        }
        if (e.key === 'Enter') {
          e.preventDefault();
          executeCommand(displayCommands[selectedIndex]);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, selectedIndex, displayCommands]);

  // Focus input when opened
  useEffect(() => {
    if (isOpen) {
      inputRef.current?.focus();
    }
  }, [isOpen]);

  // Execute command
  const executeCommand = (cmd: CommandItem) => {
    if (!cmd) return;

    // Add to recent commands
    setRecentCommands((prev) => {
      const filtered = prev.filter((id) => id !== cmd.id);
      return [cmd.id, ...filtered].slice(0, 5);
    });

    cmd.action();
    setIsOpen(false);
    setSearch('');
    setSelectedIndex(0);
  };

  return (
    <>
      {/* Trigger Button */}
      <motion.button
        onClick={() => setIsOpen(true)}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        className="
          fixed bottom-8 right-8 z-40
          flex items-center gap-2
          px-4 py-3
          rounded-xl
          bg-deltran-dark-charcoal/80 backdrop-blur-xl
          border border-white/10
          text-zinc-400 hover:text-white
          shadow-gold-md hover:shadow-gold-lg
          transition-all duration-300
          group
        "
      >
        <Search size={18} />
        <span className="text-sm font-medium">Search</span>
        <div className="flex items-center gap-1 px-2 py-0.5 rounded bg-white/5 text-xs font-mono">
          <Command size={12} />
          <span>K</span>
        </div>
      </motion.button>

      {/* Modal */}
      <AnimatePresence>
        {isOpen && (
          <>
            {/* Overlay */}
            <motion.div
              variants={modalOverlay}
              initial="hidden"
              animate="visible"
              exit="exit"
              onClick={() => setIsOpen(false)}
              className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm"
            />

            {/* Palette */}
            <motion.div
              variants={modalContent}
              initial="hidden"
              animate="visible"
              exit="exit"
              className="fixed top-[10%] left-1/2 -translate-x-1/2 z-50 w-full max-w-2xl"
            >
              <div className="glass-gold rounded-2xl shadow-gold-xl overflow-hidden">
                {/* Search Input */}
                <div className="p-4 border-b border-white/10">
                  <div className="relative">
                    <Search className="absolute left-4 top-1/2 -translate-y-1/2 text-zinc-400" size={20} />
                    <input
                      ref={inputRef}
                      type="text"
                      value={search}
                      onChange={(e) => {
                        setSearch(e.target.value);
                        setSelectedIndex(0);
                      }}
                      placeholder="Search commands or navigate..."
                      className="
                        w-full
                        pl-12 pr-4 py-3
                        bg-transparent
                        text-white placeholder:text-zinc-500
                        text-lg
                        outline-none
                      "
                    />
                  </div>
                </div>

                {/* Results */}
                <div className="max-h-96 overflow-y-auto">
                  {displayCommands.length > 0 ? (
                    <div className="p-2">
                      {/* Recent section */}
                      {!search && recentItems.length > 0 && (
                        <div className="mb-2">
                          <div className="px-3 py-2 text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center gap-2">
                            <Clock size={12} />
                            Recent
                          </div>
                          {recentItems.map((cmd, index) => (
                            <CommandItemComponent
                              key={cmd.id}
                              command={cmd}
                              isSelected={index === selectedIndex}
                              onClick={() => executeCommand(cmd)}
                            />
                          ))}
                        </div>
                      )}

                      {/* All commands */}
                      {!search && recentItems.length > 0 && (
                        <div className="px-3 py-2 text-xs font-semibold text-zinc-500 uppercase tracking-wider">
                          All Commands
                        </div>
                      )}
                      {displayCommands.slice(recentItems.length).map((cmd, index) => (
                        <CommandItemComponent
                          key={cmd.id}
                          command={cmd}
                          isSelected={index + recentItems.length === selectedIndex}
                          onClick={() => executeCommand(cmd)}
                        />
                      ))}
                    </div>
                  ) : (
                    <div className="p-8 text-center text-zinc-500">
                      <Search size={48} className="mx-auto mb-4 opacity-20" />
                      <p>No commands found</p>
                    </div>
                  )}
                </div>

                {/* Footer */}
                <div className="p-3 border-t border-white/10 flex items-center justify-between text-xs text-zinc-500">
                  <div className="flex items-center gap-4">
                    <span className="flex items-center gap-1">
                      <kbd className="px-2 py-1 rounded bg-white/5 font-mono">↑</kbd>
                      <kbd className="px-2 py-1 rounded bg-white/5 font-mono">↓</kbd>
                      Navigate
                    </span>
                    <span className="flex items-center gap-1">
                      <kbd className="px-2 py-1 rounded bg-white/5 font-mono">↵</kbd>
                      Select
                    </span>
                  </div>
                  <span className="flex items-center gap-1">
                    <kbd className="px-2 py-1 rounded bg-white/5 font-mono">Esc</kbd>
                    Close
                  </span>
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </>
  );
}

// Command Item Component
function CommandItemComponent({
  command,
  isSelected,
  onClick,
}: {
  command: CommandItem;
  isSelected: boolean;
  onClick: () => void;
}) {
  const Icon = command.icon;

  return (
    <motion.button
      onClick={onClick}
      whileHover={{ x: 5 }}
      className={`
        w-full
        flex items-center gap-3
        px-3 py-3
        rounded-xl
        transition-all duration-200
        ${
          isSelected
            ? 'bg-deltran-gold/20 text-white'
            : 'text-zinc-400 hover:text-white hover:bg-white/5'
        }
      `}
    >
      <div
        className={`
        p-2 rounded-lg
        ${isSelected ? 'bg-deltran-gold text-white' : 'bg-white/5'}
        transition-colors
      `}
      >
        <Icon size={18} />
      </div>

      <div className="flex-1 text-left">
        <div className="font-medium">{command.title}</div>
        {command.description && <div className="text-xs text-zinc-500">{command.description}</div>}
      </div>

      {isSelected && <ArrowRight size={18} className="text-deltran-gold" />}
    </motion.button>
  );
}
