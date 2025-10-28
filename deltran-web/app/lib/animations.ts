/**
 * Premium Animation Library
 * Ultra-smooth animations following the "Digital Luxury Finance" philosophy
 */

import { Variants, Transition } from 'framer-motion';

// ============================================================================
// EASING FUNCTIONS - Premium timing functions
// ============================================================================

export const EASE = {
  premium: [0.4, 0, 0.2, 1], // Main easing for all transitions
  bounce: [0.34, 1.56, 0.64, 1], // Soft bounce
  smooth: [0.25, 0.46, 0.45, 0.94], // Ultra smooth
  sharp: [0.4, 0, 0.6, 1], // Quick and decisive
} as const;

// ============================================================================
// DURATION CONSTANTS - Timing hierarchy
// ============================================================================

export const DURATION = {
  instant: 0.15,
  fast: 0.2,
  base: 0.3,
  moderate: 0.4,
  slow: 0.5,
  slower: 0.8,
  crawl: 1.2,
} as const;

// ============================================================================
// STAGGER CONFIGURATIONS - Cascading animations
// ============================================================================

export const STAGGER = {
  fast: 0.05,
  base: 0.08,
  slow: 0.12,
  slower: 0.2,
} as const;

// ============================================================================
// "LIQUID GOLD FLOW" - Signature entrance animation
// ============================================================================

export const liquidGoldFlow: Variants = {
  hidden: {
    opacity: 0,
    y: 30,
    filter: 'blur(20px)',
    scale: 0.95,
  },
  visible: {
    opacity: 1,
    y: 0,
    filter: 'blur(0px)',
    scale: 1,
    transition: {
      duration: DURATION.slow,
      ease: EASE.premium,
    },
  },
};

// ============================================================================
// "FLOATING CRYSTALS" - Card hover animations
// ============================================================================

export const floatingCrystal: Variants = {
  rest: {
    y: 0,
    scale: 1,
    boxShadow: '0 4px 16px rgba(212, 175, 55, 0.2)',
  },
  hover: {
    y: -10,
    scale: 1.02,
    boxShadow: '0 16px 48px rgba(212, 175, 55, 0.3)',
    transition: {
      duration: DURATION.moderate,
      ease: EASE.bounce,
    },
  },
  tap: {
    y: -5,
    scale: 0.98,
    transition: {
      duration: DURATION.fast,
    },
  },
};

// ============================================================================
// "MAGNETIC HOVER" - Button interactions
// ============================================================================

export const magneticHover: Variants = {
  rest: {
    scale: 1,
  },
  hover: {
    scale: 1.05,
    transition: {
      duration: DURATION.base,
      ease: EASE.bounce,
    },
  },
  tap: {
    scale: 0.95,
    transition: {
      duration: DURATION.fast,
    },
  },
};

// ============================================================================
// "QUANTUM TRANSITION" - Page transitions
// ============================================================================

export const quantumTransition = {
  exit: {
    opacity: 0,
    scale: 0.95,
    filter: 'blur(10px)',
    transition: {
      duration: DURATION.moderate,
      ease: EASE.premium,
    },
  },
  enter: {
    opacity: 0,
    scale: 1.05,
    filter: 'blur(10px)',
  },
  center: {
    opacity: 1,
    scale: 1,
    filter: 'blur(0px)',
    transition: {
      duration: DURATION.slow,
      ease: EASE.premium,
    },
  },
};

// ============================================================================
// STAGGER CONTAINER - Parent for cascading children
// ============================================================================

export const staggerContainer = (staggerDelay: number = STAGGER.base): Variants => ({
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: {
      staggerChildren: staggerDelay,
      delayChildren: 0.1,
    },
  },
});

// ============================================================================
// SLIDE ANIMATIONS - Directional entrances
// ============================================================================

export const slideFromLeft: Variants = {
  hidden: { x: -60, opacity: 0 },
  visible: {
    x: 0,
    opacity: 1,
    transition: {
      duration: DURATION.slow,
      ease: EASE.premium,
    },
  },
};

export const slideFromRight: Variants = {
  hidden: { x: 60, opacity: 0 },
  visible: {
    x: 0,
    opacity: 1,
    transition: {
      duration: DURATION.slow,
      ease: EASE.premium,
    },
  },
};

export const slideFromTop: Variants = {
  hidden: { y: -60, opacity: 0 },
  visible: {
    y: 0,
    opacity: 1,
    transition: {
      duration: DURATION.slow,
      ease: EASE.premium,
    },
  },
};

export const slideFromBottom: Variants = {
  hidden: { y: 60, opacity: 0 },
  visible: {
    y: 0,
    opacity: 1,
    transition: {
      duration: DURATION.slow,
      ease: EASE.premium,
    },
  },
};

