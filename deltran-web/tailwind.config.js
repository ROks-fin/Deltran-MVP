/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./app/**/*.{js,ts,jsx,tsx,mdx}'],
  theme: {
    extend: {
      colors: {
        'deltran-gold': '#d4af37',
        'deltran-gold-light': '#f4cf57',
        'deltran-dark': '#0a0a0a',
      },
    },
  },
  plugins: [],
}
