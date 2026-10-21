import { useState } from "react";
import { Icon } from "@/components/ui/Icon";
import { Badge, Field, Spinner } from "@/components/ui/Panel";
import { Modal } from "@/components/ui/Modal";
import { generationApi } from "@/services";
import { useProviderStore } from "@/stores/providerStore";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { ANIM_TYPE_OPTIONS } from "@/utils/constants";
import type { AnimationView } from "@/types";

interface Props {
  animation: AnimationView;
  frameId: string;
  onClose: () => void;
}

/** 单帧 AI 重生成：只重新生成选中帧，不影响其他帧（§19 核心能力） */
export function RegenerateFrameModal({ animation, frameId, onClose }: Props) {
  const providers = useProviderStore((s) => s.providers);
  const pushToast = useUiStore((s) => s.pushToast);
  const refreshTasks = useTaskStore((s) => s.refresh);
  const [extra, setExtra] = useState("");
  const [providerId, setProviderId] = useState("");
  const [busy, setBusy] = useState(false);

  const activeProvider =
    providers.find((p) => p.id === providerId) ?? providers.find((p) => p.hasKey) ?? providers[0];
  const typeLabel = ANIM_TYPE_OPTIONS.find((t) => t.value === animation.type)?.label ?? animation.type;

  const submit = async () => {
    if (!activeProvider) return;
    setBusy(true);
    try {
      await generationApi.regenerateFrame({
        animationId: animation.id,
        frameId,
        providerId: activeProvider.id,
        extraInstructions: extra.trim(),
      });
      pushToast("success", "单帧重生成任务已加入队列，完成后当前动画自动刷新。");
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
      title={`AI 重生成单帧 · ${animation.name}`}
      onClose={onClose}
      width={480}
      footer={
        <>
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button className="btn btn-primary" disabled={busy || !activeProvider} onClick={() => void submit()}>
            {busy ? <Spinner size={11} /> : <Icon name="wand" size={12} />}
            生成
          </button>
        </>
      }
    >
      <div className="space-y-3">
        <div className="border border-line bg-elevated/40 px-2.5 py-2 text-[11px] leading-relaxed text-inkdim">
          <div className="mb-1 flex items-center gap-2">
            <Badge tone="accent">{typeLabel}</Badge>
            <span className="text-inkfaint">
              仅重画选中的一帧；旧帧自动备份到 history/ 文件夹。
            </span>
          </div>
          AI 将以当前帧为参考，保持角色外观、配色与风格完全不变，
          按内置动作逻辑重绘该帧的姿态（共 {animation.frameCount} 帧）。
          不会重新生成整个动画。
        </div>

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

        <Field label="姿态微调提示词" hint="可选">
          <input
            className="input"
            value={extra}
            onChange={(e) => setExtra(e.target.value)}
            placeholder="例如：武器挥得更高 / 身体前倾更多 / 手臂伸直"
          />
        </Field>
      </div>
    </Modal>
  );
}
