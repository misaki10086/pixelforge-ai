import { useMemo, useState } from "react";
import { Icon } from "@/components/ui/Icon";
import { Badge, Field, Panel } from "@/components/ui/Panel";
import { AssetImage } from "@/components/AssetImage";
import { generationApi } from "@/services";
import { useAssetStore } from "@/stores/assetStore";
import { useProjectStore } from "@/stores/projectStore";
import { useProviderStore } from "@/stores/providerStore";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { buildPromptPreview } from "@/utils/promptPreview";
import {
  ANIMATIONS,
  ASSET_TYPES,
  DIRECTIONS,
  FRAME_COUNTS,
  GENERATOR_READY_TYPES,
  OUTLINES,
  PALETTES,
  PERSPECTIVES,
  PRESET_PALETTES,
  RESOLUTIONS,
  STYLE_PRESETS,
} from "@/utils/constants";
import { cn } from "@/utils/cn";
import type { AssetType, GenerationParams } from "@/types";

const DEFAULT_FORM = {
  assetName: "",
  description: "",
  style: "16bit",
  resolution: 64,
  perspective: "side",
  direction: "front",
  palette: "none",
  outline: "1px",
  animation: "none",
  frameCount: 1,
  extraInstructions: "",
  autoResize: true,
  removeBackground: true,
};

type FormState = typeof DEFAULT_FORM;

