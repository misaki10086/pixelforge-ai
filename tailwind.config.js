/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: ["selector", '[data-theme="dark"]'],
  theme: {
    extend: {
      colors: {
        // 所有语义色均为 CSS 变量（rgb 三元组），支持 Dark / Light 主题切换
        base: "rgb(var(--c-base) / <alpha-value>)",
        panel: "rgb(var(--c-panel) / <alpha-value>)",
        elevated: "rgb(var(--c-elevated) / <alpha-value>)",
        input: "rgb(var(--c-input) / <alpha-value>)",
        hover: "rgb(var(--c-hover) / <alpha-value>)",
        line: "rgb(var(--c-line) / <alpha-value>)",
        linestrong: "rgb(var(--c-line-strong) / <alpha-value>)",
        ink: "rgb(var(--c-ink) / <alpha-value>)",
        inkdim: "rgb(var(--c-ink-dim) / <alpha-value>)",
        inkfaint: "rgb(var(--c-ink-faint) / <alpha-value>)",
        accent: "rgb(var(--c-accent) / <alpha-value>)",
        accentdim: "rgb(var(--c-accent-dim) / <alpha-value>)",
        ok: "rgb(var(--c-ok) / <alpha-value>)",
        warn: "rgb(var(--c-warn) / <alpha-value>)",
        bad: "rgb(var(--c-bad) / <alpha-value>)",
        info: "rgb(var(--c-info) / <alpha-value>)",
        selbg: "rgb(var(--c-sel-bg) / <alpha-value>)",
      },
      fontFamily: {
        sans: [
          "Segoe UI",
          "system-ui",
          "-apple-system",
          "PingFang SC",
          "Microsoft YaHei",
          "sans-serif",
        ],
        mono: ["Cascadia Mono", "Consolas", "Menlo", "monospace"],
      },
      fontSize: {
        "2xs": ["11px", "14px"],
      },
    },
  },
  plugins: [],
};
