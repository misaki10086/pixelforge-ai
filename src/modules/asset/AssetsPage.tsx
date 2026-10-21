import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { AssetImage } from "@/components/AssetImage";
import { Icon } from "@/components/ui/Icon";
import { Badge, EmptyState, Panel, Spinner } from "@/components/ui/Panel";
import { AssetPreviewModal } from "./AssetPreviewModal";
import { assetApi } from "@/services";
import { filterAssets, useAssetStore, type GalleryMode } from "@/stores/assetStore";
import { useProjectStore } from "@/stores/projectStore";
import { useProviderStore } from "@/stores/providerStore";
import { useUiStore } from "@/stores/uiStore";
import { ASSET_TYPES, IMAGE_EXT_FILTER, assetTypeLabel } from "@/utils/constants";
import { cn } from "@/utils/cn";
import { formatDate } from "@/utils/format";

const MODES: Array<{ id: GalleryMode; icon: string; title: string }> = [
  { id: "grid", icon: "assets", title: "网格视图" },
  { id: "compact", icon: "pixel", title: "紧凑视图" },
  { id: "list", icon: "layers", title: "列表视图" },
];

export function AssetsPage() {
  const current = useProjectStore((s) => s.current);
  const assets = useAssetStore((s) => s.assets);
  const loading = useAssetStore((s) => s.loading);
  const search = useAssetStore((s) => s.search);
  const typeFilter = useAssetStore((s) => s.typeFilter);
  const providerFilter = useAssetStore((s) => s.providerFilter);
  const mode = useAssetStore((s) => s.mode);
  const sortDesc = useAssetStore((s) => s.sortDesc);
  const setFilters = useAssetStore((s) => s.setFilters);
  const load = useAssetStore((s) => s.load);
  const providers = useProviderStore((s) => s.providers);
  const pushToast = useUiStore((s) => s.pushToast);

  const [detailId, setDetailId] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [importType, setImportType] = useState<string>("character");
  const [dragOver, setDragOver] = useState(false);

  const visible = useMemo(
    () => filterAssets({ assets, search, typeFilter, providerFilter, sortDesc }),
    [assets, search, typeFilter, providerFilter, sortDesc]
  );

  // 后端生成完成事件已由 App 全局处理；这里补一次本地拉取兜底
  useEffect(() => {
    const un = listen<string>("assets-changed", () => {
      const pid = useProjectStore.getState().current?.id;
      if (pid) void load(pid);
    });
    return () => {
      void un.then((fn) => fn());
    };
  }, [load]);

  const usedProviders = useMemo(() => {
    const ids = new Set(assets.map((a) => a.providerId).filter(Boolean) as string[]);
    return providers.filter((p) => ids.has(p.id));
  }, [assets, providers]);

  const importImages = async (paths: string[]) => {
    if (!current) {
      pushToast("warn", "请先打开一个项目。");
      return;
    }
    setImporting(true);
    try {
      for (const p of paths) {
        await assetApi.import(current.id, p, importType);
      }
      await load(current.id);
      pushToast("success", `已导入 ${paths.length} 个文件（${assetTypeLabel(importType)}）。`);
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setImporting(false);
    }
  };

  const pickAndImport = async () => {
    const picked = await openFileDialog({
      multiple: true,
      title: "导入图片",
      filters: [{ name: "图片", extensions: IMAGE_EXT_FILTER }],
    });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    await importImages(paths);
  };

  if (!current) {
    return (
      <Panel className="m-2" scroll={false}>
        <EmptyState
          icon="projects"
          title="未打开项目"
          description="资产库按项目组织。请先在「项目」页创建或打开一个项目。"
        />
      </Panel>
    );
  }

  return (
    <div className="flex h-full flex-col gap-2 p-2">
      {/* 工具栏 */}
      <div className="flex shrink-0 flex-wrap items-center gap-1.5">
        <div className="relative">
          <input
            className="input w-[220px] pl-7"
            placeholder="搜索名称 / 提示词 / 标签…"
            value={search}
            onChange={(e) => setFilters({ search: e.target.value })}
          />
          <Icon name="search" size={12} className="absolute left-2 top-2 text-inkfaint" />
        </div>

        <select
          className="select"
          value={typeFilter}
          onChange={(e) => setFilters({ typeFilter: e.target.value as typeof typeFilter })}
          title="按类型过滤"
        >
          <option value="all">全部类型</option>
          {ASSET_TYPES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>

        <select
          className="select"
          value={providerFilter}
          onChange={(e) => setFilters({ providerFilter: e.target.value })}
          title="按服务商过滤"
        >
          <option value="all">全部服务商</option>
          {usedProviders.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>

        <button
          className="btn h-7 px-2"
          onClick={() => setFilters({ sortDesc: !sortDesc })}
          title={sortDesc ? "最新优先" : "最早优先"}
        >
          <Icon name="clock" size={12} />
          {sortDesc ? "最新" : "最早"}
        </button>

        <div className="ml-auto flex items-center gap-1">
          <span className="mr-1 text-[11px] text-inkfaint">
            {loading ? <Spinner size={12} /> : `${visible.length} / ${assets.length}`}
          </span>
          {MODES.map((m) => (
            <button
              key={m.id}
              className={cn("btn h-7 w-7 p-0", mode === m.id && "border-accent/70 text-accent")}
              onClick={() => setFilters({ mode: m.id })}
              title={m.title}
            >
              <Icon name={m.icon} size={13} />
            </button>
          ))}
          <select
            className="select h-7"
            value={importType}
            onChange={(e) => setImportType(e.target.value)}
            title="导入到哪个类型目录"
          >
            {ASSET_TYPES.map((t) => (
              <option key={t.value} value={t.value}>
                {t.label}
              </option>
            ))}
          </select>
          <button
            className={cn("btn h-7 px-2.5", dragOver && "border-accent text-accent")}
            disabled={importing}
            onClick={() => void pickAndImport()}
            onDragOver={(e) => {
              e.preventDefault();
              setDragOver(true);
            }}
            onDragLeave={() => setDragOver(false)}
            onDrop={(e) => {
              e.preventDefault();
              setDragOver(false);
              // 浏览器拖放不带本地路径，使用文件选择器导入
              void pickAndImport();
            }}
            title="导入本地图片到所选类型目录"
          >
            <Icon name="import" size={13} />
            导入图片
          </button>
        </div>
      </div>

      {/* 资产内容区 */}
      <Panel className="min-h-0 flex-1" bodyClassName="p-2">
        {visible.length === 0 ? (
          <EmptyState
            icon="assets"
            title={assets.length === 0 ? "项目还没有资产" : "没有符合条件的资产"}
            description={
              assets.length === 0
                ? "通过「生成」页生成资产，或点击右上角「导入图片」从本地导入。"
                : "尝试调整搜索关键词或过滤条件。"
            }
            actions={
              assets.length === 0 && (
                <button className="btn btn-primary" onClick={() => void pickAndImport()}>
                  <Icon name="import" size={13} />
                  导入本地图片
                </button>
              )
            }
          />
        ) : mode === "list" ? (
          <table className="w-full border-collapse text-[12px]">
            <thead>
              <tr className="border-b border-line text-left text-[10px] uppercase tracking-wider text-inkfaint">
                <th className="px-2 py-1.5 font-semibold">预览</th>
                <th className="px-2 py-1.5 font-semibold">名称</th>
                <th className="px-2 py-1.5 font-semibold">类型</th>
                <th className="px-2 py-1.5 font-semibold text-right">尺寸</th>
                <th className="px-2 py-1.5 font-semibold text-right">版本</th>
                <th className="px-2 py-1.5 font-semibold">创建时间</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((a) => (
                <tr key={a.id} className="cursor-pointer border-b border-line/50 hover:bg-hover" onClick={() => setDetailId(a.id)}>
                  <td className="px-2 py-1">
                    <AssetImage path={a.thumbnailPath ?? a.imagePath} alt={a.name} className="h-8 w-8" />
                  </td>
                  <td className="px-2 py-1 text-ink">{a.name}</td>
                  <td className="px-2 py-1 text-inkdim">{assetTypeLabel(a.type)}</td>
                  <td className="px-2 py-1 text-right font-mono text-inkdim">
                    {a.width}×{a.height}
                  </td>
                  <td className="px-2 py-1 text-right font-mono text-inkdim">v{a.currentVersion}</td>
                  <td className="px-2 py-1 text-inkdim">{formatDate(a.createdAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div
            className={cn(
              "flex flex-wrap gap-2",
              mode === "compact" && "gap-1.5"
            )}
          >
            {visible.map((a) => (
              <button
                key={a.id}
                className={cn(
                  "group text-left hover:border-accent/60 border border-transparent",
                  mode === "grid" ? "w-[140px]" : "w-[88px]"
                )}
                onClick={() => setDetailId(a.id)}
              >
                <AssetImage
                  path={a.thumbnailPath ?? a.imagePath}
                  alt={a.name}
                  className={mode === "grid" ? "h-[120px] w-full" : "h-[72px] w-full"}
                />
                <div className={cn("truncate text-[11px] text-ink", mode === "grid" ? "mt-1" : "mt-0.5")}>
                  {a.name}
                </div>
                {mode === "grid" && (
                  <div className="flex items-center justify-between text-[10px] text-inkfaint">
                    <span>{assetTypeLabel(a.type)}</span>
                    <Badge>v{a.currentVersion}</Badge>
                  </div>
                )}
              </button>
            ))}
          </div>
        )}
      </Panel>

      {detailId && <AssetPreviewModal assetId={detailId} onClose={() => setDetailId(null)} />}
    </div>
  );
}