export function GeneratePage() {
  const current = useProjectStore((s) => s.current);
  const providers = useProviderStore((s) => s.providers);
  const pushToast = useUiStore((s) => s.pushToast);
  const openDetail = useAssetStore((s) => s.openDetail);
  const assets = useAssetStore((s) => s.assets);
  const refreshTasks = useTaskStore((s) => s.refresh);
  const tasks = useTaskStore((s) => s.tasks);

  const [assetType, setAssetType] = useState<AssetType>("character");
  const [form, setForm] = useState<FormState>(DEFAULT_FORM);
  const [advanced, setAdvanced] = useState(false);
  const [providerId, setProviderId] = useState<string>("");
  const [submitting, setSubmitting] = useState(false);

  // 本次会话最近一次成功生成的资产
  const lastSuccess = tasks.find((t) => t.status === "success" && t.assetId && t.projectId === current?.id);
  const lastAsset = lastSuccess ? assets.find((a) => a.id === lastSuccess.assetId) ?? null : null;

  // Provider 选择：手动选择优先，否则取第一个已配置 Key 的
  const activeProvider =
    providers.find((p) => p.id === providerId) ?? providers.find((p) => p.hasKey) ?? providers[0];

  const ready = GENERATOR_READY_TYPES.includes(assetType);

  // 调色板锁定（Retro Diffusion 思路）：选择带色值的预设调色板时，
  // 把精确 hex 列表注入提示词，让 AI 直接按调色板作画，生成结果更易量化对齐
  const paletteLock = useMemo(() => {
    const paletteHexIds: Record<string, string> = { pico8: "pico8", gameboy: "gb", cga: "cga" };
    const presetId = paletteHexIds[form.palette];
    if (!presetId) return "";
    const preset = PRESET_PALETTES.find((p) => p.id === presetId);
    if (!preset) return "";
    return `Use exactly this ${preset.colors.length}-color palette and no other colors: ${preset.colors.join(", ")}`;
  }, [form.palette]);

  const params: GenerationParams = useMemo(
    () => ({
      projectId: current?.id ?? "",
      assetType,
      assetName: form.assetName,
      providerId: activeProvider?.id ?? "",
      description: form.description,
      style: form.style,
      resolution: form.resolution,
      perspective: form.perspective,
      direction: form.direction,
      palette: form.palette,
      outline: form.outline,
      animation: form.animation,
      frameCount: form.frameCount,
      extraInstructions: [form.extraInstructions, paletteLock].filter(Boolean).join(" | "),
      autoResize: form.autoResize,
      removeBackground: form.removeBackground,
    }),
    [current?.id, assetType, form, activeProvider?.id, paletteLock]
  );

  const promptPreview = useMemo(() => buildPromptPreview(params), [params]);

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const submit = async (repeat: boolean) => {
    if (!current) {
      pushToast("warn", "请先创建或打开一个项目。");
      return;
    }
    if (!activeProvider) {
      pushToast("warn", "请先在「设置 → AI 服务商」中配置一个服务商并填写 API Key。");
      return;
    }
    if (!form.assetName.trim() || (!repeat && !form.description.trim())) {
      pushToast("warn", "请填写资产名称与描述。");
      return;
    }
    setSubmitting(true);
    try {
      const task = await generationApi.start(params);
      pushToast("success", `任务已加入队列（${task.id.slice(0, 6)}）。`);
      void refreshTasks();
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const viewResult = async () => {
    if (!lastAsset) return;
    try {
      await openDetail(lastAsset.id);
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex h-full flex-col gap-2 p-2">
      {/* 资产类型选择 */}
      <div className="flex shrink-0 items-center gap-1 border-b border-line pb-2">
        {ASSET_TYPES.map((t) => {
          const isReady = GENERATOR_READY_TYPES.includes(t.value);
          return (
            <button
              key={t.value}
              className={cn(
                "btn h-7 px-2.5",
                assetType === t.value && "border-accent/70 bg-accent/10 text-accent",
                !isReady && "opacity-55"
              )}
              onClick={() => setAssetType(t.value)}
              title={isReady ? t.label : `${t.label} 生成器（即将上线）`}
            >
              {t.label}
              {!isReady && <span className="ml-1 text-[8px] text-inkfaint">即将</span>}
            </button>
          );
        })}
      </div>

      {!ready ? (
        <Panel className="flex-1" scroll={false}>
          <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
            <Badge tone="warn">即将上线</Badge>
            <div className="text-[14px] text-ink">{assetTypeLabel(assetType)} 生成器将在后续阶段开放</div>
            <div className="max-w-md text-[11px] leading-relaxed text-inkfaint">
              将支持专属参数（如地块自动九宫、特效帧序列、模块化场景套件）与批量生成。
              当前阶段请先使用角色生成器。
            </div>
          </div>
        </Panel>
      ) : (
        <div className="flex min-h-0 flex-1 gap-2">
          {/* 左侧参数表单 */}
          <div className="w-[340px] shrink-0 overflow-y-auto pr-0.5">
            <Panel
              title={
                <>
                  <Icon name="generate" size={12} />
                  角色生成器
                </>
              }
              bodyClassName="space-y-3 p-2.5"
            >
              <Field label="资产名称" hint="创建资产时使用">
                <input
                  className="input"
                  value={form.assetName}
                  onChange={(e) => set("assetName", e.target.value)}
                  placeholder="例如：骑士 / 法师 / 史莱姆"
                />
              </Field>

              <Field label="描述" hint="越具体越好">
                <textarea
                  className="input h-[64px] resize-none py-1.5"
                  value={form.description}
                  onChange={(e) => set("description", e.target.value)}
                  placeholder="例如：身披银甲、持火焰剑的年轻骑士，披风随风摆动"
                />
              </Field>

              <div className="grid grid-cols-2 gap-2">
                <Field label="风格">
                  <select className="select w-full" value={form.style} onChange={(e) => set("style", e.target.value)}>
                    {STYLE_PRESETS.map((s) => (
                      <option key={s.value} value={s.value}>
                        {s.label}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="分辨率">
                  <select
                    className="select w-full font-mono"
                    value={form.resolution}
                    onChange={(e) => set("resolution", Number(e.target.value))}
                  >
                    {RESOLUTIONS.map((r) => (
                      <option key={r} value={r}>
                        {r}×{r}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="视角">
                  <select className="select w-full" value={form.perspective} onChange={(e) => set("perspective", e.target.value)}>
                    {PERSPECTIVES.map((p) => (
                      <option key={p.value} value={p.value}>
                        {p.label}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="朝向">
                  <select className="select w-full" value={form.direction} onChange={(e) => set("direction", e.target.value)}>
                    {DIRECTIONS.map((d) => (
                      <option key={d.value} value={d.value}>
                        {d.label}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="调色板" hint={paletteLock ? "已锁定色值" : undefined}>
                  <select className="select w-full" value={form.palette} onChange={(e) => set("palette", e.target.value)}>
                    {PALETTES.map((p) => (
                      <option key={p.value} value={p.value}>
                        {p.label}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="描边">
                  <select className="select w-full" value={form.outline} onChange={(e) => set("outline", e.target.value)}>
                    {OUTLINES.map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </Field>
              </div>

              <Field label="动画">
                <select className="select w-full" value={form.animation} onChange={(e) => set("animation", e.target.value)}>
                  {ANIMATIONS.map((a) => (
                    <option key={a.value} value={a.value}>
                      {a.label}
                    </option>
                  ))}
                </select>
              </Field>

              {form.animation !== "none" && (
                <Field label="帧数" hint="生成横向精灵表">
                  <div className="flex gap-1">
                    {FRAME_COUNTS.map((fc) => (
                      <button
                        key={fc}
                        className={cn("btn h-6 flex-1 px-0 font-mono", form.frameCount === fc && "border-accent/70 text-accent")}
                        onClick={() => set("frameCount", fc)}
                      >
                        {fc}
                      </button>
                    ))}
                  </div>
                </Field>
              )}

              {/* 高级设置 */}
              <div>
                <button
                  className="flex w-full items-center gap-1 text-[11px] text-inkdim hover:text-ink"
                  onClick={() => setAdvanced((v) => !v)}
                >
                  <Icon name="chevron" size={12} className={cn("transition-transform", advanced && "rotate-180")} />
                  高级设置
                </button>
                {advanced && (
                  <div className="mt-2 space-y-2.5 border border-line bg-elevated/30 p-2">
                    <Field label="附加提示词" hint="追加到提示词末尾">
                      <textarea
                        className="input h-[48px] resize-none py-1.5"
                        value={form.extraInstructions}
                        onChange={(e) => set("extraInstructions", e.target.value)}
                        placeholder="例如：银色铠甲 / 红色披风"
                      />
                    </Field>
                    <label className="flex items-center gap-2 text-[11px] text-inkdim">
                      <input
                        type="checkbox"
                        checked={form.autoResize}
                        onChange={(e) => set("autoResize", e.target.checked)}
                      />
                      自动缩放到目标分辨率（最近邻）
                    </label>
                    <label className="flex items-center gap-2 text-[11px] text-inkdim">
                      <input
                        type="checkbox"
                        checked={form.removeBackground}
                        onChange={(e) => set("removeBackground", e.target.checked)}
                      />
                      移除纯色背景（边沿洪泛）
                    </label>
                    <div className="text-[10px] leading-relaxed text-inkfaint">
                      使用参考图与蒙版的局部编辑将在第三阶段开放。
                    </div>
                  </div>
                )}
              </div>

              <Field label="AI 服务商" hint={activeProvider && !activeProvider.hasKey ? "该服务商缺少 API Key" : undefined}>
                <select
                  className="select w-full"
                  value={activeProvider?.id ?? ""}
                  onChange={(e) => setProviderId(e.target.value)}
                >
                  {providers.length === 0 && <option value="">未配置（前往设置）</option>}
                  {providers.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                      {p.hasKey ? "" : "（缺少 Key）"}
                    </option>
                  ))}
                </select>
              </Field>

              <div className="flex gap-2 pt-1">
                <button
                  className="btn btn-primary flex-1"
                  disabled={submitting || !current || !activeProvider}
                  onClick={() => void submit(false)}
                  title={!current ? "请先打开项目" : !activeProvider ? "请先配置服务商" : ""}
                >
                  <Icon name="generate" size={13} />
                  {submitting ? "提交中…" : "开始生成"}
                </button>
                <button
                  className="btn"
                  disabled={submitting || !current || !activeProvider}
                  onClick={() => void submit(true)}
                  title="使用当前参数重新生成"
                >
                  <Icon name="refresh" size={13} />
                  再次生成
                </button>
              </div>
            </Panel>
          </div>

          {/* 右侧预览区 */}
          <Panel
            title={
              <>
                <Icon name="eye" size={12} />
                提示词预览 / 结果
              </>
            }
            className="min-w-0 flex-1"
            bodyClassName="flex flex-col p-2.5"
          >
            <div className="mb-2 flex items-center gap-2 text-[10px] text-inkfaint">
              <Badge tone="accent">{activeProvider ? activeProvider.name : "未选择服务商"}</Badge>
              {activeProvider && <span>{activeProvider.imageModel || activeProvider.model || "未设置图像模型"}</span>}
              <span>实际输出 1024×1024{form.autoResize && ` → 缩放至 ${form.resolution}×${form.resolution}`}</span>
            </div>

            <pre className="checkerboard min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-words border border-line p-2.5 font-mono text-[11px] leading-relaxed text-inkdim">
              {promptPreview}
            </pre>

            {lastAsset && (
              <div className="mt-2 flex items-center gap-2.5 border border-ok/40 bg-ok/5 p-2">
                <AssetImage
                  path={lastAsset.thumbnailPath ?? lastAsset.imagePath}
                  alt={lastAsset.name}
                  className="h-[72px] w-[72px] shrink-0"
                />
                <div className="min-w-0 flex-1">
                  <div className="text-[12px] text-ink">{lastAsset.name}</div>
                  <div className="text-[10px] text-inkfaint">
                    生成完成 · {lastAsset.width}×{lastAsset.height} · v{lastAsset.currentVersion}
                  </div>
                  <button className="btn mt-1.5 h-6 px-2 text-[11px]" onClick={() => void viewResult()}>
                    <Icon name="eye" size={12} />
                    查看详情 / 导出
                  </button>
                </div>
              </div>
            )}

            {!current && (
              <div className="mt-2 border border-warn/40 bg-warn/10 px-2 py-1.5 text-[11px] text-warn">
                请先在「项目」页创建项目，生成结果将保存到项目目录。
              </div>
            )}
          </Panel>
        </div>
      )}
    </div>
  );
}

function assetTypeLabel(t: AssetType): string {
  return ASSET_TYPES.find((x) => x.value === t)?.label ?? t;
}
