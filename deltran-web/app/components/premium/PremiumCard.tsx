'use client';

import React, { ReactNode } from 'react';
import { motion, useMotionValue, useTransform } from 'framer-motion';
import { floatingCrystal } from '@/app/lib/animations';

interface PremiumCardProps {
  children: ReactNode;
  className?: string;
  hoverable?: boolean;
  glowEffect?: boolean;
  parallaxStrength?: number;
  delay?: number;
}

/**
 * "Floating Crystals" Premium Card Component
 * Multi-layered card with parallax hover effect and golden glow
 */
export function PremiumCard({
  children,
  className = '',
  hoverable = true,
  glowEffect = true,
  parallaxStrength = 15,
  delay = 0,
}: PremiumCardProps) {
  const x = useMotionValue(0);
  const y = useMotionValue(0);

  const rotateX = useTransform(y, [-100, 100], [parallaxStrength, -parallaxStrength]);
  const rotateY = useTransform(x, [-100, 100], [-parallaxStrength, parallaxStrength]);

  const handleMouseMove = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!hoverable) return;

    const rect = event.currentTarget.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;

    x.set(event.clientX - centerX);
    y.set(event.clientY - centerY);
  };

  const handleMouseLeave = () => {
    x.set(0);
    y.set(0);
  };

  return (
    <motion.div
      initial="rest"
      whileHover={hoverable ? 'hover' : undefined}
      whileTap={hoverable ? 'tap' : undefined}
      variants={floatingCrystal}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      style={{
        rotateX,
        rotateY,
        transformStyle: 'preserve-3d',
      }}
      transition={{ delay }}
      className={`
        relative group
        rounded-2xl
        bg-gradient-to-br from-deltran-dark-charcoal/80 to-deltran-dark-obsidian/80
        backdrop-blur-xl
        border border-white/5
        overflow-hidden
        ${hoverable ? 'cursor-pointer' : ''}
        ${className}
      `}
    >
      {/* Golden glow effect */}
      {glowEffect && (
        <div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-500">
          <div className="absolute inset-0 bg-gradient-to-r from-deltran-gold/5 via-deltran-gold-light/5 to-deltran-gold/5 blur-xl" />
        </div>
      )}

      {/* Shimmer overlay on hover */}
      <motion.div
        className="absolute inset-0 opacity-0 group-hover:opacity-100"
        initial={{ x: '-100%' }}
        whileHover={{
          x: '100%',
          transition: { duration: 1.5, ease: 'linear' },
        }}
      >
        <div className="h-full w-full bg-gradient-to-r from-transparent via-white/5 to-transparent" />
      </motion.div>

      {/* Corner accents */}
      <div className="absolute top-0 right-0 w-24 h-24 bg-gradient-to-br from-deltran-gold/10 to-transparent rounded-bl-full opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
      <div className="absolute bottom-0 left-0 w-24 h-24 bg-gradient-to-tr from-deltran-gold/10 to-transparent rounded-tr-full opacity-0 group-hover:opacity-100 transition-opacity duration-500" />

      {/* Inner glow border */}
      <div className="absolute inset-0 rounded-2xl ring-1 ring-inset ring-white/0 group-hover:ring-deltran-gold/20 transition-all duration-500" />

      {/* Content */}
      <div className="relative z-10" style={{ transform: 'translateZ(20px)' }}>
        {children}
      </div>
    </motion.div>
  );
}

/**
 * Premium Card with glass morphism effect
 */
export function GlassCard({
  children,
  className = '',
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.5 }}
      className={`
        relative
        rounded-2xl
        bg-white/5
        backdrop-blur-3xl
        border border-white/10
        shadow-premium
        ${className}
      `}
    >
      {/* Frosted glass texture */}
      <div className="absolute inset-0 bg-gradient-to-br from-white/5 to-transparent rounded-2xl" />

      {/* Content */}
      <div className="relative z-10">{children}</div>
    </motion.div>
  );
}

/**
 * Premium Card with gold gradient border
 */
export function GoldBorderCard({
  children,
  className = '',
  animated = true,
}: {
  children: ReactNode;
  className?: string;
  animated?: boolean;
}) {
  return (
    <div className={`relative group ${className}`}>
      {/* Animated gradient border */}
      {animated && (
        <motion.div
          className="absolute -inset-[1px] rounded-2xl bg-gradient-to-r from-deltran-gold via-deltran-gold-light to-deltran-gold opacity-0 group-hover:opacity-100 blur-sm"
          animate={{
            backgroundPosition: ['0% 50%', '100% 50%', '0% 50%'],
          }}
          transition={{
            duration: 3,
            repeat: Infinity,
            ease: 'linear',
          }}
          style={{
            backgroundSize: '200% 200%',
          }}
        />
      )}

      {/* Card content */}
      <div className="relative rounded-2xl bg-deltran-dark-charcoal border border-white/10 overflow-hidden">
        {children}
      </div>
    </div>
  );
}

/**
 * Metric Card with animated counter
 */
interface MetricCardProps {
  title: string;
  value: string | number;
  change?: string;
  icon?: ReactNode;
  trend?: 'up' | 'down' | 'neutral';
  delay?: number;
}

export function MetricCard({ title, value, change, icon, trend = 'neutral', delay = 0 }: MetricCardProps) {
  const trendColors = {
    up: 'text-green-400',
    down: 'text-red-400',
    neutral: 'text-zinc-400',
  };

  return (
    <PremiumCard delay={delay} className="p-6">
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <p className="text-sm font-medium text-zinc-400 mb-1">{title}</p>
          <motion.h3
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.5, delay: delay + 0.2 }}
            className="text-3xl font-bold text-gradient-gold"
          >
            {value}
          </motion.h3>
        </div>
        {icon && (
          <motion.div
            initial={{ opacity: 0, rotate: -180 }}
            animate={{ opacity: 1, rotate: 0 }}
            transition={{ duration: 0.5, delay: delay + 0.3 }}
            className="p-3 rounded-xl bg-deltran-gold/10 text-deltran-gold"
          >
            {icon}
          </motion.div>
        )}
      </div>
      {change && (
        <motion.p
          initial={{ opacity: 0, x: -10 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.5, delay: delay + 0.4 }}
          className={`text-sm font-medium ${trendColors[trend]}`}
        >
          {change}
        </motion.p>
      )}
    </PremiumCard>
  );
}
