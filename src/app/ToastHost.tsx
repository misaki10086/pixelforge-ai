import { Icon } from "@/components/ui/Icon";
import { useUiStore, toastIcon } from "@/stores/uiStore";
import { cn } from "@/utils/cn";
import { createPortal } from "react-dom";

export function ToastHost() {
  const toasts = useUiStore((s) => s.toasts);
  const dismiss = useUiStore((s) => s.dismissToast);
  if (toasts.length === 0) return null;

  return createPortal(
    <div className="pointer-events-none fixed bottom-9 left-1/2 z-[60] flex -translate-x-1/2 flex-col items-center gap-1.5">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={cn(
            "pointer-events-auto flex max-w-lg items-center gap-2 border bg-elevated px-3 py-1.5 text-[12px] shadow-lg",
            t.kind === "error" && "border-bad/60 text-bad",
            t.kind === "success" && "border-ok/50 text-ok",
            t.kind === "warn" && "border-warn/50 text-warn",
            t.kind === "info" && "border-line text-ink"
          )}
        >
          <span className="text-[11px]">{toastIcon[t.kind]}</span>
          <span className="text-ink">{t.message}</span>
          <button className="text-inkfaint hover:text-ink" onClick={() => dismiss(t.id)}>
            <Icon name="close" size={11} />
          </button>
        </div>
      ))}
    </div>,
    document.body
  );
}
