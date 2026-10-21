import type { ReactNode } from "react";
import { cn } from "@/utils/cn";
import { Icon } from "./Icon";

interface PanelProps {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
  /** 面板体不使用滚动（外层自行控制） */
  scroll?: boolean;
}

/** 标准工作区面板：带标题栏与操作区，直角边框，高信息密度 */
export function Panel({ title, actions, children, className, bodyClassName, scroll = true }: PanelProps) {
  return (
    <div className={cn("panel flex min-h-0 flex-col", className)}>
      {(title || actions) && (
        <div className="flex h-8 shrink-0 items-center justify-between border-b border-line bg-elevated/60 px-2.5">
          <div className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-inkdim">
            {title}
          </div>
          <div className="flex items-center gap-1">{actions}</div>
        </div>
      )}
      <div className={cn("min-h-0 flex-1", scroll && "overflow-y-auto", bodyClassName)}>{children}</div>
    </div>
  );
}

export function Field({
  label,
  children,
  hint,
  className,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
  className?: string;
}) {
  return (
    <label className={cn("block", className)}>
      <span className="mb-1 flex items-center justify-between text-[11px] text-inkdim">
        <span>{label}</span>
        {hint && <span className="text-inkfaint">{hint}</span>}
      </span>
      {children}
    </label>
  );
}

export function Badge({
  children,
  tone = "default",
  className,
}: {
  children: ReactNode;
  tone?: "default" | "ok" | "bad" | "warn" | "info" | "accent";
  className?: string;
}) {
  const tones: Record<string, string> = {
    default: "border-line text-inkdim",
    ok: "border-ok/50 text-ok",
    bad: "border-bad/50 text-bad",
    warn: "border-warn/50 text-warn",
    info: "border-info/50 text-info",
    accent: "border-accent/50 text-accent",
  };
  return (
    <span
      className={cn(
        "inline-flex h-[18px] items-center border px-1.5 text-[10px] uppercase tracking-wide",
        tones[tone],
        className
      )}
    >
      {children}
    </span>
  );
}

export function EmptyState({
  icon = "folder",
  title,
  description,
  actions,
}: {
  icon?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
      <div className="flex h-12 w-12 items-center justify-center border border-line bg-elevated text-inkfaint">
        <Icon name={icon} size={22} />
      </div>
      <div className="text-[13px] font-medium text-inkdim">{title}</div>
      {description && <div className="max-w-sm text-[11px] leading-relaxed text-inkfaint">{description}</div>}
      {actions && <div className="mt-2 flex items-center gap-2">{actions}</div>}
    </div>
  );
}

export function Spinner({ size = 14, className }: { size?: number; className?: string }) {
  return (
    <span
      className={cn("inline-block animate-spin border-2 border-line border-t-accent rounded-full", className)}
      style={{ width: size, height: size }}
    />
  );
}

export function ProgressBar({ value, className }: { value: number; className?: string }) {
  const pct = Math.max(0, Math.min(1, value));
  return (
    <div className={cn("h-1.5 w-full border border-line bg-input", className)}>
      <div
        className="h-full bg-accent transition-[width] duration-300"
        style={{ width: `${Math.round(pct * 100)}%` }}
      />
    </div>
  );
}

export function ComingSoon({ feature, items }: { feature: string; items: string[] }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-10 text-center">
      <div className="flex items-center gap-2 border border-warn/40 bg-warn/10 px-3 py-1 text-[11px] uppercase tracking-widest text-warn">
        <Icon name="clock" size={13} />
        即将上线
      </div>
      <div className="text-[15px] font-semibold text-ink">{feature}</div>
      <p className="max-w-md text-[11px] leading-relaxed text-inkfaint">
        该模块属于后续开发阶段，当前版本尚未启用。以下为规划中的能力：
      </p>
      <ul className="w-full max-w-md space-y-1 text-left">
        {items.map((it) => (
          <li key={it} className="flex items-center gap-2 border border-line bg-panel px-2.5 py-1.5 text-[12px] text-inkdim">
            <Icon name="check" size={12} className="text-inkfaint" />
            {it}
          </li>
        ))}
      </ul>
    </div>
  );
}
