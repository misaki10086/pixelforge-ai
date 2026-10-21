import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { AppShell } from "@/app/AppShell";
import { projectApi } from "@/services";
import { useAnimationStore } from "@/stores/animationStore";
import { useProjectStore } from "@/stores/projectStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { useAssetStore } from "@/stores/assetStore";
import { useProviderStore } from "@/stores/providerStore";

export default function App() {
  useEffect(() => {
    void (async () => {
      await useSettingsStore.getState().load();
      await useProviderStore.getState().load();
      await useProjectStore.getState().loadProjects();
      await useProjectStore.getState().restoreLast();
      await useTaskStore.getState().init();
    })();
  }, []);

  // 项目切换时刷新资产与动画列表；生成事件触发对应刷新
  useEffect(() => {
    const unsubProject = useProjectStore.subscribe((s) => {
      const pid = s.current?.id ?? null;
      if (pid && useAssetStore.getState().loadedForProject !== pid) {
        void useAssetStore.getState().load(pid);
        void useAnimationStore.getState().load(pid);
      } else if (!pid) {
        useAssetStore.getState().clear();
        useAnimationStore.getState().clear();
      }
    });

    const unlistenAssets = listen<unknown>("assets-changed", () => {
      const pid = useProjectStore.getState().current?.id;
      if (pid) void useAssetStore.getState().load(pid);
    });

    // 动画任务完成事件（payload 为动画 id 或 true）
    const unlistenAnims = listen<unknown>("animations-changed", () => {
      void useAnimationStore.getState().refreshCurrent();
    });

    return () => {
      unsubProject();
      void unlistenAssets.then((fn) => fn());
      void unlistenAnims.then((fn) => fn());
    };
  }, []);

  // 全局快捷键
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod) return;
      const target = e.target as HTMLElement | null;
      const inInput = target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
      if (e.key.toLowerCase() === "s") {
        e.preventDefault();
        const current = useProjectStore.getState().current;
        if (current) {
          void projectApi
            .persistJson(current.id)
            .then(() => useUiStore.getState().pushToast("success", "项目元数据已保存。"))
            .catch((err) => useUiStore.getState().pushToast("error", String(err)));
        }
      }
      if (inInput) return;
      // Ctrl+Z / Ctrl+Shift+Z 等编辑器快捷键将在 Editor 模块启用后接入
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return <AppShell />;
}
