module.exports = {
  content: ["./App.tsx", "./src/**/*.{ts,tsx}", "./app/**/*.{ts,tsx}"],
  presets: [require("nativewind/preset")],
  theme: { extend: { fontFamily: { sans: ["Inter"] } } },
  plugins: [],
};
