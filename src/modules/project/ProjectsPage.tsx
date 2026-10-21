import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Icon } from "@/components/ui/Icon";
import { Badge, EmptyState, Panel } from "@/components/ui/Panel";
import { Modal } from "@/components/ui/Modal";
import { Field } from "@/components/ui/Panel";
import { projectApi } from "@/services";
import { useProjectStore } from "@/stores/projectStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useUiStore } from "@/stores/uiStore";
import { formatDate } from "@/utils/format";
import type { Project } from "@/types";

type DialogState =
  | { kind: "create" }
  | { kind: "rename"; project: Project }
  | { kind: "delete"; project: Project }
  | null;

function ProjectForm({
  initialName,
  initialDesc,
  onSubmit,
  onCancel,
  submitLabel,
}: {
  initialName?: string;
  initialDesc?: string;
  onSubmit: (name: string, desc: string, baseDir?: string) => Promise<void>;
  onCancel: () => void;
  submitLabel: string;
}) {
  const [name, setName] = useState(initialName ?? "");
  const [desc, setDesc] = useState(initialDesc ?? "");
  const [baseDir, setBaseDir] = useState<string | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  const projectDir = useSettingsStore((s) => s.values["project_dir"]);

  const pickDir = async () => {
    const picked = await openDialog({ directory: true, title: "选择项目存放目录" });
    if (typeof picked === "string") setBaseDir(picked);
  };

  return (
    <form
      className="space-y-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (busy) return;
        setBusy(true);
        onSubmit(name, desc, baseDir).finally(() => setBusy(false));
      }}
    >
      <Field label="项目名称" hint="必填">
        <input
          className="input"
          value={name}
          autoFocus
          onChange={(e) => setName(e.target.value)}
          placeholder="例如：我的 RPG"
        />
      </Field>
      <Field label="描述">
        <input
          className="input"
          value={desc}
          onChange={(e) => setDesc(e.target.value)}
          placeholder="一句话描述这个游戏（可选）"
        />
      </Field>
      {!initialName && (
        <Field label="存放位置" hint={baseDir ?? projectDir ?? "默认目录"}>
          <div className="flex gap-1.5">
            <button type="button" className="btn" onClick={() => void pickDir()}>
              <Icon name="folder" size={13} />
              选择目录
            </button>
            {baseDir && (
              <button type="button" className="btn" onClick={() => setBaseDir(undefined)}>
                恢复默认
              </button>
            )}
          </div>
        </Field>
      )}
      <div className="flex justify-end gap-2 pt-1">
        <button type="button" className="btn" onClick={onCancel}>
          取消
        </button>
        <button type="submit" className="btn btn-primary" disabled={busy || !name.trim()}>
          {submitLabel}
        </button>
      </div>
    </form>
  );
}

