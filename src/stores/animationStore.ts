import { create } from "zustand";
import { animationApi } from "@/services";
import type { AnimationDetail, AnimationView } from "@/types";

interface AnimationState {
  animations: AnimationView[];
  detail: AnimationDetail | null;
  loading: boolean;
  loadedForProject: string | null;
  load: (projectId: string) => Promise<void>;
  open: (animationId: string) => Promise<void>;
  /** 任务成功后刷新当前动画与列表 */
  refreshCurrent: () => Promise<void>;
  clear: () => void;
}

export const useAnimationStore = create<AnimationState>((set, get) => ({
  animations: [],
  detail: null,
  loading: false,
  loadedForProject: null,

  load: async (projectId) => {
    set({ loading: true });
    try {
      const animations = await animationApi.list(projectId);
      set({ animations, loadedForProject: projectId });
      // 当前打开的动画被删除时清空详情
      const cur = get().detail?.animation.id;
      if (cur && !animations.some((a) => a.id === cur)) {
        set({ detail: null });
      }
    } finally {
      set({ loading: false });
    }
  },

  open: async (animationId) => {
    const detail = await animationApi.detail(animationId);
    set({ detail });
  },

  refreshCurrent: async () => {
    const cur = get().detail?.animation.id;
    if (cur) {
      try {
        set({ detail: await animationApi.detail(cur) });
      } catch {
        set({ detail: null });
      }
    }
    const pid = get().loadedForProject;
    if (pid) await get().load(pid);
  },

  clear: () => set({ animations: [], detail: null, loadedForProject: null }),
}));
