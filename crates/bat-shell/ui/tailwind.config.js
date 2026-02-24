/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './index.html',
    './src/**/*.{ts,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        brand: {
          green: '#39FF14',
          'green-dim': '#2bcc10',
        },
      },
    },
  },
  plugins: [],
}
