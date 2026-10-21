import { Icon } from "@/components/ui/Icon";
import { useProjectStore } from "@/stores/projectStore";
import { useProviderStore } from "@/stores/providerStore";
import { activeTaskCount, useTaskStore } from "@/stores/taskStore";

export function StatusBar() {
  const current = useProjectStore((s) => s.current);
  const providers = useProviderStore((s) => s.providers);
  const tasks = useTaskStore((s) => s.tasks);
  const toggleDock = useTaskStore((s) => s.toggleDock);
  const active = activeTaskCount(tasks);
  const withKey = providers.filter((p) => p.hasKey).length;

  return (
    <footer className="flex h-6 shrink-0 items-center gap-4 border-t border-line bg-panel px-3 text-[10px] text-inkfaint">
      <span className="flex items-center gap-1">
        <Icon name="folder" size={10} />
        {current ? current.name : "未打开项目"}
      </span>
      <span className="flex items-center gap-1">
        <Icon name="assets" size={10} />
        {current ? `${current.assetCount} 资产` : "—"}
      </span>
      <span className="flex items-center gap-1">
        <Icon name="db" size={10} />
        {withKey > 0 ? `${withKey}/${providers.length} 个服务商可用` : "无可用服务商"}
      </span>
      <button
        className="ml-auto flex items-center gap-1 hover:text-ink"
        onClick={toggleDock}
        title="查看任务队列"
      >
        <Icon name="clock" size={10} />
        {active > 0 ? `${active} 个任务进行中` : "任务队列空闲"}
      </button>
    </footer>
  );
}
