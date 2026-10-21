import { Icon } from "@/components/ui/Icon";
import { projectApi } from "@/services";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { activeTaskCount, useTaskStore } from "@/stores/taskStore";
import { cn } from "@/utils/cn";

const VIEW_TITLES: Record<string, string> = {
  dashboard: "概览",
  generate: "生成",
  assets: "资产库",
  animation: "动画",
  editor: "编辑器",
  projects: "项目",
  styles: "风格库",
  settings: "设置",
};

export function TopBar() {
  const view = useUiStore((s) => s.view);
  const setView = useUiStore((s) => s.setView);
  const toggleSidebar = useUiStore((s) => s.toggleSidebar);
  const pushToast = useUiStore((s) => s.pushToast);
  const current = useProjectStore((s) => s.current);
  const tasks = useTaskStore((s) => s.tasks);
  const toggleDock = useTaskStore((s) => s.toggleDock);
  const active = activeTaskCount(tasks);

  const onSave = async () => {
    if (!current) {
      pushToast("warn", "当前没有打开的项目。");
      return;
    }
    try {
      await projectApi.persistJson(current.id);
      pushToast("success", "项目元数据已保存。");
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <header className="flex h-9 shrink-0 items-center gap-2 border-b border-line bg-panel px-2">
      <button className="btn btn-ghost h-6 w-6 p-0" onClick={toggleSidebar} title="收起/展开侧栏">
        <Icon name="panel" size={14} />
      </button>

      <div className="flex items-center gap-1.5 border-r border-line pr-3">
        <img src="/icon.png" alt="" className="h-4 w-4" style={{ imageRendering: "pixelated" }} />
        <span className="text-[12px] font-bold tracking-wide text-ink">PixelForge AI</span>
        <span className="text-[10px] text-inkfaint">v0.1.0</span>
      </div>

      <div className="flex items-center gap-2 text-[12px]">
        <span className="text-inkdim">{VIEW_TITLES[view] ?? view}</span>
        {current && (
          <>
            <span className="text-inkfaint">/</span>
            <span
              className="flex items-center gap-1 text-ink"
              title={current.path}
            >
              <Icon name="folder" size={12} className="text-inkfaint" />
              {current.name}
            </span>
          </>
        )}
      </div>

      <div className="ml-auto flex items-center gap-1.5">
        <button
          className="btn h-6 px-2 text-[11px]"
          onClick={onSave}
          title="保存项目元数据（Ctrl+S）"
        >
          <Icon name="check" size={12} />
          保存
        </button>
        <button
          className="btn btn-primary h-6 px-2.5 text-[11px]"
          onClick={() => setView("generate")}
          title="生成新资产"
        >
          <Icon name="generate" size={12} />
          生成
        </button>
        <button
          className={cn("btn h-6 w-7 p-0", active > 0 && "border-accent/60 text-accent")}
          onClick={toggleDock}
          title={`任务队列（${active} 个进行中）`}
        >
          <Icon name="clock" size={13} />
          {active > 0 && <span className="absolute -mt-5 ml-3.5 h-1.5 w-1.5 rounded-full bg-accent" />}
        </button>
      </div>
    </header>
  );
}
