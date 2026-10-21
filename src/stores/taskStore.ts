import { create } from "zustand";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { generationApi } from "@/services";
import type { GenerationTask } from "@/types";

interface TaskState {
  tasks: GenerationTask[];
  dockOpen: boolean;
  listening: boolean;
  init: () => Promise<void>;
  refresh: () => Promise<void>;
  toggleDock: () => void;
  cancel: (taskId: string) => Promise<void>;
  retry: (taskId: string) => Promise<void>;
  clearFinished: () => Promise<void>;
  upsert: (task: GenerationTask) => void;
}

function upsertTask(list: GenerationTask[], task: GenerationTask): GenerationTask[] {
  const idx = list.findIndex((t) => t.id === task.id);
  if (idx === -1) return [task, ...list].slice(0, 100);
  const next = [...list];
  next[idx] = task;
  return next;
}

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  dockOpen: false,
  listening: false,

  init: async () => {
    if (get().listening) return;
    set({ listening: true });
    const unlisten: UnlistenFn = await listen<GenerationTask>("task-updated", (event) => {
      get().upsert(event.payload);
      if (event.payload.status === "running") {
        set({ dockOpen: true });
      }
    });
    window.addEventListener("unload", () => unlisten());
    await get().refresh();
  },

  refresh: async () => {
    const tasks = await generationApi.listTasks(50);
    set({ tasks });
  },

  toggleDock: () => set((s) => ({ dockOpen: !s.dockOpen })),

  upsert: (task) => set((s) => ({ tasks: upsertTask(s.tasks, task) })),

  cancel: async (taskId) => {
    const task = await generationApi.cancel(taskId);
    get().upsert(task);
  },

  retry: async (taskId) => {
    const task = await generationApi.retry(taskId);
    get().upsert(task);
  },

  clearFinished: async () => {
    await generationApi.clearFinished();
    await get().refresh();
  },
}));

export function activeTaskCount(tasks: GenerationTask[]): number {
  return tasks.filter((t) => t.status === "queued" || t.status === "running").length;
}
