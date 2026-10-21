import { useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Icon } from "./Icon";

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: number;
}

export function Modal({ title, onClose, children, footer, width = 460 }: ModalProps) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/55"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="panel shadow-2xl" style={{ width, maxWidth: "94vw" }}>
        <div className="flex h-9 items-center justify-between border-b border-line bg-elevated px-3">
          <span className="text-[12px] font-semibold tracking-wide">{title}</span>
          <button className="btn btn-ghost h-6 w-6 p-0" onClick={onClose} title="关闭 (Esc)">
            <Icon name="close" size={13} />
          </button>
        </div>
        <div className="max-h-[70vh] overflow-y-auto p-3">{children}</div>
        {footer && (
          <div className="flex items-center justify-end gap-2 border-t border-line px-3 py-2">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body
  );
}
