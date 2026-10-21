import { Icon } from "@/components/ui/Icon";
import { Badge, ProgressBar, Spinner } from "@/components/ui/Panel";
import { formatDuration } from "@/utils/format";
import { ANIM_TYPE_OPTIONS, assetTypeLabel } from "@/utils/constants";
import { activeTaskCount, useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { cn } from "@/utils/cn";
import type { GenerationTask, TaskStatus } from "@/types";

const STATUS_LABEL: Record<TaskStatus, string> = {
  queued: "排队中",
  running: "生成中",
  success: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

/** 任务种类（后端 kind 字段为英文标识） */
const KIND_LABEL: Record<string, string> = {
  image: "图片生成",
  generate: "图片生成",
  animation: "动画生成",
  frame: "单帧重绘",
  regenerate_frame: "单帧重绘",
};

function statusTone(status: TaskStatus): "default" | "ok" | "bad" | "warn" | "info" {
  switch (status) {
    case "success":
      return "ok";
    case "failed":
      return "bad";
    case "cancelled":
      return "warn";
    case "running":
      return "info";
    default:
      return "default";
  }
}

function TaskRow({ task }: { task: GenerationTask }) {
  const cancel = useTaskStore((s) => s.cancel);
  const retry = useTaskStore((s) => s.retry);
  const pushToast = useUiStore((s) => s.pushToast);
  const busy = task.status === "queued" || task.status === "running";

  // 兼容三种任务参数结构（图片 / 动画 / 单帧）
  const p = (task.params ?? {}) as Record<string, unknown>;
  const rawName = (p["assetName"] ?? p["name"] ?? p["kind"]) as string | undefined;
  const label = rawName && String(rawName).trim() ? String(rawName) : (KIND_LABEL[task.kind] ?? task.kind);
  const sub = p["assetType"]
    ? assetTypeLabel(String(p["assetType"]))
    : p["animType"]
      ? (ANIM_TYPE_OPTIONS.find((t) => t.value === p["animType"])?.label ?? String(p["animType"]))
      : "";

  const run = async (fn: () => Promise<void>) => {
    try {
      await fn();
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="border border-line bg-panel">
      <div className="flex items-center gap-2 px-2 pt-1.5">
        {busy ? <Spinner size={11} /> : <Icon name={task.status === "success" ? "check" : "alert"} size={12} className={task.status === "success" ? "text-ok" : "text-inkfaint"} />}
        <span className="truncate text-[11px] text-ink" title={task.prompt}>
          {label}
        </span>
        <Badge tone={statusTone(task.status)} className="ml-auto shrink-0">
          {STATUS_LABEL[task.status]}
        </Badge>
        {busy ? (
          <button
            className="btn btn-ghost h-5 w-5 p-0"
            title="取消任务"
            onClick={() => void run(() => cancel(task.id))}
          >
            <Icon name="close" size={11} />
          </button>
        ) : (
          task.status !== "success" && (
            <button
              className="btn btn-ghost h-5 w-5 p-0"
              title="重试"
              onClick={() => void run(() => retry(task.id))}
            >
              <Icon name="refresh" size={11} />
            </button>
          )
        )}
      </div>
      <div className="px-2 pb-1.5 pt-1">
        <ProgressBar value={task.progress} />
        <div className="mt-1 flex items-center justify-between text-[10px] text-inkfaint">
          <span className="truncate">
            {task.model || "—"} · {sub}
          </span>
          <span>{formatDuration(task.elapsedMs)}</span>
        </div>
        {task.error && (
          <div className="mt-1 border border-bad/40 bg-bad/10 px-1.5 py-1 text-[10px] leading-snug text-bad">
            {task.error}
          </div>
        )}
      </div>
    </div>
  );
}

export function TaskDock() {
  const tasks = useTaskStore((s) => s.tasks);
  const open = useTaskStore((s) => s.dockOpen);
  const toggleDock = useTaskStore((s) => s.toggleDock);
  const clearFinished = useTaskStore((s) => s.clearFinished);
  const pushToast = useUiStore((s) => s.pushToast);
  const active = activeTaskCount(tasks);
  if (!open) return null;

  return (
    <div className="fixed bottom-7 right-3 z-40 flex w-[300px] flex-col border border-linestrong bg-base/95 shadow-2xl backdrop-blur">
      <div className="flex h-7 items-center justify-between border-b border-line bg-elevated px-2">
        <span className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-inkdim">
          <Icon name="clock" size={12} />
          任务队列
          {active > 0 && <Badge tone="accent">{active} 进行中</Badge>}
        </span>
        <div className="flex items-center">
          <button
            className="btn btn-ghost h-5 px-1 text-[10px]"
            title="清除已完成任务"
            onClick={() => void clearFinished().catch((e) => pushToast("error", String(e)))}
          >
            清除
          </button>
          <button className="btn btn-ghost h-5 w-5 p-0" onClick={toggleDock} title="收起">
            <Icon name="close" size={12} />
          </button>
        </div>
      </div>
      <div className={cn("max-h-[40vh] space-y-1.5 overflow-y-auto p-1.5")}>
        {tasks.length === 0 ? (
          <div className="p-4 text-center text-[11px] text-inkfaint">暂无任务</div>
        ) : (
          tasks.slice(0, 12).map((t) => <TaskRow key={t.id} task={t} />)
        )}
      </div>
    </div>
  );
}
