import { useCallback, useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Icon } from "@/components/ui/Icon";
import { Badge, Field, Panel, Spinner } from "@/components/ui/Panel";
import { ProviderFormModal } from "./ProviderFormModal";
import { providerApi, settingsApi } from "@/services";
import { useProviderStore } from "@/stores/providerStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useUiStore } from "@/stores/uiStore";
import { PROVIDER_KIND_LABELS } from "@/utils/constants";
import { cn } from "@/utils/cn";
import type { ProviderView, ThemeMode } from "@/types";

type Section = "general" | "providers" | "storage" | "appearance" | "shortcuts";

const SECTIONS: Array<{ id: Section; label: string }> = [
  { id: "general", label: "通用" },
  { id: "providers", label: "AI 服务商" },
  { id: "storage", label: "存储" },
  { id: "appearance", label: "外观" },
  { id: "shortcuts", label: "快捷键" },
];

function ProvidersSection() {
  const providers = useProviderStore((s) => s.providers);
  const pushToast = useUiStore((s) => s.pushToast);
  const [editing, setEditing] = useState<ProviderView | "new" | null>(null);
  const [testing, setTesting] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<Record<string, string>>({});

  const test = async (p: ProviderView) => {
    setTesting(p.id);
    setTestResult((m) => ({ ...m, [p.id]: "" }));
    try {
      const result = await providerApi.test({
        id: p.id,
        name: p.name,
        kind: p.kind,
        baseUrl: p.baseUrl,
        model: p.model,
        imageModel: p.imageModel,
        requestFormat: p.requestFormat,
        headers: p.headers,
        apiKey: "",
        isDefault: p.isDefault,
      });
      setTestResult((m) => ({
        ...m,
        [p.id]: result.ok ? `✓ ${result.message}` : `✕ ${result.message}`,
      }));
      if (result.ok) pushToast("success", `「${p.name}」连接成功。`);
      else pushToast("error", result.message);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setTestResult((m) => ({ ...m, [p.id]: `✕ ${msg}` }));
      pushToast("error", msg);
    } finally {
      setTesting(null);
    }
  };

  const fetchModels = async (p: ProviderView) => {
    try {
      const models = await providerApi.listModels(p.id);
      pushToast(
        models.length > 0 ? "success" : "warn",
        models.length > 0 ? `「${p.name}」共 ${models.length} 个模型，示例：${models.slice(0, 3).join(", ")}…` : "服务商未返回任何模型。"
      );
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <p className="max-w-lg text-[11px] leading-relaxed text-inkfaint">
          UI 通过 Service 层调用服务商管理器，不直接访问 AI SDK。API Key 存储在 Windows
          凭据管理器中，不写入数据库与配置文件。
        </p>
        <button className="btn btn-primary h-7 shrink-0" onClick={() => setEditing("new")}>
          <Icon name="plus" size={12} />
          新增服务商
        </button>
      </div>

      {providers.length === 0 && (
        <div className="border border-dashed border-line p-6 text-center text-[12px] text-inkfaint">
          尚未配置任何服务商。添加 OpenAI、Gemini 或自定义 OpenAI 兼容接口后即可开始生成。
        </div>
      )}

      {providers.map((p) => (
        <div key={p.id} className="border border-line bg-panel">
          <div className="flex items-center gap-2 px-2.5 py-2">
            <span className="text-[12px] font-medium text-ink">{p.name}</span>
            <Badge>{PROVIDER_KIND_LABELS[p.kind] ?? p.kind}</Badge>
            {p.hasKey ? <Badge tone="ok">Key 已配置</Badge> : <Badge tone="bad">缺少 Key</Badge>}
            <span className="ml-2 truncate font-mono text-[10px] text-inkfaint" title={p.baseUrl}>
              {p.baseUrl || "（默认地址）"} · {p.imageModel || p.model || "未设置模型"}
            </span>
            <div className="ml-auto flex shrink-0 gap-1">
              <button className="btn h-6 px-2 text-[11px]" disabled={testing === p.id} onClick={() => void test(p)}>
                {testing === p.id ? <Spinner size={11} /> : <Icon name="check" size={12} />}
                测试连接
              </button>
              <button className="btn h-6 px-2 text-[11px]" onClick={() => void fetchModels(p)}>
                <Icon name="refresh" size={12} />
                获取模型
              </button>
              <button className="btn btn-ghost h-6 w-6 p-0" title="编辑" onClick={() => setEditing(p)}>
                <Icon name="editor" size={12} />
              </button>
              <button
                className="btn btn-ghost h-6 w-6 p-0 text-bad"
                title="删除"
                onClick={() =>
                  void providerApi
                    .delete(p.id)
                    .then(() => useProviderStore.getState().load())
                    .then(() => pushToast("success", `已删除「${p.name}」。`))
                    .catch((e) => pushToast("error", e instanceof Error ? e.message : String(e)))
                }
              >
                <Icon name="trash" size={12} />
              </button>
            </div>
          </div>
          {testResult[p.id] && (
            <div
              className={cn(
                "border-t px-2.5 py-1.5 text-[11px]",
                testResult[p.id].startsWith("✓") ? "border-ok/40 text-ok" : "border-bad/40 text-bad"
              )}
            >
              {testResult[p.id]}
            </div>
          )}
        </div>
      ))}

      {editing && (
        <ProviderFormModal
          initial={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
        />
      )}
    </div>
  );
}

function StorageSection() {
  const values = useSettingsStore((s) => s.values);
  const setSetting = useSettingsStore((s) => s.set);
  const pushToast = useUiStore((s) => s.pushToast);

  const pickDir = async (key: "project_dir") => {
    const picked = await openDialog({ directory: true, title: "选择目录" });
    if (typeof picked === "string") {
      await setSetting(key, picked);
      pushToast("success", "存储目录已更新。");
    }
  };

  return (
    <div className="max-w-xl space-y-3">
      <Field label="项目目录" hint="所有项目默认保存在此目录下">
        <div className="flex gap-1.5">
          <input className="input font-mono text-[11px]" value={values["project_dir"] ?? ""} readOnly />
          <button className="btn shrink-0" onClick={() => void pickDir("project_dir")}>
            <Icon name="folder" size={13} />
            浏览
          </button>
          <button
            className="btn shrink-0"
            onClick={() => void settingsApi.reveal(values["project_dir"] ?? "").catch((e) => pushToast("error", String(e)))}
          >
            打开
          </button>
        </div>
      </Field>
      <Field label="缓存目录" hint="应用数据与缓存位置">
        <div className="flex gap-1.5">
          <input className="input font-mono text-[11px]" value={values["cache_dir"] ?? ""} readOnly />
          <button
            className="btn shrink-0"
            onClick={() => void settingsApi.reveal(values["cache_dir"] ?? "").catch((e) => pushToast("error", String(e)))}
          >
            打开
          </button>
        </div>
      </Field>
      <p className="text-[10px] leading-relaxed text-inkfaint">
        数据库（SQLite）保存在应用数据目录；图片等大文件保存在各项目目录内，数据库不存储图片二进制。
      </p>
    </div>
  );
}

function AppearanceSection() {
  const theme = useSettingsStore((s) => s.theme);
  const setSetting = useSettingsStore((s) => s.set);

  const options: Array<{ id: ThemeMode; label: string }> = [
    { id: "dark", label: "深色（默认）" },
    { id: "light", label: "浅色" },
  ];

  return (
    <div className="space-y-2">
      {options.map((o) => (
        <label key={o.id} className="flex w-56 cursor-pointer items-center gap-2 border border-line bg-panel px-3 py-2 text-[12px]">
          <input
            type="radio"
            name="theme"
            checked={theme === o.id}
            onChange={() => void setSetting("theme", o.id)}
          />
          <span className={theme === o.id ? "text-ink" : "text-inkdim"}>{o.label}</span>
          {o.id === "dark" && <Badge className="ml-auto">推荐</Badge>}
        </label>
      ))}
      <p className="text-[10px] text-inkfaint">浅色主题为辅助可读性提供的基础支持，深色为完整优化主题。</p>
    </div>
  );
}

const SHORTCUTS: Array<[string, string, boolean]> = [
  ["Ctrl+S", "保存项目元数据", false],
  ["Ctrl+O", "打开项目（跳转项目页）", false],
  ["Ctrl+Z / Ctrl+Shift+Z", "撤销 / 重做（编辑器）", true],
  ["Ctrl+C / Ctrl+V", "复制 / 粘贴像素（编辑器）", true],
  ["Delete", "删除选区（编辑器）", true],
  ["Space", "播放 / 暂停动画（时间轴）", true],
];

function ShortcutsSection() {
  return (
    <div className="max-w-xl space-y-1">
      {SHORTCUTS.map(([key, desc, soon]) => (
        <div key={key} className="flex items-center justify-between border border-line bg-panel px-2.5 py-1.5">
          <span className="font-mono text-[11px] text-ink">{key}</span>
          <span className="flex items-center gap-2 text-[11px] text-inkdim">
            {desc}
            {soon && <Badge tone="warn">即将上线</Badge>}
          </span>
        </div>
      ))}
    </div>
  );
}

function GeneralSection() {
  return (
    <div className="max-w-xl space-y-2 text-[12px] leading-relaxed text-inkdim">
      <p>
        PixelForge AI 是本地优先的 AI 游戏资产工作台：项目、资产、任务与配置全部保存在本机。
      </p>
      <div className="grid grid-cols-2 gap-2 pt-1">
        <div className="border border-line bg-panel px-2.5 py-2">
          <div className="text-[10px] uppercase tracking-wider text-inkfaint">工作流</div>
          AI 生成 → 资产处理 → 动画 → 管理 → 引擎导出
        </div>
        <div className="border border-line bg-panel px-2.5 py-2">
          <div className="text-[10px] uppercase tracking-wider text-inkfaint">数据边界</div>
          UI 不直接访问文件系统 / SQLite / Provider
        </div>
      </div>
    </div>
  );
}

export function SettingsPage() {
  const [section, setSection] = useState<Section>("providers");
  const loadProviders = useCallback(() => useProviderStore.getState().load(), []);
  useEffect(() => {
    void loadProviders();
  }, [loadProviders]);

  return (
    <div className="flex h-full gap-2 overflow-hidden p-2">
      <div className="w-[150px] shrink-0">
        <Panel title={<>设置</>} bodyClassName="p-1.5">
          {SECTIONS.map((s) => (
            <button
              key={s.id}
              className={cn(
                "flex h-8 w-full items-center px-2.5 text-[12px]",
                section === s.id ? "bg-selbg text-accent" : "text-inkdim hover:bg-hover hover:text-ink"
              )}
              onClick={() => setSection(s.id)}
            >
              {s.label}
            </button>
          ))}
        </Panel>
      </div>
      <div className="min-w-0 flex-1 overflow-y-auto">
        <Panel title={<>{SECTIONS.find((s) => s.id === section)?.label}</>} bodyClassName="p-3">
          {section === "general" && <GeneralSection />}
          {section === "providers" && <ProvidersSection />}
          {section === "storage" && <StorageSection />}
          {section === "appearance" && <AppearanceSection />}
          {section === "shortcuts" && <ShortcutsSection />}
        </Panel>
      </div>
    </div>
  );
}
