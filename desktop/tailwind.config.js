/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: "#1a1d27",
        border: "#2d3143",
        accent: "#6366f1",
        "accent-hover": "#818cf8",
        muted: "#64748b",
      },
    },
  },
  plugins: [],
};