// ============================================================================
// SCALE ANIMATIONS - Growth effects
// ============================================================================

export const scaleIn: Variants = {
  hidden: {
    opacity: 0,
    scale: 0.8,
  },
  visible: {
    opacity: 1,
    scale: 1,
    transition: {
      duration: DURATION.moderate,
      ease: EASE.bounce,
    },
  },
};

export const scaleOut: Variants = {
  hidden: {
    opacity: 0,
    scale: 1.2,
  },
  visible: {
    opacity: 1,
    scale: 1,
    transition: {
      duration: DURATION.moderate,
      ease: EASE.premium,
    },
  },
};

// ============================================================================
// FADE ANIMATIONS - Simple opacity
// ============================================================================

export const fadeIn: Variants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: {
      duration: DURATION.moderate,
      ease: EASE.smooth,
    },
  },
};

export const fadeInUp: Variants = {
  hidden: { opacity: 0, y: 20 },
  visible: {
    opacity: 1,
    y: 0,
    transition: {
      duration: DURATION.moderate,
      ease: EASE.premium,
    },
  },
};

// ============================================================================
// SHIMMER EFFECT - Loading states
// ============================================================================

export const shimmerAnimation: Transition = {
  repeat: Infinity,
  repeatType: 'loop',
  duration: 2.5,
  ease: 'linear',
};

// ============================================================================
// PULSE EFFECT - Attention grabber
// ============================================================================

export const pulseGold: Transition = {
  repeat: Infinity,
  repeatType: 'reverse',
  duration: 2,
  ease: EASE.smooth,
};

// ============================================================================
// ROTATION ANIMATIONS - Spinning effects
// ============================================================================

export const rotate360: Transition = {
  repeat: Infinity,
  duration: 3,
  ease: 'linear',
};

// ============================================================================
// MODAL/DIALOG ANIMATIONS - Overlay effects
// ============================================================================

export const modalOverlay: Variants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: {
      duration: DURATION.base,
    },
  },
  exit: {
    opacity: 0,
    transition: {
      duration: DURATION.base,
    },
  },
};

export const modalContent: Variants = {
  hidden: {
    opacity: 0,
    scale: 0.9,
    y: 20,
  },
  visible: {
    opacity: 1,
    scale: 1,
    y: 0,
    transition: {
      duration: DURATION.moderate,
      ease: EASE.bounce,
    },
  },
  exit: {
    opacity: 0,
    scale: 0.95,
    y: 20,
    transition: {
      duration: DURATION.base,
      ease: EASE.premium,
    },
  },
};

// ============================================================================
// TOAST/NOTIFICATION ANIMATIONS
// ============================================================================

export const toastSlideIn: Variants = {
  hidden: {
    x: 400,
    opacity: 0,
  },
  visible: {
    x: 0,
    opacity: 1,
    transition: {
      duration: DURATION.moderate,
      ease: EASE.bounce,
    },
  },
  exit: {
    x: 400,
    opacity: 0,
    transition: {
      duration: DURATION.base,
      ease: EASE.premium,
    },
  },
};

// ============================================================================
// NUMBER COUNTER ANIMATION - Animated values
// ============================================================================

export const numberCounterTransition: Transition = {
  duration: DURATION.crawl,
  ease: EASE.smooth,
};

// ============================================================================
// HOVER LIFT - Subtle elevation on hover
// ============================================================================

export const hoverLift = {
  rest: {
    y: 0,
    transition: {
      duration: DURATION.base,
      ease: EASE.premium,
    },
  },
  hover: {
    y: -4,
    transition: {
      duration: DURATION.base,
      ease: EASE.bounce,
    },
  },
};

// ============================================================================
// GLOW EFFECT - Pulsating glow
// ============================================================================

export const glowPulse: Variants = {
  initial: {
    boxShadow: '0 0 20px rgba(212, 175, 55, 0.3)',
  },
  animate: {
    boxShadow: [
      '0 0 20px rgba(212, 175, 55, 0.3)',
      '0 0 40px rgba(212, 175, 55, 0.5)',
      '0 0 20px rgba(212, 175, 55, 0.3)',
    ],
    transition: {
      duration: 2,
      repeat: Infinity,
      ease: EASE.smooth,
    },
  },
};

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Create a custom delay for staggered animations
 */
export const createStaggerDelay = (index: number, baseDelay: number = STAGGER.base) => ({
  delay: index * baseDelay,
});

/**
 * Create a spring animation config
 */
export const springConfig = (stiffness: number = 300, damping: number = 30) => ({
  type: 'spring' as const,
  stiffness,
  damping,
});

/**
 * Create a custom transition with duration and easing
 */
export const customTransition = (
  duration: number = DURATION.base,
  ease: readonly number[] = EASE.premium
): Transition => ({
  duration,
  ease: ease as any,
});
