/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        bg: '#0d1117',
        card: '#161b22',
        border: '#30363d',
        accent: '#58a6ff',
        green: '#3fb950',
        yellow: '#d29922',
        red: '#f85149',
        muted: '#8b949e',
      },
      fontFamily: {
        mono: ['ui-monospace', 'SFMono-Regular', 'Menlo', 'Monaco', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
}
