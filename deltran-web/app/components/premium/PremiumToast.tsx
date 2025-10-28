'use client';

import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { CheckCircle, XCircle, AlertCircle, Info, X } from 'lucide-react';
import { toastSlideIn } from '@/app/lib/animations';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;
}

interface ToastContextType {
  showToast: (type: ToastType, title: string, message?: string, duration?: number) => void;
}

// Toast Store (simple state management)
class ToastStore {
  private listeners: Set<(toasts: Toast[]) => void> = new Set();
  private toasts: Toast[] = [];

  subscribe(listener: (toasts: Toast[]) => void) {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  addToast(toast: Omit<Toast, 'id'>) {
    const id = Math.random().toString(36).substring(7);
    const newToast = { ...toast, id };
    this.toasts = [...this.toasts, newToast];
    this.notify();

    // Auto remove after duration
    setTimeout(() => {
      this.removeToast(id);
    }, toast.duration || 5000);

    return id;
  }

  removeToast(id: string) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
    this.notify();
  }

  private notify() {
    this.listeners.forEach((listener) => listener(this.toasts));
  }
}

const toastStore = new ToastStore();

// Hook to use toasts
export function useToast() {
  const showToast = (type: ToastType, title: string, message?: string, duration?: number) => {
    toastStore.addToast({ type, title, message, duration });
  };

  return { showToast };
}

// Toast Container Component
export function PremiumToastContainer() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    return toastStore.subscribe(setToasts);
  }, []);

  return (
    <div className="fixed top-4 right-4 z-[100] flex flex-col gap-3 pointer-events-none">
      <AnimatePresence mode="popLayout">
        {toasts.map((toast) => (
          <ToastItem
            key={toast.id}
            toast={toast}
            onClose={() => toastStore.removeToast(toast.id)}
          />
        ))}
      </AnimatePresence>
    </div>
  );
}

// Individual Toast Item
function ToastItem({ toast, onClose }: { toast: Toast; onClose: () => void }) {
  const [particles, setParticles] = useState<Array<{ x: number; y: number; delay: number }>>([]);

  useEffect(() => {
    // Generate particles for success toast
    if (toast.type === 'success') {
      const newParticles = Array.from({ length: 12 }, (_, i) => ({
        x: Math.random() * 200 - 100,
        y: Math.random() * 200 - 100,
        delay: i * 0.05,
      }));
      setParticles(newParticles);
    }
  }, [toast.type]);

  const icons = {
    success: CheckCircle,
    error: XCircle,
    warning: AlertCircle,
    info: Info,
  };

  const colors = {
    success: {
      bg: 'from-emerald-500/20 to-green-500/20',
      border: 'border-emerald-500/30',
      icon: 'text-emerald-400',
      glow: 'shadow-emerald-500/20',
    },
    error: {
      bg: 'from-red-500/20 to-rose-500/20',
      border: 'border-red-500/30',
      icon: 'text-red-400',
      glow: 'shadow-red-500/20',
    },
    warning: {
      bg: 'from-amber-500/20 to-yellow-500/20',
      border: 'border-amber-500/30',
      icon: 'text-amber-400',
      glow: 'shadow-amber-500/20',
    },
    info: {
      bg: 'from-blue-500/20 to-cyan-500/20',
      border: 'border-blue-500/30',
      icon: 'text-blue-400',
      glow: 'shadow-blue-500/20',
    },
  };

  const Icon = icons[toast.type];
  const colorScheme = colors[toast.type];

  return (
    <motion.div
      variants={toastSlideIn}
      initial="hidden"
      animate="visible"
      exit="exit"
      layout
      className="relative pointer-events-auto"
    >
      {/* Particles for success */}
      {toast.type === 'success' && (
        <div className="absolute inset-0 pointer-events-none">
          {particles.map((particle, i) => (
            <motion.div
              key={i}
              initial={{ x: 0, y: 0, scale: 1, opacity: 0 }}
              animate={{
                x: particle.x,
                y: particle.y,
                scale: 0,
                opacity: [0, 1, 0],
              }}
              transition={{
                duration: 1.5,
                delay: particle.delay,
                ease: 'easeOut',
              }}
              className="absolute top-1/2 left-1/2 w-2 h-2 rounded-full bg-emerald-400"
            />
          ))}
        </div>
      )}

      {/* Toast Card */}
      <motion.div
        whileHover={{ scale: 1.02, x: -5 }}
        className={`
          relative
          min-w-[400px]
          rounded-xl
          bg-gradient-to-r ${colorScheme.bg}
          backdrop-blur-xl
          border ${colorScheme.border}
          shadow-lg ${colorScheme.glow}
          overflow-hidden
        `}
      >
        {/* Background gradient overlay */}
        <div className="absolute inset-0 bg-deltran-dark-charcoal/90" />

        {/* Shimmer effect */}
        <motion.div
          className="absolute inset-0 opacity-0"
          animate={{
            opacity: [0, 0.3, 0],
            x: ['-100%', '100%'],
          }}
          transition={{
            duration: 2,
            repeat: Infinity,
            ease: 'linear',
          }}
        >
          <div className="h-full w-full bg-gradient-to-r from-transparent via-white/10 to-transparent" />
        </motion.div>

        {/* Content */}
        <div className="relative p-4 flex items-start gap-4">
          {/* Icon with pulsing glow */}
          <motion.div
            animate={{
              scale: [1, 1.1, 1],
            }}
            transition={{
              duration: 2,
              repeat: Infinity,
              ease: 'easeInOut',
            }}
            className="relative"
          >
            <motion.div
              animate={{
                opacity: [0.3, 0.6, 0.3],
                scale: [1, 1.5, 1],
              }}
              transition={{
                duration: 2,
                repeat: Infinity,
              }}
              className={`absolute inset-0 ${colorScheme.icon} blur-lg`}
            />
            <Icon className={`relative ${colorScheme.icon}`} size={24} />
          </motion.div>

          {/* Text content */}
          <div className="flex-1 min-w-0">
            <motion.h4
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 }}
              className="text-sm font-semibold text-white mb-1"
            >
              {toast.title}
            </motion.h4>
            {toast.message && (
              <motion.p
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ delay: 0.2 }}
                className="text-sm text-zinc-400"
              >
                {toast.message}
              </motion.p>
            )}
          </div>

          {/* Close button */}
          <motion.button
            onClick={onClose}
            whileHover={{ scale: 1.1, rotate: 90 }}
            whileTap={{ scale: 0.9 }}
            className="text-zinc-500 hover:text-white transition-colors"
          >
            <X size={18} />
          </motion.button>
        </div>

        {/* Progress bar */}
        <motion.div
          initial={{ scaleX: 1 }}
          animate={{ scaleX: 0 }}
          transition={{
            duration: (toast.duration || 5000) / 1000,
            ease: 'linear',
          }}
          className={`h-1 ${colorScheme.icon} origin-left`}
        />
      </motion.div>
    </motion.div>
  );
}

// Example usage helper
export const toast = {
  success: (title: string, message?: string, duration?: number) => {
    toastStore.addToast({ type: 'success', title, message, duration });
  },
  error: (title: string, message?: string, duration?: number) => {
    toastStore.addToast({ type: 'error', title, message, duration });
  },
  warning: (title: string, message?: string, duration?: number) => {
    toastStore.addToast({ type: 'warning', title, message, duration });
  },
  info: (title: string, message?: string, duration?: number) => {
    toastStore.addToast({ type: 'info', title, message, duration });
  },
};
