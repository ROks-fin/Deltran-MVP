/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./app/**/*.{js,ts,jsx,tsx,mdx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Premium Gold Palette
        'deltran-gold': {
          DEFAULT: '#d4af37',
          light: '#e6c757',
          dark: '#b89730',
          glow: 'rgba(212, 175, 55, 0.3)',
        },
        // Dark Premium Backgrounds
        'deltran-dark': {
          DEFAULT: '#0a0a0a',
          midnight: '#0a0a0a',
          charcoal: '#1a1a1a',
          obsidian: '#0f0f0f',
          card: '#1a1a1a',
        },
        // Light Premium Accents
        'deltran-light': {
          platinum: '#e5e5e7',
          pearl: '#f8f8f8',
        },
      },
      fontFamily: {
        sans: ['Inter', 'Helvetica Neue', 'sans-serif'],
        serif: ['Playfair Display', 'Georgia', 'serif'],
        mono: ['JetBrains Mono', 'Consolas', 'monospace'],
      },
      fontSize: {
        'display-xl': ['4.5rem', { lineHeight: '1.1', letterSpacing: '-0.02em' }],
        'display-lg': ['3.75rem', { lineHeight: '1.1', letterSpacing: '-0.02em' }],
        'display-md': ['3rem', { lineHeight: '1.2', letterSpacing: '-0.01em' }],
      },
      spacing: {
        '18': '4.5rem',
        '22': '5.5rem',
        '88': '22rem',
        '128': '32rem',
      },
      borderRadius: {
        'xl': '1rem',
        '2xl': '1.5rem',
        '3xl': '2rem',
      },
      boxShadow: {
        'gold-sm': '0 2px 8px rgba(212, 175, 55, 0.15)',
        'gold-md': '0 4px 16px rgba(212, 175, 55, 0.2)',
        'gold-lg': '0 8px 32px rgba(212, 175, 55, 0.25)',
        'gold-xl': '0 16px 48px rgba(212, 175, 55, 0.3)',
        'premium': '0 25px 50px -12px rgba(0, 0, 0, 0.5)',
        'inner-gold': 'inset 0 2px 4px rgba(212, 175, 55, 0.1)',
      },
      animation: {
        'shimmer': 'shimmer 2.5s linear infinite',
        'float': 'float 6s ease-in-out infinite',
        'pulse-gold': 'pulse-gold 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'glow': 'glow 2s ease-in-out infinite',
        'slide-up': 'slide-up 0.5s cubic-bezier(0.4, 0, 0.2, 1)',
        'slide-down': 'slide-down 0.5s cubic-bezier(0.4, 0, 0.2, 1)',
        'fade-in': 'fade-in 0.5s cubic-bezier(0.4, 0, 0.2, 1)',
        'scale-in': 'scale-in 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
      },
      keyframes: {
        shimmer: {
          '0%': { backgroundPosition: '-200% center' },
          '100%': { backgroundPosition: '200% center' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0px)' },
          '50%': { transform: 'translateY(-20px)' },
        },
        'pulse-gold': {
          '0%, 100%': { opacity: '1', boxShadow: '0 0 0 0 rgba(212, 175, 55, 0.7)' },
          '50%': { opacity: '0.8', boxShadow: '0 0 0 10px rgba(212, 175, 55, 0)' },
        },
        glow: {
          '0%, 100%': { filter: 'brightness(1) drop-shadow(0 0 5px rgba(212, 175, 55, 0.5))' },
          '50%': { filter: 'brightness(1.2) drop-shadow(0 0 15px rgba(212, 175, 55, 0.8))' },
        },
        'slide-up': {
          '0%': { transform: 'translateY(30px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        'slide-down': {
          '0%': { transform: 'translateY(-30px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        'fade-in': {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        'scale-in': {
          '0%': { transform: 'scale(0.95)', opacity: '0' },
          '100%': { transform: 'scale(1)', opacity: '1' },
        },
      },
      backdropBlur: {
        xs: '2px',
        '3xl': '64px',
      },
      transitionTimingFunction: {
        'premium': 'cubic-bezier(0.4, 0, 0.2, 1)',
        'bounce-soft': 'cubic-bezier(0.34, 1.56, 0.64, 1)',
      },
      transitionDuration: {
        '400': '400ms',
        '600': '600ms',
      },
    },
  },
  plugins: [],
}
