import { useEffect, useState } from "react";
import { Icon } from "@/components/ui/Icon";
import { Field, Spinner } from "@/components/ui/Panel";
import { Modal } from "@/components/ui/Modal";
import { providerApi } from "@/services";
import { useProviderStore } from "@/stores/providerStore";
import { useUiStore } from "@/stores/uiStore";
import { DEFAULT_BASE_URLS, DEFAULT_IMAGE_MODELS, PROVIDER_KIND_LABELS } from "@/utils/constants";
import type { HeaderPair, ProviderInput, ProviderKind, ProviderView } from "@/types";

const KINDS: ProviderKind[] = ["openai", "gemini", "custom"];

const EMPTY: ProviderInput = {
  name: "",
  kind: "openai",
  baseUrl: DEFAULT_BASE_URLS["openai"] ?? "",
  model: "",
  imageModel: DEFAULT_IMAGE_MODELS["openai"] ?? "",
  requestFormat: "openai",
  headers: [],
  apiKey: "",
  isDefault: false,
};

export function ProviderFormModal({ initial, onClose }: { initial: ProviderView | null; onClose: () => void }) {
  const pushToast = useUiStore((s) => s.pushToast);
  const [form, setForm] = useState<ProviderInput>(EMPTY);
  const [models, setModels] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testMsg, setTestMsg] = useState<string | null>(null);

  useEffect(() => {
    if (initial) {
      setForm({
        id: initial.id,
        name: initial.name,
        kind: initial.kind,
        baseUrl: initial.baseUrl,
        model: initial.model,
        imageModel: initial.imageModel,
        requestFormat: initial.requestFormat,
        headers: initial.headers,
        apiKey: "",
        isDefault: initial.isDefault,
      });
    }
  }, [initial]);

  const set = <K extends keyof ProviderInput>(key: K, value: ProviderInput[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const switchKind = (kind: ProviderKind) => {
    setForm((f) => ({
      ...f,
      kind,
      baseUrl: f.baseUrl || DEFAULT_BASE_URLS[kind] || "",
      imageModel: f.imageModel || DEFAULT_IMAGE_MODELS[kind] || "",
      requestFormat: kind === "gemini" ? "gemini" : "openai",
    }));
  };

  const setHeader = (idx: number, patch: Partial<HeaderPair>) =>
    setForm((f) => ({
      ...f,
      headers: f.headers.map((h, i) => (i === idx ? { ...h, ...patch } : h)),
    }));

  const submit = async () => {
    setBusy(true);
    try {
      const saved = await providerApi.save(form);
      await useProviderStore.getState().load();
      pushToast("success", `服务商「${saved.name}」已保存。`);
      onClose();
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const testConnection = async () => {
    setTesting(true);
    setTestMsg(null);
    try {
      const result = await providerApi.test(form);
      setModels(result.models);
      setTestMsg(result.ok ? `✓ ${result.message}` : `✕ ${result.message}`);
    } catch (e) {
      setTestMsg(`✕ ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setTesting(false);
    }
  };

  return (
    <Modal
      title={initial ? `编辑服务商 · ${initial.name}` : "新增服务商"}
      onClose={onClose}
      width={560}
      footer={
        <>
          <button className="btn" type="button" disabled={testing} onClick={() => void testConnection()}>
            {testing ? <Spinner size={11} /> : <Icon name="check" size={12} />}
            测试连接
          </button>
          <button className="btn btn-primary" disabled={busy || !form.name.trim()} onClick={() => void submit()}>
            {busy ? "保存中…" : "保存"}
          </button>
        </>
      }
    >
      <form className="space-y-3" onSubmit={(e) => e.preventDefault()}>
        <div className="grid grid-cols-2 gap-2">
          <Field label="名称" hint="必填">
            <input
              className="input"
              value={form.name}
              onChange={(e) => set("name", e.target.value)}
              placeholder="例如：OpenAI 主力 / 本地中转"
            />
          </Field>
          <Field label="类型">
            <select className="select w-full" value={form.kind} onChange={(e) => switchKind(e.target.value as ProviderKind)}>
              {KINDS.map((k) => (
                <option key={k} value={k}>
                  {PROVIDER_KIND_LABELS[k]}
                </option>
              ))}
            </select>
          </Field>
        </div>

        <Field
          label="接口地址"
          hint={form.kind === "custom" ? "必填（如 https://api.example.com/v1）" : "留空使用官方地址"}
        >
          <input
            className="input font-mono text-[11px]"
            value={form.baseUrl}
            onChange={(e) => set("baseUrl", e.target.value)}
            placeholder={DEFAULT_BASE_URLS[form.kind]}
          />
        </Field>

        <Field label="API Key" hint={initial ? "已保存于系统凭据管理器，留空表示不修改" : "保存后写入系统凭据管理器"}>
          <input
            className="input font-mono text-[11px]"
            type="password"
            value={form.apiKey}
            onChange={(e) => set("apiKey", e.target.value)}
            placeholder={initial?.hasKey ? "••••••••（已配置）" : "sk-..."}
          />
        </Field>

        <div className="grid grid-cols-2 gap-2">
          <Field label="文本模型">
            <input
              className="input font-mono text-[11px]"
              value={form.model}
              onChange={(e) => set("model", e.target.value)}
              list="pf-models"
              placeholder="gpt-4o-mini"
            />
          </Field>
          <Field label="图像模型">
            <input
              className="input font-mono text-[11px]"
              value={form.imageModel}
              onChange={(e) => set("imageModel", e.target.value)}
              list="pf-models"
              placeholder={DEFAULT_IMAGE_MODELS[form.kind]}
            />
          </Field>
        </div>
        <datalist id="pf-models">
          {models.map((m) => (
            <option key={m} value={m} />
          ))}
        </datalist>

        {form.kind === "custom" && (
          <Field label="请求格式" hint="决定调用协议">
            <select className="select w-full" value={form.requestFormat} onChange={(e) => set("requestFormat", e.target.value)}>
              <option value="openai">OpenAI 兼容（/images/generations）</option>
              <option value="gemini">Google Gemini 兼容（generateContent）</option>
            </select>
          </Field>
        )}

        <div>
          <div className="mb-1 flex items-center justify-between text-[11px] text-inkdim">
            <span>自定义请求头</span>
            <button
              type="button"
              className="btn btn-ghost h-5 px-1.5 text-[10px]"
              onClick={() => set("headers", [...form.headers, { key: "", value: "" }])}
            >
              <Icon name="plus" size={10} />
              添加
            </button>
          </div>
          {form.headers.length === 0 && (
            <div className="border border-dashed border-line px-2 py-1.5 text-[10px] text-inkfaint">
              无自定义请求头（Authorization / x-goog-api-key 会自动附加）
            </div>
          )}
          <div className="space-y-1">
            {form.headers.map((h, i) => (
              <div key={i} className="flex gap-1.5">
                <input
                  className="input font-mono text-[11px]"
                  value={h.key}
                  placeholder="请求头"
                  onChange={(e) => setHeader(i, { key: e.target.value })}
                />
                <input
                  className="input flex-1 font-mono text-[11px]"
                  value={h.value}
                  placeholder="值"
                  onChange={(e) => setHeader(i, { value: e.target.value })}
                />
                <button
                  type="button"
                  className="btn btn-ghost h-7 w-7 shrink-0 p-0"
                  onClick={() => set("headers", form.headers.filter((_, j) => j !== i))}
                >
                  <Icon name="close" size={11} />
                </button>
              </div>
            ))}
          </div>
        </div>

        {testMsg && (
          <div
            className={`border px-2 py-1.5 text-[11px] ${
              testMsg.startsWith("✓") ? "border-ok/40 bg-ok/5 text-ok" : "border-bad/40 bg-bad/5 text-bad"
            }`}
          >
            {testMsg}
          </div>
        )}
      </form>
    </Modal>
  );
}
