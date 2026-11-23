/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#182230",
        muted: "#667085",
        brand: "#6558e8",
      },
      boxShadow: {
        panel: "0 10px 35px rgba(27, 38, 59, 0.06)",
      },
    },
  },
  plugins: [],
};
