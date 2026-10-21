import { useCallback, useEffect, useState } from "react";
import { AssetImage } from "@/components/AssetImage";
import { Icon } from "@/components/ui/Icon";
import { Badge, EmptyState, Panel, Spinner } from "@/components/ui/Panel";
import { assetTypeLabel } from "@/utils/constants";
import { formatDuration, formatRelative, truncate } from "@/utils/format";
import { assetApi } from "@/services";
import { useAssetStore } from "@/stores/assetStore";
import { useProjectStore } from "@/stores/projectStore";
import { useProviderStore } from "@/stores/providerStore";
import { useUiStore } from "@/stores/uiStore";
import type { DashboardStats, TaskStatus } from "@/types";
import { PROVIDER_KIND_LABELS } from "@/utils/constants";

/** 任务状态中文名（数据库与后端 status 为英文枚举） */
const STATUS_LABEL: Record<TaskStatus, string> = {
  queued: "排队中",
  running: "生成中",
  success: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

/** 任务种类（后端 kind 为英文标识） */
const TASK_KIND_LABEL: Record<string, string> = {
  image: "图片生成",
  generate: "图片生成",
  animation: "动画生成",
  frame: "单帧重绘",
  regenerate_frame: "单帧重绘",
};

function StatBlock({ label, value, tone }: { label: string; value: string | number; tone?: string }) {
  return (
    <div className="panel flex min-w-[110px] flex-1 flex-col gap-0.5 px-3 py-2">
      <span className="text-[10px] uppercase tracking-wider text-inkfaint">{label}</span>
      <span className={`font-mono text-[20px] leading-none ${tone ?? "text-ink"}`}>{value}</span>
    </div>
  );
}

export function DashboardPage() {
  const current = useProjectStore((s) => s.current);
  const projects = useProjectStore((s) => s.projects);
  const providers = useProviderStore((s) => s.providers);
  const setView = useUiStore((s) => s.setView);
  const openDetail = useAssetStore((s) => s.openDetail);
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setStats(await assetApi.dashboardStats(current?.id ?? null));
    } catch {
      setStats(null);
    } finally {
      setLoading(false);
    }
  }, [current?.id]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const hasProvider = providers.length > 0;

  return (
    <div className="flex h-full gap-2 overflow-y-auto p-2">
      {/* 左列：项目 + 概览 */}
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div className="flex flex-wrap gap-2">
          <StatBlock label="资产总数" value={stats?.totalAssets ?? 0} />
          <StatBlock label="角色" value={stats?.byType.find(([t]) => t === "character")?.[1] ?? 0} />
          <StatBlock label="动画" value={stats?.totalAnimations ?? 0} />
          <StatBlock
            label="任务"
            value={`${stats?.totalTasks ?? 0}${stats?.failedTasks ? ` / ${stats.failedTasks} 失败` : ""}`}
            tone={stats?.failedTasks ? "text-bad" : undefined}
          />
          <StatBlock label="最后打开" value={current ? formatRelative(current.lastOpenedAt) : "—"} />
        </div>

        {current ? (
          <Panel
            title={
              <>
                <Icon name="generate" size={12} />
                最近生成的资产
              </>
            }
            actions={
              <button className="btn btn-ghost h-5 px-1.5 text-[10px]" onClick={() => setView("assets")}>
                查看全部
              </button>
            }
            className="min-h-[180px]"
            bodyClassName="p-2"
          >
            {stats && stats.recentAssets.length > 0 ? (
              <div className="flex flex-wrap gap-2">
                {stats.recentAssets.map((a) => (
                  <button
                    key={a.id}
                    className="group w-[108px] text-left"
                    onClick={() => void openDetail(a.id).catch(() => undefined)}
                    title={`${a.name} · ${assetTypeLabel(a.type)}`}
                  >
                    <AssetImage
                      path={a.thumbnailPath ?? a.imagePath}
                      alt={a.name}
                      className="h-[80px] w-full transition-colors group-hover:border-accent/60"
                    />
                    <div className="mt-1 truncate text-[11px] text-ink">{a.name}</div>
                    <div className="text-[10px] text-inkfaint">
                      {assetTypeLabel(a.type)} · {a.width}×{a.height}
                    </div>
                  </button>
                ))}
              </div>
            ) : (
              <EmptyState
                icon="generate"
                title="还没有资产"
                description="在「生成」页使用 AI 生成第一个角色 / 道具 / 场景，或从「资产库」页导入本地图片。"
                actions={
                  <>
                    <button className="btn btn-primary" onClick={() => setView("generate")}>
                      <Icon name="generate" size={13} />
                      开始生成
                    </button>
                    <button className="btn" onClick={() => setView("assets")}>
                      导入图片
                    </button>
                  </>
                }
              />
            )}
          </Panel>
        ) : (
          <Panel title={<>开始使用</>} className="min-h-[220px]" scroll={false}>
            <EmptyState
              icon="projects"
              title="尚未打开任何项目"
              description="创建一个项目来管理你的游戏资产。项目数据保存在本地磁盘，可随时切换。"
              actions={
                <>
                  <button className="btn btn-primary" onClick={() => setView("projects")}>
                    <Icon name="plus" size={13} />
                    新建项目
                  </button>
                  <button className="btn" onClick={() => setView("projects")}>
                    打开项目
                  </button>
                </>
              }
            />
          </Panel>
        )}

        {loading && (
          <div className="flex items-center justify-center gap-2 py-1 text-[11px] text-inkfaint">
            <Spinner size={12} /> 加载统计中…
          </div>
        )}

        {/* 最近项目 */}
        <Panel title={<>最近项目</>} className="min-h-[120px]" bodyClassName="p-1.5">
          {projects.length === 0 ? (
            <div className="p-4 text-center text-[11px] text-inkfaint">暂无项目</div>
          ) : (
            <div className="grid grid-cols-2 gap-1.5 lg:grid-cols-3">
              {projects.slice(0, 6).map((p) => (
                <button
                  key={p.id}
                  className="flex flex-col gap-0.5 border border-line bg-elevated/40 px-2.5 py-2 text-left hover:border-linestrong"
                  onClick={() => void useProjectStore.getState().openProject(p.id).catch(() => undefined)}
                  title={p.path}
                >
                  <span className="flex items-center gap-1.5 text-[12px] text-ink">
                    <Icon name="folder" size={12} className="text-accent" />
                    {p.name}
                    {current?.id === p.id && <Badge tone="accent">当前</Badge>}
                  </span>
                  <span className="truncate text-[10px] text-inkfaint">{p.path}</span>
                  <span className="text-[10px] text-inkdim">
                    {p.assetCount} 资产 · {formatRelative(p.lastOpenedAt)}
                  </span>
                </button>
              ))}
            </div>
          )}
        </Panel>
      </div>

      {/* 右列：Provider 状态 + 最近任务 */}
      <div className="flex w-[300px] shrink-0 flex-col gap-2">
        <Panel
          title={
            <>
              <Icon name="db" size={12} />
              AI 服务商状态
            </>
          }
          actions={
            <button className="btn btn-ghost h-5 px-1.5 text-[10px]" onClick={() => setView("settings")}>
              配置
            </button>
          }
          bodyClassName="p-2"
        >
          {!hasProvider ? (
            <div className="space-y-2 text-[11px] leading-relaxed text-inkdim">
              <div className="flex items-center gap-2 border border-warn/40 bg-warn/10 px-2 py-1.5 text-warn">
                <Icon name="alert" size={13} />
                尚未配置任何 AI 服务商
              </div>
              生成资产前需要先在设置中添加服务商（支持 OpenAI、Gemini 与自定义 OpenAI 兼容接口），API
              Key 保存在系统凭据管理器中。
            </div>
          ) : (
            <div className="space-y-1.5">
              {providers.map((p) => (
                <div key={p.id} className="border border-line bg-elevated/40 px-2 py-1.5">
                  <div className="flex items-center justify-between">
                    <span className="text-[12px] text-ink">{p.name}</span>
                    {p.hasKey ? <Badge tone="ok">已配置</Badge> : <Badge tone="bad">缺少 Key</Badge>}
                  </div>
                  <div className="mt-0.5 text-[10px] text-inkfaint">
                    {PROVIDER_KIND_LABELS[p.kind] ?? p.kind} · {truncate(p.imageModel || p.model || "未设置模型", 32)}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>

        <Panel
          title={
            <>
              <Icon name="clock" size={12} />
              最近任务
            </>
          }
          className="flex-1"
          bodyClassName="p-1.5 space-y-1"
        >
          {!stats || stats.recentTasks.length === 0 ? (
            <div className="p-4 text-center text-[11px] text-inkfaint">暂无任务记录</div>
          ) : (
            stats.recentTasks.map((t) => {
              const p = (t.params ?? {}) as Record<string, unknown>;
              const label = String(p["assetName"] ?? p["name"] ?? TASK_KIND_LABEL[t.kind] ?? t.kind);
              return (
              <div key={t.id} className="border border-line bg-elevated/40 px-2 py-1.5">
                <div className="flex items-center justify-between text-[11px]">
                  <span className="truncate text-ink">{label}</span>
                  <Badge
                    tone={
                      t.status === "success"
                        ? "ok"
                        : t.status === "failed"
                          ? "bad"
                          : t.status === "cancelled"
                            ? "warn"
                            : "info"
                    }
                  >
                    {STATUS_LABEL[t.status] ?? t.status}
                  </Badge>
                </div>
                <div className="mt-0.5 flex justify-between text-[10px] text-inkfaint">
                  <span>{formatRelative(t.createdAt)}</span>
                  <span>{formatDuration(t.elapsedMs)}</span>
                </div>
                {t.error && <div className="mt-1 text-[10px] leading-snug text-bad">{truncate(t.error, 120)}</div>}
              </div>
              );
            })
          )}
        </Panel>
      </div>
    </div>
  );
}
