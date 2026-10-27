/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        bg: 'var(--sp-bg)',
        card: 'var(--sp-surface)',
        border: 'var(--sp-border)',
        accent: 'var(--sp-primary)',
        green: '#16835d',
        yellow: '#d29922',
        red: 'var(--sp-primary)',
        muted: 'var(--sp-text-3)',
      },
      fontFamily: {
        sans: ['var(--sp-font-sans)'],
        mono: ['var(--sp-font-mono)'],
      },
    },
  },
  plugins: [],
}
