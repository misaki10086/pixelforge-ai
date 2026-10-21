import { cn } from "@/utils/cn";

/** 内联 SVG 图标集（24x24 stroke），避免引入外部图标库 */
const PATHS: Record<string, string> = {
  dashboard: "M4 4h7v7H4zM13 4h7v4h-7zM13 10h7v10h-7zM4 13h7v7H4z",
  generate: "M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8zM19 16l.9 2.1L22 19l-2.1.9L19 22l-.9-2.1L16 19l2.1-.9z",
  assets: "M4 5h16v14H4zM4 15l4.5-4.5 4 4M12 13l3-3 5 5M15.5 8.5a1 1 0 100 .01",
  animation: "M4 4h16v16H4zM4 9h16M4 14h16M9 4v16M15 4v16",
  editor: "M4 20l4-1L19.5 7.5a2 2 0 00-3-3L5 16zM14 6l3 3",
  projects: "M3 6h6l2 2h10v11H3zM3 6V4h7l2 2",
  styles:
    "M12 3a9 9 0 100 18c1.5 0 2-.9 2-2 0-1.4-1-1.8-1-3 0-1.2 1-2 2.4-2H17a4 4 0 004-4c0-4-4-7-9-7zM7.5 10.5a1 1 0 100 .01M11 7a1 1 0 100 .01M15.5 8.5a1 1 0 100 .01",
  settings:
    "M12 8.5a3.5 3.5 0 100 7 3.5 3.5 0 000-7zM19 12l1.8-1.2-1.4-3.4-2.1.5-1.6-1.6.5-2.1-3.4-1.4L11.6 5h-2.2L8.2 2.8 4.8 4.2l.5 2.1-1.6 1.6-2.1-.5L.2 10.8 2 12l-1.8 1.2 1.4 3.4 2.1-.5 1.6 1.6-.5 2.1 3.4 1.4 1.2-2.2h2.2l1.2 2.2 3.4-1.4-.5-2.1 1.6-1.6 2.1.5 1.4-3.4z",
  play: "M7 4l13 8-13 8z",
  pause: "M6 4h4v16H6zM14 4h4v16h-4z",
  plus: "M12 5v14M5 12h14",
  close: "M6 6l12 12M18 6L6 18",
  chevron: "M6 9l6 6 6-6",
  search: "M10.5 4a6.5 6.5 0 104.6 11.1L20 20M15 15l5 5",
  import: "M12 3v11M7 10l5 5 5-5M4 19h16",
  export: "M12 21V10M7 14l5-5 5 5M4 4h16",
  trash: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6",
  copy: "M9 9h11v11H9zM5 15H4V4h11v1",
  refresh: "M20 12a8 8 0 11-2.3-5.7M20 4v5h-5",
  check: "M4 12l5 5L20 7",
  alert: "M12 3l10 18H2zM12 10v4M12 17.5v.01",
  eye: "M2 12s4-6 10-6 10 6 10 6-4 6-10 6-10-6-10-6zM12 9.5a2.5 2.5 0 100 5 2.5 2.5 0 000-5z",
  wand: "M6 18L18 6M14 4l1 2 2 1-2 1-1 2-1-2-2-1 2-1zM19 12l.7 1.3L21 14l-1.3.7L19 16l-.7-1.3L17 14l1.3-.7z",
  panel: "M4 4h16v16H4zM9 4v16",
  folder: "M3 6h6l2 2h10v11H3z",
  external: "M14 4h6v6M20 4l-9 9M18 13v6H5V6h6",
  clock: "M12 3a9 9 0 100 18 9 9 0 000-18zM12 7v5l3 3",
  db: "M12 3c4.4 0 8 1.3 8 3s-3.6 3-8 3-8-1.3-8-3 3.6-3 8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
  layers: "M12 3l9 5-9 5-9-5zM3 13l9 5 9-5M3 17l9 5 9-5",
  image: "M4 5h16v14H4zM4 15l4.5-4.5 4 4M12 13l3-3 5 5",
  pixel: "M4 4h4v4H4zM10 4h4v4h-4zM16 4h4v4h-4zM4 10h4v4H4zM10 10h4v4h-4zM16 10h4v4h-4zM4 16h4v4H4zM10 16h4v4h-4zM16 16h4v4h-4z",
};

export type IconName = keyof typeof PATHS | string;

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
  strokeWidth?: number;
}

export function Icon({ name, size = 16, className, strokeWidth = 1.7 }: IconProps) {
  const d = PATHS[name] ?? PATHS["pixel"];
  const filled = name === "play" || name === "pause" || name === "generate";
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={filled ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth={filled ? 0 : strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn("shrink-0", className)}
      aria-hidden
    >
      <path d={d} />
    </svg>
  );
}
