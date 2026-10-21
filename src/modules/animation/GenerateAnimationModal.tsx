import { useMemo, useState } from "react";
import { Icon } from "@/components/ui/Icon";
import { Badge, Field, Spinner } from "@/components/ui/Panel";
import { Modal } from "@/components/ui/Modal";
import { AssetImage } from "@/components/AssetImage";
import { generationApi } from "@/services";
import { useAssetStore } from "@/stores/assetStore";
import { useProjectStore } from "@/stores/projectStore";
import { useProviderStore } from "@/stores/providerStore";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { ANIM_TYPE_OPTIONS, CHARACTER_LIKE_TYPES, FPS_OPTIONS, RESOLUTIONS } from "@/utils/constants";
import { cn } from "@/utils/cn";
import type { AnimationType } from "@/types";

const FRAME_COUNTS = [2, 4, 6, 8, 12] as const;

export function GenerateAnimationModal({ onClose }: { onClose: () => void }) {
  const current = useProjectStore((s) => s.current);
  const providers = useProviderStore((s) => s.providers);
  const assets = useAssetStore((s) => s.assets);
  const pushToast = useUiStore((s) => s.pushToast);
  const refreshTasks = useTaskStore((s) => s.refresh);

  const [name, setName] = useState("");
  const [animType, setAnimType] = useState<AnimationType>("walk");
  const [frameCount, setFrameCount] = useState(4);
  const [fps, setFps] = useState(8);
  const [resolution, setResolution] = useState(64);
  const [sourceAssetId, setSourceAssetId] = useState<string>("");
  const [description, setDescription] = useState("");
  const [extra, setExtra] = useState("");
  const [removeBg, setRemoveBg] = useState(true);
  const [autoSplit, setAutoSplit] = useState(true);
  const [providerId, setProviderId] = useState("");
  const [busy, setBusy] = useState(false);

  const characterAssets = useMemo(
    () => assets.filter((a) => CHARACTER_LIKE_TYPES.includes(a.type)),
    [assets]
  );
  const activeProvider =
    providers.find((p) => p.id === providerId) ?? providers.find((p) => p.hasKey) ?? providers[0];
  const sourceAsset = characterAssets.find((a) => a.id === sourceAssetId) ?? null;
  const animTypeLabel = ANIM_TYPE_OPTIONS.find((t) => t.value === animType)?.label ?? animType;

  const submit = async () => {
    if (!current || !activeProvider) return;
    if (!name.trim()) {
      pushToast("warn", "请填写动画名称。");
      return;
    }
    if (!sourceAsset && !description.trim()) {
      pushToast("warn", "请选择参考角色资产，或填写角色描述。");
      return;
    }
    setBusy(true);
    try {
      await generationApi.startAnimation({
        projectId: current.id,
        name: name.trim(),
        animType,
        frameCount,
        fps,
        resolution,
        providerId: activeProvider.id,
        sourceAssetId: sourceAsset?.id ?? null,
        description: description.trim(),
        extraInstructions: extra.trim(),
        removeBackground: removeBg,
        autoSplit,
      });
      pushToast("success", "动画生成任务已加入队列，完成后自动出现在列表。");
      void refreshTasks();
      onClose();
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="从角色生成动画"
      onClose={onClose}
      width={560}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button
            className="btn btn-primary"
            disabled={busy || !current || !activeProvider}
            onClick={() => void submit()}
          >
            {busy ? <Spinner size={11} /> : <Icon name="generate" size={12} />}
            加入任务队列
          </button>
        </>
      }
    >
      <div className="space-y-3">
        {/* 参考角色选择 */}
        <Field label="参考角色（已有资产）" hint={sourceAsset ? "将使用 AI 图生图保持角色一致" : "不选则按描述直接生成"}>
          <select className="select w-full" value={sourceAssetId} onChange={(e) => setSourceAssetId(e.target.value)}>
            <option value="">— 无参考（按描述生成新角色动画）—</option>
            {characterAssets.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}（{a.width}×{a.height}）
              </option>
            ))}
          </select>
        </Field>

        {sourceAsset && (
          <div className="flex items-center gap-2.5 border border-accent/40 bg-accent/5 p-2">
            <AssetImage path={sourceAsset.thumbnailPath ?? sourceAsset.imagePath} alt={sourceAsset.name} className="h-14 w-14" />
            <div className="text-[11px] leading-relaxed text-inkdim">
              <div className="text-[12px] text-ink">{sourceAsset.name}</div>
              将以此角色的外观 / 配色 / 比例为基准生成{animTypeLabel}动画，
              逐帧姿态由内置动作逻辑控制。
            </div>
          </div>
        )}

        <div className="grid grid-cols-2 gap-2">
          <Field label="动画名称">
            <input className="input" value={name} onChange={(e) => setName(e.target.value)} placeholder="例如：骑士行走" />
          </Field>
          <Field label="AI 服务商">
            <select className="select w-full" value={activeProvider?.id ?? ""} onChange={(e) => setProviderId(e.target.value)}>
              {providers.length === 0 && <option value="">未配置</option>}
              {providers.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                  {p.hasKey ? "" : "（缺少 Key）"}
                </option>
              ))}
            </select>
          </Field>
        </div>

        <div className="grid grid-cols-2 gap-2">
          <Field label="动画类型（动作逻辑）">
            <select className="select w-full" value={animType} onChange={(e) => setAnimType(e.target.value as AnimationType)}>
              {ANIM_TYPE_OPTIONS.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="输出分辨率（单帧）">
            <select className="select w-full font-mono" value={resolution} onChange={(e) => setResolution(Number(e.target.value))}>
              <option value={0}>保持原始</option>
              {RESOLUTIONS.map((r) => (
                <option key={r} value={r}>
                  {r}×{r}
                </option>
              ))}
            </select>
          </Field>
        </div>

        <Field label="帧数">
          <div className="flex gap-1">
            {FRAME_COUNTS.map((fc) => (
              <button
                key={fc}
                className={cn("btn h-6 flex-1 px-0 font-mono", frameCount === fc && "border-accent/70 text-accent")}
                onClick={() => setFrameCount(fc)}
              >
                {fc}
              </button>
            ))}
          </div>
        </Field>

        <div className="grid grid-cols-2 gap-2">
          <Field label="FPS">
            <select className="select w-full" value={fps} onChange={(e) => setFps(Number(e.target.value))}>
              {FPS_OPTIONS.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </select>
          </Field>
          <Field label="角色描述（无参考时必填）">
            <input
              className="input"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={sourceAsset ? "可补充：受击时帽子会飞起" : "例如：绿色史莱姆，大眼睛"}
            />
          </Field>
        </div>

        <Field label="附加提示词">
          <input className="input" value={extra} onChange={(e) => setExtra(e.target.value)} placeholder="可选，追加到提示词末尾" />
        </Field>

        <label className="flex items-center gap-2 text-[11px] text-inkdim">
          <input type="checkbox" checked={removeBg} onChange={(e) => setRemoveBg(e.target.checked)} />
          生成后自动移除背景
        </label>
        <label className="flex items-center gap-2 text-[11px] text-inkdim">
          <input type="checkbox" checked={autoSplit} onChange={(e) => setAutoSplit(e.target.checked)} />
          智能切帧（按透明间隙检测帧边界，帧距不均时比均分更可靠）
        </label>

        <div className="border border-line bg-elevated/40 px-2 py-1.5 text-[10px] leading-relaxed text-inkfaint">
          <div className="mb-0.5 flex items-center gap-1.5">
            <Badge tone="accent">动作逻辑</Badge>
            内置逐帧关键帧模板（接触 / 过渡 / 发力 / 回收），保证动作在逻辑上成立。
          </div>
          示例（行走 Walk）：接触步 → 过渡（身体最高）→ 反向接触步 → 过渡（身体最低）；
          多余帧自动在关键帧之间平滑过渡。
        </div>
      </div>
    </Modal>
  );
}