export function ProjectsPage() {
  const projects = useProjectStore((s) => s.projects);
  const current = useProjectStore((s) => s.current);
  const openProject = useProjectStore((s) => s.openProject);
  const loadProjects = useProjectStore((s) => s.loadProjects);
  const pushToast = useUiStore((s) => s.pushToast);
  const [dialog, setDialog] = useState<DialogState>(null);

  const run = async (fn: () => Promise<unknown>, okMsg: string) => {
    try {
      await fn();
      await loadProjects();
      pushToast("success", okMsg);
      setDialog(null);
    } catch (e) {
      pushToast("error", e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex h-full flex-col gap-2 p-2">
      <Panel
        title={
          <>
            <Icon name="projects" size={12} />
            项目管理
          </>
        }
        actions={
          <button className="btn btn-primary h-6 px-2 text-[11px]" onClick={() => setDialog({ kind: "create" })}>
            <Icon name="plus" size={12} />
            新建项目
          </button>
        }
        className="flex-1"
        bodyClassName="p-2"
      >
        {projects.length === 0 ? (
          <EmptyState
            icon="projects"
            title="还没有项目"
            description="一个项目对应一个游戏。所有资产、生成记录与导出文件都保存在项目目录内。"
            actions={
              <button className="btn btn-primary" onClick={() => setDialog({ kind: "create" })}>
                <Icon name="plus" size={13} />
                新建第一个项目
              </button>
            }
          />
        ) : (
          <table className="w-full border-collapse text-[12px]">
            <thead>
              <tr className="border-b border-line text-left text-[10px] uppercase tracking-wider text-inkfaint">
                <th className="px-2 py-1.5 font-semibold">名称</th>
                <th className="px-2 py-1.5 font-semibold">路径</th>
                <th className="px-2 py-1.5 font-semibold text-right">资产</th>
                <th className="px-2 py-1.5 font-semibold">创建时间</th>
                <th className="px-2 py-1.5 font-semibold">最后打开</th>
                <th className="px-2 py-1.5 font-semibold text-right">操作</th>
              </tr>
            </thead>
            <tbody>
              {projects.map((p) => (
                <tr
                  key={p.id}
                  className={`border-b border-line/50 hover:bg-hover ${current?.id === p.id ? "bg-selbg/50" : ""}`}
                >
                  <td className="px-2 py-1.5">
                    <button
                      className="flex items-center gap-1.5 text-ink hover:text-accent"
                      onClick={() =>
                        void openProject(p.id).catch((e) =>
                          pushToast("error", e instanceof Error ? e.message : String(e))
                        )
                      }
                      title="打开项目"
                    >
                      <Icon name="folder" size={13} className={current?.id === p.id ? "text-accent" : "text-inkfaint"} />
                      {p.name}
                    </button>
                    {current?.id === p.id && <Badge tone="accent" className="ml-2">当前</Badge>}
                    {p.description && (
                      <div className="text-[10px] text-inkfaint">{p.description}</div>
                    )}
                  </td>
                  <td className="max-w-[280px] truncate px-2 py-1.5 font-mono text-[10px] text-inkdim" title={p.path}>
                    {p.path}
                  </td>
                  <td className="px-2 py-1.5 text-right font-mono">{p.assetCount}</td>
                  <td className="px-2 py-1.5 text-inkdim">{formatDate(p.createdAt)}</td>
                  <td className="px-2 py-1.5 text-inkdim">{formatDate(p.lastOpenedAt)}</td>
                  <td className="px-2 py-1.5">
                    <div className="flex justify-end gap-1">
                      <button
                        className="btn btn-ghost h-6 w-6 p-0"
                        title="打开"
                        onClick={() =>
                          void openProject(p.id).catch((e) =>
                            pushToast("error", e instanceof Error ? e.message : String(e))
                          )
                        }
                      >
                        <Icon name="external" size={12} />
                      </button>
                      <button
                        className="btn btn-ghost h-6 w-6 p-0"
                        title="重命名"
                        onClick={() => setDialog({ kind: "rename", project: p })}
                      >
                        <Icon name="editor" size={12} />
                      </button>
                      <button
                        className="btn btn-ghost h-6 w-6 p-0"
                        title="复制项目"
                        onClick={() =>
                          void run(() => projectApi.duplicate(p.id), `已创建项目副本：${p.name} 副本`)
                        }
                      >
                        <Icon name="copy" size={12} />
                      </button>
                      <button
                        className="btn btn-ghost h-6 w-6 p-0 text-bad"
                        title="删除项目"
                        onClick={() => setDialog({ kind: "delete", project: p })}
                      >
                        <Icon name="trash" size={12} />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>

      {dialog?.kind === "create" && (
        <Modal title="新建项目" onClose={() => setDialog(null)} width={480}>
          <ProjectForm
            submitLabel="创建"
            onCancel={() => setDialog(null)}
            onSubmit={(name, desc, baseDir) =>
              run(
                () =>
                  projectApi.create(name, desc, baseDir).then((p) => useProjectStore.getState().openProject(p.id)),
                "项目创建成功。"
              )
            }
          />
        </Modal>
      )}

      {dialog?.kind === "rename" && (
        <Modal title={`重命名「${dialog.project.name}」`} onClose={() => setDialog(null)} width={440}>
          <ProjectForm
            initialName={dialog.project.name}
            initialDesc={dialog.project.description}
            submitLabel="保存"
            onCancel={() => setDialog(null)}
            onSubmit={(name) => run(() => projectApi.rename(dialog.project.id, name), "项目已重命名。")}
          />
        </Modal>
      )}

      {dialog?.kind === "delete" && (
        <Modal title={`删除项目「${dialog.project.name}」`} onClose={() => setDialog(null)} width={440}>
          <div className="space-y-3 text-[12px] text-inkdim">
            <div className="flex items-start gap-2 border border-bad/40 bg-bad/10 px-2.5 py-2 text-bad">
              <Icon name="alert" size={14} className="mt-0.5" />
              此操作不可撤销。删除后数据库中的资产记录将一并移除。
            </div>
            <p>项目目录：</p>
            <p className="break-all border border-line bg-input px-2 py-1.5 font-mono text-[10px] text-ink">
              {dialog.project.path}
            </p>
            <p>是否同时删除磁盘上的项目文件（图片、导出等）？</p>
            <div className="flex justify-end gap-2 pt-1">
              <button className="btn" onClick={() => setDialog(null)}>
                取消
              </button>
              <button
                className="btn"
                onClick={() => run(() => projectApi.delete(dialog.project.id, false), "项目记录已删除（保留文件）。")}
              >
                仅删除记录
              </button>
              <button
                className="btn btn-danger"
                onClick={() => run(() => projectApi.delete(dialog.project.id, true), "项目与文件已删除。")}
              >
                <Icon name="trash" size={12} />
                删除全部
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
