'use client';

import React, { ReactNode, useRef } from 'react';
import { motion, useMotionValue, useSpring, useTransform } from 'framer-motion';
import { magneticHover, DURATION, EASE } from '@/app/lib/animations';

interface PremiumButtonProps {
  children: ReactNode;
  onClick?: () => void;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg' | 'xl';
  disabled?: boolean;
  loading?: boolean;
  magnetic?: boolean;
  icon?: ReactNode;
  iconPosition?: 'left' | 'right';
  className?: string;
  fullWidth?: boolean;
}

/**
 * Premium Button with Magnetic Hover Effect
 * The button subtly follows the cursor when nearby
 */
export function PremiumButton({
  children,
  onClick,
  variant = 'primary',
  size = 'md',
  disabled = false,
  loading = false,
  magnetic = true,
  icon,
  iconPosition = 'left',
  className = '',
  fullWidth = false,
}: PremiumButtonProps) {
  const ref = useRef<HTMLButtonElement>(null);

  // Magnetic effect
  const x = useMotionValue(0);
  const y = useMotionValue(0);

  // Spring animation for smooth magnetic effect
  const springX = useSpring(x, { stiffness: 200, damping: 20 });
  const springY = useSpring(y, { stiffness: 200, damping: 20 });

  const handleMouseMove = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (!magnetic || disabled) return;

    const rect = ref.current?.getBoundingClientRect();
    if (!rect) return;

    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;

    const distanceX = e.clientX - centerX;
    const distanceY = e.clientY - centerY;

    // Limit magnetic pull to 15px
    x.set(Math.max(-15, Math.min(15, distanceX * 0.2)));
    y.set(Math.max(-15, Math.min(15, distanceY * 0.2)));
  };

  const handleMouseLeave = () => {
    x.set(0);
    y.set(0);
  };

  // Variant styles
  const variants = {
    primary: 'bg-gradient-to-r from-deltran-gold to-deltran-gold-light text-white shadow-gold-md hover:shadow-gold-xl',
    secondary: 'bg-deltran-dark-charcoal border-2 border-deltran-gold/30 text-deltran-gold hover:border-deltran-gold',
    ghost: 'bg-transparent border border-white/10 text-white hover:bg-white/5 hover:border-white/20',
    danger: 'bg-gradient-to-r from-red-500 to-rose-600 text-white shadow-lg hover:shadow-xl',
  };

  // Size styles
  const sizes = {
    sm: 'px-4 py-2 text-sm',
    md: 'px-6 py-3 text-base',
    lg: 'px-8 py-4 text-lg',
    xl: 'px-10 py-5 text-xl',
  };

  return (
    <motion.button
      ref={ref}
      onClick={disabled || loading ? undefined : onClick}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      style={{
        x: springX,
        y: springY,
      }}
      variants={magneticHover}
      initial="rest"
      whileHover={!disabled && !loading ? 'hover' : undefined}
      whileTap={!disabled && !loading ? 'tap' : undefined}
      disabled={disabled || loading}
      className={`
        relative
        rounded-xl
        font-semibold
        overflow-hidden
        transition-all
        duration-300
        group
        ${variants[variant]}
        ${sizes[size]}
        ${fullWidth ? 'w-full' : ''}
        ${disabled || loading ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
        ${className}
      `}
    >
      {/* Shimmer effect on hover */}
      <motion.div
        className="absolute inset-0 opacity-0 group-hover:opacity-100"
        initial={{ x: '-100%' }}
        whileHover={{
          x: '100%',
          transition: { duration: 0.7, ease: 'linear' },
        }}
      >
        <div className="h-full w-full bg-gradient-to-r from-transparent via-white/20 to-transparent skew-x-12" />
      </motion.div>

      {/* Glow effect */}
      {variant === 'primary' && (
        <div className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300 blur-xl bg-deltran-gold/30" />
      )}

      {/* Content */}
      <span className="relative z-10 flex items-center justify-center gap-2">
        {loading ? (
          <>
            <motion.div
              animate={{ rotate: 360 }}
              transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
              className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full"
            />
            Loading...
          </>
        ) : (
          <>
            {icon && iconPosition === 'left' && <span>{icon}</span>}
            {children}
            {icon && iconPosition === 'right' && <span>{icon}</span>}
          </>
        )}
      </span>

      {/* Bottom shine */}
      <div className="absolute bottom-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-white/30 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
    </motion.button>
  );
}

/**
 * Icon Button - Circular button for actions
 */
export function IconButton({
  children,
  onClick,
  variant = 'ghost',
  size = 'md',
  disabled = false,
  tooltip,
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: 'primary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  tooltip?: string;
}) {
  const sizeClasses = {
    sm: 'w-8 h-8',
    md: 'w-10 h-10',
    lg: 'w-12 h-12',
  };

  const variantClasses = {
    primary: 'bg-deltran-gold text-white hover:bg-deltran-gold-light',
    ghost: 'bg-transparent hover:bg-white/5 text-zinc-400 hover:text-white',
    danger: 'bg-red-500 text-white hover:bg-red-600',
  };

  return (
    <motion.button
      onClick={disabled ? undefined : onClick}
      variants={magneticHover}
      initial="rest"
      whileHover={!disabled ? 'hover' : undefined}
      whileTap={!disabled ? 'tap' : undefined}
      disabled={disabled}
      title={tooltip}
      className={`
        relative
        rounded-full
        flex items-center justify-center
        transition-all duration-300
        ${sizeClasses[size]}
        ${variantClasses[variant]}
        ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
      `}
    >
      {children}
    </motion.button>
  );
}

/**
 * Button Group - Multiple buttons in a row
 */
export function ButtonGroup({ children }: { children: ReactNode }) {
  return <div className="flex gap-3 flex-wrap">{children}</div>;
}

/**
 * Loading Button State
 */
export function LoadingDots() {
  return (
    <div className="flex gap-1">
      {[0, 1, 2].map((i) => (
        <motion.div
          key={i}
          className="w-2 h-2 bg-current rounded-full"
          animate={{
            y: ['0%', '-50%', '0%'],
            opacity: [1, 0.5, 1],
          }}
          transition={{
            duration: 0.6,
            repeat: Infinity,
            delay: i * 0.1,
          }}
        />
      ))}
    </div>
  );
}
