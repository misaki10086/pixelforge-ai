import { useCallback, useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { AssetImage } from "@/components/AssetImage";
import { Icon } from "@/components/ui/Icon";
import { Badge, Field } from "@/components/ui/Panel";
import { Modal } from "@/components/ui/Modal";
import { assetApi, animationApi, imageApi, settingsApi } from "@/services";
import { useAnimationStore } from "@/stores/animationStore";
import { useAssetStore } from "@/stores/assetStore";
import { useProviderStore } from "@/stores/providerStore";
import { useUiStore } from "@/stores/uiStore";
import { assetTypeLabel, PRESET_PALETTES } from "@/utils/constants";
import { formatDate } from "@/utils/format";
import { cn } from "@/utils/cn";
import type { AssetDetail, AssetVersion } from "@/types";

export function AssetPreviewModal({ assetId, onClose }: { assetId: string; onClose: () => void }) {
  const pushToast = useUiStore((s) => s.pushToast);
  const load = useAssetStore((s) => s.load);
  const loadedFor = useAssetStore((s) => s.loadedForProject);
  const [detail, setDetail] = useState<AssetDetail | null>(null);
  const [activeVersion, setActiveVersion] = useState<AssetVersion | null>(null);
  const [compareWith, setCompareWith] = useState<AssetVersion | null>(null);
  const [name, setName] = useState("");
  const [tags, setTags] = useState("");
  const [busy, setBusy] = useState(false);
  // 颜色 / 风格处理
  const [palette, setPalette] = useState<string[]>([]);
  const [paletteDither, setPaletteDither] = useState(false);
  const [presetPaletteId, setPresetPaletteId] = useState("");
  const [pxResolution, setPxResolution] = useState(0);
  const [pxPalette, setPxPalette] = useState(0);
  const [pxDither, setPxDither] = useState(false);
  // 拆分为动画
  const [splitCols, setSplitCols] = useState(4);
  const [splitRows, setSplitRows] = useState(1);
  const [splitFps, setSplitFps] = useState(8);
  const [splitAuto, setSplitAuto] = useState(true);

  const providers = useProviderStore((s) => s.providers);

  const reload = useCallback(async () => {
    const d = await assetApi.detail(assetId);
    setDetail(d);
    setName(d.asset.name);
    setTags(d.asset.tags.join(", "));
    setActiveVersion(d.versions.find((v) => v.version === d.asset.currentVersion) ?? d.versions[d.versions.length - 1] ?? null);
  }, [assetId]);

  useEffect(() => {
    void reload().catch((e) => pushToast("error", e instanceof Error ? e.message : String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reload]);

  const run = async (fn: () => Promise<unknown>, okMsg?: string) => {
    setBusy(true);
    try {
      await fn();
      await reload();
      if (loadedFor) await load(loadedFor);
      if (okMsg) pushToast("success", okMsg);
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const saveMeta = () => {
    if (!detail) return;
    const tagList = tags.split(",").map((t) => t.trim()).filter(Boolean);
    void run(() => assetApi.update(detail.asset.id, name, tagList), "资产信息已更新。");
  };

  const exportPng = async () => {
    if (!detail) return;
    const picked = await openDialog({ directory: true, title: "选择导出目录（PNG + JSON）" });
    if (typeof picked !== "string") return;
    await run(() => assetApi.export(detail.asset.id, picked, "generic"), "已导出 PNG 与 JSON 元数据。");
    void settingsApi.reveal(picked).catch(() => undefined);
  };

  const exportForEngine = async (engine: "godot" | "unity") => {
    if (!detail) return;
    const engineName = engine === "godot" ? "Godot" : "Unity";
    const picked = await openDialog({ directory: true, title: `选择导出目录（${engineName}）` });
    if (typeof picked !== "string") return;
    await run(() => assetApi.export(detail.asset.id, picked, engine), `已导出到 ${engineName} 目录。`);
  };

  const providerName = (id: string | null) =>
    id ? (providers.find((p) => p.id === id)?.name ?? id) : "—";

  const folderOf = (p: string) => p.replace(/[\\/][^\\/]+$/, "");

  const extractPalette = async (count: number) => {
    setBusy(true);
    try {
      setPalette(await imageApi.extractPalette(assetId, count));
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const splitToAnimation = async () => {
    setBusy(true);
    try {
      const anim = await animationApi.splitSpriteSheet(assetId, splitCols, splitRows, splitFps, splitAuto);
      await useAnimationStore.getState().load(detail?.asset.projectId ?? "");
      pushToast("success", `已创建动画「${anim.name}」（${anim.frameCount} 帧），可在「动画」页查看。`);
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  if (!detail) {
    return (
      <Modal title="资产详情" onClose={onClose} width={860}>
        <div className="p-6 text-center text-[12px] text-inkfaint">加载中…</div>
      </Modal>
    );
  }

  const { asset } = detail;
  const versions = detail.versions;
  const promptText = activeVersion?.prompt || asset.prompt;

  return (
    <Modal
      title={`资产 · ${asset.name}`}
      onClose={onClose}
      width={880}
      footer={
        <>
          <button className="btn" disabled={busy} onClick={() => void exportForEngine("godot")}>
            <Icon name="export" size={12} />
            Godot 基础导出
          </button>
          <button className="btn" disabled={busy} onClick={() => void exportForEngine("unity")}>
            <Icon name="export" size={12} />
            Unity 基础导出
          </button>
          <button className="btn btn-primary" disabled={busy || !activeVersion} onClick={() => void exportPng()}>
            <Icon name="export" size={12} />
            导出 PNG + JSON
          </button>
        </>
      }
    >
      <div className="flex gap-3">
        {/* 左：大图 */}
        <div className="flex w-[300px] shrink-0 flex-col gap-2">
          <AssetImage
            path={(compareWith ?? activeVersion)?.imagePath ?? asset.imagePath}
            alt={asset.name}
            className="h-[300px] w-full"
          />
          {compareWith && activeVersion && (
            <div className="grid grid-cols-2 gap-2">
              <div>
                <div className="mb-0.5 text-[10px] text-inkfaint">A · v{compareWith.version}</div>
                <AssetImage path={compareWith.imagePath} alt="compare-a" className="h-[110px] w-full" />
              </div>
              <div>
                <div className="mb-0.5 text-[10px] text-inkfaint">B · v{activeVersion.version}</div>
                <AssetImage path={activeVersion.imagePath} alt="compare-b" className="h-[110px] w-full" />
              </div>
            </div>
          )}
          <div className="flex items-center justify-between border border-line bg-elevated/40 px-2 py-1.5 text-[10px] text-inkfaint">
            <span>
              {activeVersion ? `${activeVersion.width}×${activeVersion.height}` : `${asset.width}×${asset.height}`}
            </span>
            <span>{assetTypeLabel(asset.type)}</span>
            <span>v{asset.currentVersion}</span>
          </div>
        </div>

        {/* 右：信息与版本 */}
        <div className="flex min-w-0 flex-1 flex-col gap-2.5">
          <div className="grid grid-cols-2 gap-2">
            <label className="block">
              <span className="mb-1 block text-[11px] text-inkdim">名称</span>
              <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
            </label>
            <label className="block">
              <span className="mb-1 block text-[11px] text-inkdim">标签（逗号分隔）</span>
              <input className="input" value={tags} onChange={(e) => setTags(e.target.value)} placeholder="例如：主角, 剑" />
            </label>
          </div>

          <div className="grid grid-cols-3 gap-2 text-[10px] text-inkfaint">
            <div className="border border-line bg-elevated/40 px-2 py-1.5">
              <div>AI 服务商</div>
              <div className="truncate text-[11px] text-inkdim">{providerName(asset.providerId)}</div>
            </div>
            <div className="border border-line bg-elevated/40 px-2 py-1.5">
              <div>模型</div>
              <div className="truncate text-[11px] text-inkdim">{asset.model ?? "—"}</div>
            </div>
            <div className="border border-line bg-elevated/40 px-2 py-1.5">
              <div>创建时间</div>
              <div className="truncate text-[11px] text-inkdim">{formatDate(asset.createdAt)}</div>
            </div>
          </div>

          <div>
            <div className="mb-1 flex items-center justify-between text-[11px] text-inkdim">
              <span>提示词</span>
              <span className="text-inkfaint">{activeVersion?.note}</span>
            </div>
            <pre className="max-h-[72px] overflow-y-auto whitespace-pre-wrap break-words border border-line bg-input px-2 py-1.5 font-mono text-[10px] leading-relaxed text-inkdim">
              {promptText || "（无记录）"}
            </pre>
          </div>

          {/* 版本列表 */}
          <div className="min-h-0 flex-1">
            <div className="mb-1 flex items-center justify-between">
              <span className="text-[11px] text-inkdim">
                版本历史 <span className="text-inkfaint">（{versions.length}）</span>
              </span>
              <div className="flex gap-1">
                {compareWith ? (
                  <button className="btn btn-ghost h-5 px-1.5 text-[10px]" onClick={() => setCompareWith(null)}>
                    退出对比
                  </button>
                ) : (
                  versions.length > 1 && (
                    <span className="text-[10px] text-inkfaint">点「对比」选择 A 版本</span>
                  )
                )}
              </div>
            </div>
            <div className="max-h-[150px] space-y-1 overflow-y-auto pr-1">
              {[...versions].reverse().map((v) => (
                <div
                  key={v.id}
                  className={`flex items-center gap-2 border px-1.5 py-1 ${
                    activeVersion?.id === v.id ? "border-accent/60 bg-accent/5" : "border-line bg-elevated/30"
                  }`}
                >
                  <button className="shrink-0" onClick={() => setActiveVersion(v)} title="预览此版本">
                    <AssetImage path={v.thumbnailPath ?? v.imagePath} alt={`v${v.version}`} className="h-9 w-9" />
                  </button>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5 text-[11px] text-ink">
                      v{v.version}
                      {v.version === asset.currentVersion && <Badge tone="accent">当前</Badge>}
                      <span className="truncate text-inkfaint">{v.note}</span>
                    </div>
                    <div className="text-[10px] text-inkfaint">
                      {formatDate(v.createdAt)} · {v.width}×{v.height}
                    </div>
                  </div>
                  <div className="flex shrink-0 gap-0.5">
                    {compareWith?.id !== v.id && (
                      <button
                        className="btn btn-ghost h-6 w-6 p-0"
                        title="设为对比 A"
                        onClick={() => setCompareWith(v)}
                        disabled={busy}
                      >
                        <Icon name="copy" size={12} />
                      </button>
                    )}
                    <button
                      className="btn btn-ghost h-6 w-6 p-0"
                      title="还原为新版本（不覆盖历史）"
                      disabled={busy || v.version === asset.currentVersion}
                      onClick={() => void run(() => assetApi.restoreVersion(v.id), `已还原为 v${asset.currentVersion + 1}。`)}
                    >
                      <Icon name="refresh" size={12} />
                    </button>
                    <button
                      className="btn btn-ghost h-6 w-6 p-0 text-bad"
                      title="删除此版本"
                      disabled={busy || versions.length <= 1}
                      onClick={() => void run(() => assetApi.deleteVersion(v.id), "版本已删除。")}
                    >
                      <Icon name="trash" size={12} />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* 颜色 / 风格处理 */}
          <div className="border border-line bg-elevated/30 p-2">
            <div className="mb-1.5 flex items-center gap-1.5 text-[11px] text-inkdim">
              <Icon name="styles" size={12} />
              颜色 / 风格处理
              <span className="text-inkfaint">（结果保存为新版本）</span>
            </div>

            {/* 调色板提取 */}
            <div className="mb-2">
              <div className="mb-1 flex items-center gap-1.5">
                <span className="text-[10px] text-inkfaint">提取主色调色板：</span>
                {[8, 16, 32, 64].map((n) => (
                  <button
                    key={n}
                    className="btn h-5 px-1.5 font-mono text-[10px]"
                    disabled={busy}
                    onClick={() => void extractPalette(n)}
                  >
                    {n}
                  </button>
                ))}
                {palette.length > 0 && (
                  <button className="btn btn-ghost h-5 px-1 text-[10px]" onClick={() => setPalette([])}>
                    清除
                  </button>
                )}
              </div>
              {palette.length > 0 ? (
                <div className="flex flex-wrap items-center gap-1">
                  {palette.map((hex, i) => (
                    <span
                      key={hex + i}
                      className="flex h-6 w-6 items-end justify-center border border-linestrong"
                      style={{ backgroundColor: hex }}
                      title={hex}
                    >
                      <span className="w-full bg-black/55 text-center text-[7px] font-mono text-white">
                        {hex.slice(1)}
                      </span>
                    </span>
                  ))}
                  <label className="ml-1 flex items-center gap-1 text-[10px] text-inkdim">
                    <input type="checkbox" checked={paletteDither} onChange={(e) => setPaletteDither(e.target.checked)} />
                    抖动
                  </label>
                  <button
                    className="btn h-5 px-1.5 text-[10px]"
                    disabled={busy}
                    onClick={() =>
                      void run(() => imageApi.applyPalette(assetId, palette, paletteDither), "已按调色板生成新版本。")
                    }
                  >
                    <Icon name="check" size={10} />
                    应用为新版本
                  </button>
                </div>
              ) : (
                <div className="text-[10px] text-inkfaint">
                  提取后可将同一调色板应用到该资产（或复制 hex 用于其他资产，保证风格一致）。
                </div>
              )}
            </div>

            {/* 预设调色板（社区经典：PICO-8 / Sweetie 16 / DB16 / EDG32…） */}
            <div className="mb-2">
              <div className="mb-1 text-[10px] text-inkfaint">预设调色板（套用后保证风格统一）：</div>
              <div className="flex gap-1.5">
                <select
                  className="select flex-1"
                  value={presetPaletteId}
                  onChange={(e) => setPresetPaletteId(e.target.value)}
                >
                  <option value="">— 选择预设 —</option>
                  {PRESET_PALETTES.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.label}（{p.colors.length} 色）
                    </option>
                  ))}
                </select>
                <button
                  className="btn btn-primary shrink-0"
                  disabled={busy || !presetPaletteId}
                  onClick={() => {
                    const preset = PRESET_PALETTES.find((p) => p.id === presetPaletteId);
                    if (preset) {
                      void run(
                        () => imageApi.applyPalette(assetId, preset.colors, paletteDither),
                        `已套用「${preset.label}」生成新版本。`
                      );
                    }
                  }}
                >
                  <Icon name="check" size={10} />
                  套用
                </button>
              </div>
            </div>

            {/* Pixelize */}
            <div className="grid grid-cols-3 gap-1.5">
              <Field label="分辨率">
                <select className="select w-full font-mono text-[11px]" value={pxResolution} onChange={(e) => setPxResolution(Number(e.target.value))}>
                  <option value={0}>保持</option>
                  {[16, 32, 48, 64, 96, 128].map((r) => (
                    <option key={r} value={r}>
                      {r}px
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="调色板">
                <select className="select w-full font-mono text-[11px]" value={pxPalette} onChange={(e) => setPxPalette(Number(e.target.value))}>
                  <option value={0}>保持</option>
                  {[8, 16, 32, 64].map((n) => (
                    <option key={n} value={n}>
                      {n} 色
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="抖动">
                <select className="select w-full text-[11px]" value={pxDither ? "1" : "0"} onChange={(e) => setPxDither(e.target.value === "1")}>
                  <option value="0">无</option>
                  <option value="1">Bayer 4×4</option>
                </select>
              </Field>
            </div>
            <button
              className={cn("btn mt-1.5 w-full", (pxResolution > 0 || pxPalette > 0) && "btn-primary")}
              disabled={busy || (pxResolution === 0 && pxPalette === 0)}
              onClick={() =>
                void run(
                  () => imageApi.pixelize(assetId, pxResolution, pxPalette, pxDither),
                  "像素化处理完成，已生成新版本。"
                )
              }
            >
              <Icon name="pixel" size={11} />
              像素化处理
            </button>
          </div>

          {/* 拆分为动画 */}
          <div className="border border-line bg-elevated/30 p-2">
            <div className="mb-1.5 flex items-center gap-1.5 text-[11px] text-inkdim">
              <Icon name="animation" size={12} />
              拆分为动画
              <span className="text-inkfaint">（适合横向精灵表）</span>
            </div>
            <div className="grid grid-cols-4 gap-1.5">
              <Field label="列">
                <input
                  className="input font-mono text-[11px]"
                  type="number"
                  min={1}
                  value={splitCols}
                  onChange={(e) => setSplitCols(Math.max(1, Number(e.target.value)))}
                />
              </Field>
              <Field label="行">
                <input
                  className="input font-mono text-[11px]"
                  type="number"
                  min={1}
                  value={splitRows}
                  onChange={(e) => setSplitRows(Math.max(1, Number(e.target.value)))}
                />
              </Field>
              <Field label="FPS">
                <input
                  className="input font-mono text-[11px]"
                  type="number"
                  min={1}
                  value={splitFps}
                  onChange={(e) => setSplitFps(Math.max(1, Number(e.target.value)))}
                />
              </Field>
              <Field label="切帧">
                <select
                  className="select w-full text-[11px]"
                  value={splitAuto ? "auto" : "grid"}
                  onChange={(e) => setSplitAuto(e.target.value === "auto")}
                  title="自动：按透明间隙检测帧边界；网格：按列×行均分"
                >
                  <option value="auto">自动</option>
                  <option value="grid">网格</option>
                </select>
              </Field>
            </div>
            <button
              className="btn mt-1.5 w-full"
              disabled={busy || (splitAuto && splitRows > 1)}
              title={splitAuto && splitRows > 1 ? "自动切帧仅支持单行 Sheet，请切换为网格模式" : ""}
              onClick={() => void splitToAnimation()}
            >
              <Icon name="animation" size={11} />
              拆分并创建动画
            </button>
          </div>

          {/* 所在文件夹 */}
          <div className="flex items-center gap-2 border border-line bg-elevated/30 px-2 py-1.5">
            <Icon name="folder" size={12} className="shrink-0 text-inkfaint" />
            <code className="min-w-0 flex-1 truncate text-[9px] text-inkdim" title={activeVersion?.imagePath ?? asset.imagePath}>
              {folderOf(activeVersion?.imagePath ?? asset.imagePath)}
            </code>
            <button
              className="btn h-5 shrink-0 px-1.5 text-[10px]"
              onClick={() =>
                void settingsApi
                  .reveal(folderOf(activeVersion?.imagePath ?? asset.imagePath))
                  .catch((e) => pushToast("error", e instanceof Error ? e.message : String(e)))
              }
            >
              打开目录
            </button>
          </div>

          <div className="flex justify-end gap-2 border-t border-line pt-2">
            <button className="btn" disabled={busy} onClick={saveMeta}>
              <Icon name="check" size={12} />
              保存修改
            </button>
            <button
              className="btn btn-danger"
              disabled={busy}
              onClick={() => {
                void assetApi
                  .delete(asset.id)
                  .then(async () => {
                    if (loadedFor) await load(loadedFor);
                    pushToast("success", "资产已删除。");
                    onClose();
                  })
                  .catch((e) => pushToast("error", e instanceof Error ? e.message : String(e)));
              }}
            >
              <Icon name="trash" size={12} />
              删除资产
            </button>
          </div>
        </div>
      </div>
    </Modal>
  );
}
