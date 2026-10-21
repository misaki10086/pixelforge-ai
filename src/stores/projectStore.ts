import { create } from "zustand";
import { projectApi } from "@/services";
import type { Project } from "@/types";

interface ProjectState {
  projects: Project[];
  current: Project | null;
  loading: boolean;
  loaded: boolean;
  loadProjects: () => Promise<void>;
  restoreLast: () => Promise<void>;
  setCurrent: (project: Project | null) => void;
  openProject: (id: string) => Promise<Project>;
  refreshCurrent: () => Promise<void>;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projects: [],
  current: null,
  loading: false,
  loaded: false,

  loadProjects: async () => {
    set({ loading: true });
    try {
      const projects = await projectApi.list();
      set({ projects, loaded: true });
    } finally {
      set({ loading: false });
    }
  },

  restoreLast: async () => {
    const { projects, current } = get();
    if (current || projects.length === 0) return;
    const last = projects[0];
    if (!last) return;
    try {
      const opened = await projectApi.open(last.id);
      set((s) => ({
        current: opened,
        projects: s.projects.map((p) => (p.id === opened.id ? opened : p)),
      }));
    } catch {
      // 项目目录可能已被移动，静默失败，用户可手动打开
    }
  },

  setCurrent: (project) => set({ current: project }),

  openProject: async (id) => {
    const opened = await projectApi.open(id);
    set((s) => ({
      current: opened,
      projects: s.projects.some((p) => p.id === opened.id)
        ? s.projects.map((p) => (p.id === opened.id ? opened : p))
        : [opened, ...s.projects],
    }));
    return opened;
  },

  refreshCurrent: async () => {
    const cur = get().current;
    if (!cur) return;
    try {
      const fresh = await projectApi.open(cur.id);
      set((s) => ({
        current: fresh,
        projects: s.projects.map((p) => (p.id === fresh.id ? fresh : p)),
      }));
    } catch {
      /* 忽略刷新失败 */
    }
  },
}));
