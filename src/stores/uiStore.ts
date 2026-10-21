import { create } from "zustand";
import { cn } from "@/utils/cn";

export type ToastKind = "info" | "success" | "error" | "warn";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

interface UiState {
  view: string;
  sidebarCollapsed: boolean;
  toasts: Toast[];
  setView: (view: string) => void;
  toggleSidebar: () => void;
  pushToast: (kind: ToastKind, message: string) => void;
  dismissToast: (id: number) => void;
}

let toastSeq = 1;

export const useUiStore = create<UiState>((set) => ({
  view: "dashboard",
  sidebarCollapsed: false,
  toasts: [],
  setView: (view) => set({ view }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  pushToast: (kind, message) => {
    const id = toastSeq++;
    set((s) => ({ toasts: [...s.toasts.slice(-4), { id, kind, message }] }));
    window.setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, kind === "error" ? 6000 : 3500);
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

export const toastClass: Record<ToastKind, string> = {
  info: "border-line text-ink",
  success: "border-ok/60 text-ok",
  warn: "border-warn/60 text-warn",
  error: "border-bad/70 text-bad bg-bad/5",
};

export const toastIcon: Record<ToastKind, string> = {
  info: "ℹ",
  success: "✓",
  warn: "⚠",
  error: "✕",
};

export function toastClasses(kind: ToastKind): string {
  return cn(toastClass[kind]);
}
