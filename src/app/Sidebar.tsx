import { Icon } from "@/components/ui/Icon";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { activeTaskCount, useTaskStore } from "@/stores/taskStore";
import { cn } from "@/utils/cn";
import type { ViewId } from "@/types";

const GROUPS: Array<{ title: string; items: Array<{ id: ViewId; icon: string; label: string }> }> = [
  {
    title: "创作",
    items: [
      { id: "dashboard", icon: "dashboard", label: "概览" },
      { id: "generate", icon: "generate", label: "生成" },
      { id: "assets", icon: "assets", label: "资产库" },
      { id: "animation", icon: "animation", label: "动画" },
      { id: "editor", icon: "editor", label: "编辑器" },
    ],
  },
  {
    title: "管理",
    items: [
      { id: "projects", icon: "projects", label: "项目" },
      { id: "styles", icon: "styles", label: "风格库" },
      { id: "settings", icon: "settings", label: "设置" },
    ],
  },
];

const COMING_SOON_VIEWS: ViewId[] = ["editor", "styles"];

export function Sidebar() {
  const view = useUiStore((s) => s.view);
  const setView = useUiStore((s) => s.setView);
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const current = useProjectStore((s) => s.current);
  const tasks = useTaskStore((s) => s.tasks);
  const active = activeTaskCount(tasks);

  return (
    <nav
      className={cn(
        "flex shrink-0 flex-col border-r border-line bg-panel",
        collapsed ? "w-11" : "w-[132px]"
      )}
    >
      {GROUPS.map((group, gi) => (
        <div key={group.title} className={cn(gi > 0 && "mt-4")}>
          {!collapsed && (
            <div className="px-2.5 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-widest text-inkfaint">
              {group.title}
            </div>
          )}
          {group.items.map((item) => {
            const isSoon = COMING_SOON_VIEWS.includes(item.id);
            const isActive = view === item.id;
            return (
              <button
                key={item.id}
                onClick={() => setView(item.id)}
                title={collapsed ? item.label : isSoon ? `${item.label}（即将上线）` : item.label}
                className={cn(
                  "group relative flex h-8 w-full items-center gap-2.5 text-[12px]",
                  collapsed ? "justify-center px-0" : "px-3",
                  isActive
                    ? "bg-selbg text-accent"
                    : "text-inkdim hover:bg-hover hover:text-ink",
                  isSoon && !isActive && "opacity-55"
                )}
              >
                {isActive && <span className="absolute left-0 top-0 h-full w-0.5 bg-accent" />}
                <Icon name={item.icon} size={15} />
                {!collapsed && (
                  <>
                    <span>{item.label}</span>
                    {isSoon && (
                      <span className="ml-auto text-[8px] tracking-wider text-inkfaint">即将</span>
                    )}
                  </>
                )}
                {collapsed && item.id === "generate" && active > 0 && (
                  <span className="absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-accent" />
                )}
              </button>
            );
          })}
        </div>
      ))}

      <div className="mt-auto border-t border-line p-2">
        {!collapsed && current && (
          <div className="px-1 text-[10px] leading-tight text-inkfaint">
            <div className="truncate text-inkdim" title={current.name}>
              {current.name}
            </div>
            <div className="truncate">{current.assetCount} 资产</div>
          </div>
        )}
      </div>
    </nav>
  );
}
