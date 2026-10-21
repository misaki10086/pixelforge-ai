import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { AssetImage } from "@/components/AssetImage";
import { Icon } from "@/components/ui/Icon";
import { Badge, EmptyState, Field, Panel, Spinner } from "@/components/ui/Panel";
import { GenerateAnimationModal } from "./GenerateAnimationModal";
import { RegenerateFrameModal } from "./RegenerateFrameModal";
import { animationApi, settingsApi } from "@/services";
import { useAnimationStore } from "@/stores/animationStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { ANIM_TYPE_OPTIONS, CANVAS_ZOOMS, FPS_OPTIONS, PLAY_MODES } from "@/utils/constants";
import { cn } from "@/utils/cn";
import { formatRelative } from "@/utils/format";
import type { SpriteSheetResult } from "@/types";

type PlayMode = (typeof PLAY_MODES)[number]["value"];

/** 帧间播放定时器间隔（ms） */
function frameDelay(fps: number): number {
  return Math.max(16, Math.round(1000 / Math.max(1, fps)));
}

export function AnimationPage() {
  const current = useProjectStore((s) => s.current);
  const { animations, detail, loading } = useAnimationStore();
  const load = useAnimationStore((s) => s.load);
  const open = useAnimationStore((s) => s.open);
  const refreshCurrent = useAnimationStore((s) => s.refreshCurrent);
  const pushToast = useUiStore((s) => s.pushToast);

  const [playing, setPlaying] = useState(false);
  const [playMode, setPlayMode] = useState<PlayMode>("loop");
  const [frameIdx, setFrameIdx] = useState(0);
  const [onion, setOnion] = useState(false);
  const [zoom, setZoom] = useState<number>(0); // 0 = 自适应
  const [fpsOverride, setFpsOverride] = useState<number | null>(null);
  const [durationInput, setDurationInput] = useState("");
  const [genModal, setGenModal] = useState(false);
  const [regenFrame, setRegenFrame] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Sprite Sheet 构建参数与结果
  const [cols, setCols] = useState(0);
  const [rows, setRows] = useState(1);
  const [padding, setPadding] = useState(0);
  const [margin, setMargin] = useState(0);
  const [sheet, setSheet] = useState<SpriteSheetResult | null>(null);

  const frames = useMemo(() => detail?.frames ?? [], [detail]);
  const anim = detail?.animation ?? null;
  const fps = fpsOverride ?? anim?.fps ?? 8;
  const safeIdx = Math.min(frameIdx, Math.max(0, frames.length - 1));

  // 项目切换 / 首次进入：加载动画列表并自动打开第一个
  const projectId = current?.id;
  useEffect(() => {
    if (projectId) {
      void load(projectId).then(() => {
        const first = useAnimationStore.getState().animations[0];
        if (first) void useAnimationStore.getState().open(first.id);
      });
    }
  }, [projectId, load]);

  useEffect(() => {
    setFrameIdx(0);
    setSheet(null);
    setCols(0);
    setFpsOverride(null);
  }, [anim?.id]);

  // 播放循环（loop / pingpong / once，参考 Aseprite · Pixelorama）
  const dirRef = useRef(1);
  const playRef = useRef<number | null>(null);
  useEffect(() => {
    if (!playing || frames.length < 2) {
      if (playRef.current) window.clearInterval(playRef.current);
      playRef.current = null;
      return;
    }
    playRef.current = window.setInterval(() => {
      setFrameIdx((i) => {
        const last = frames.length - 1;
        let next = i + dirRef.current;
        if (playMode === "pingpong") {
          if (next > last || next < 0) {
            dirRef.current *= -1;
            next = i + dirRef.current;
          }
        } else if (playMode === "once") {
          if (next > last) {
            setPlaying(false);
            return last;
          }
        } else {
          next = (next + frames.length) % frames.length;
        }
        return Math.max(0, Math.min(last, next));
      });
    }, frameDelay(fps));
    return () => {
      if (playRef.current) window.clearInterval(playRef.current);
      playRef.current = null;
    };
  }, [playing, fps, frames.length, playMode]);

  // 选中帧变化时清空时长输入
  useEffect(() => setDurationInput(""), [safeIdx]);

  const run = useCallback(
    async (fn: () => Promise<unknown>, okMsg?: string) => {
      setBusy(true);
      try {
        await fn();
        await refreshCurrent();
        if (okMsg) pushToast("success", okMsg);
      } catch (e) {
        pushToast("error", e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [refreshCurrent, pushToast]
  );

  // 快捷键：Space 播放暂停 · ←/→ 切帧 · Ctrl+D 复制帧（参考 Aseprite / Piskel）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (genModal || regenFrame) return;
      const t = e.target as HTMLElement | null;
      if (t && ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName)) return;
      if (!anim || frames.length === 0) return;
      if (e.code === "Space") {
        e.preventDefault();
        setPlaying((p) => !p);
      } else if (e.key === "ArrowLeft") {
        setPlaying(false);
        setFrameIdx((i) => (i - 1 + frames.length) % frames.length);
      } else if (e.key === "ArrowRight") {
        setPlaying(false);
        setFrameIdx((i) => (i + 1) % frames.length);
      } else if (e.ctrlKey && e.key.toLowerCase() === "d") {
        e.preventDefault();
        const f = frames[safeIdx];
        if (f) void run(() => animationApi.duplicateFrame(f.id), "已复制帧。");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [anim, frames, safeIdx, genModal, regenFrame, run]);

  const buildSheet = async () => {
    if (!anim) return;
    await run(async () => {
      const result = await animationApi.buildSheet(
        anim.id,
        cols > 0 ? cols : Math.max(1, frames.length),
        rows > 0 ? rows : 1,
        padding,
        margin
      );
      setSheet(result);
    }, "精灵表已生成。");
  };

  const exportSheet = async () => {
    if (!anim) return;
    const picked = await openDialog({ directory: true, title: "选择导出目录（sheet.png + sheet.json）" });
    if (typeof picked !== "string") return;
    await run(
      () =>
        animationApi.exportSheet(
          anim.id,
          cols > 0 ? cols : Math.max(1, frames.length),
          rows > 0 ? rows : 1,
          padding,
          margin,
          picked
        ),
      "精灵表（PNG + JSON）已导出。"
    );
  };

  const exportGif = async () => {
    if (!anim) return;
    const picked = await openDialog({ directory: true, title: "选择 GIF 导出目录" });
    if (typeof picked !== "string") return;
    await run(() => animationApi.exportGif(anim.id, picked), "GIF 已导出。");
  };

  if (!current) {
    return (
      <Panel className="m-2" scroll={false}>
        <EmptyState icon="projects" title="未打开项目" description="请先在「项目」页创建或打开一个项目。" />
      </Panel>
    );
  }

  return (
    <div className="flex h-full flex-col gap-2 p-2">
      <div className="flex min-h-0 flex-1 gap-2">
        {/* 左：动画列表 */}
        <div className="w-[210px] shrink-0">
          <Panel
            title={
              <>
                <Icon name="animation" size={12} />
                动画 ({animations.length})
              </>
            }
            actions={
              <button className="btn btn-primary h-5 px-1.5 text-[10px]" onClick={() => setGenModal(true)}>
                <Icon name="plus" size={10} />
                生成
              </button>
            }
            className="h-full"
            bodyClassName="p-1"
          >
            {loading && <div className="p-3 text-center text-[11px] text-inkfaint"><Spinner size={12} /></div>}
            {!loading && animations.length === 0 && (
              <div className="p-3 text-center text-[11px] leading-relaxed text-inkfaint">
                还没有动画。
                <br />
                可以选择已有角色生成 Idle / Walk / Attack 等动画。
              </div>
            )}
            <div className="space-y-1">
              {animations.map((a) => (
                <button
                  key={a.id}
                  className={cn(
                    "w-full border px-2 py-1.5 text-left",
                    anim?.id === a.id ? "border-accent/60 bg-accent/5" : "border-line bg-elevated/40 hover:border-linestrong"
                  )}
                  onClick={() => void open(a.id).catch((e) => pushToast("error", String(e)))}
                >
                  <div className="flex items-center justify-between text-[12px] text-ink">
                    <span className="truncate">{a.name}</span>
                    <Badge>{a.frameCount} 帧</Badge>
                  </div>
                  <div className="text-[10px] text-inkfaint">
                    {ANIM_TYPE_OPTIONS.find((t) => t.value === a.type)?.label ?? a.type} · {a.fps} FPS ·{" "}
                    {formatRelative(a.createdAt)}
                  </div>
                </button>
              ))}
            </div>
          </Panel>
        </div>

        {/* 中：Canvas + Timeline */}
        <div className="flex min-w-0 flex-1 flex-col gap-2">
          <Panel
            title={
              anim ? (
                <>
                  <Icon name="eye" size={12} />
                  {anim.name}
                  <Badge tone="accent">{ANIM_TYPE_OPTIONS.find((t) => t.value === anim.type)?.label}</Badge>
                </>
              ) : (
                "画布"
              )
            }
            actions={
              anim && (
                <div className="flex items-center gap-1">
                  <button className="btn btn-ghost h-6 px-2 text-[11px]" onClick={() => setPlaying((p) => !p)} title="播放 / 暂停（Space）">
                    <Icon name={playing ? "pause" : "play"} size={12} />
                    {playing ? "暂停" : "播放"}
                  </button>
                  <select
                    className="select h-6 text-[11px]"
                    value={playMode}
                    onChange={(e) => setPlayMode(e.target.value as PlayMode)}
                    title="播放模式"
                  >
                    {PLAY_MODES.map((m) => (
                      <option key={m.value} value={m.value}>
                        {m.label}
                      </option>
                    ))}
                  </select>
                  <select
                    className="select h-6 text-[11px]"
                    value={fps}
                    onChange={(e) => setFpsOverride(Number(e.target.value))}
                    title="FPS"
                  >
                    {FPS_OPTIONS.map((f) => (
                      <option key={f} value={f}>
                        {f} FPS
                      </option>
                    ))}
                  </select>
                  <select
                    className="select h-6 text-[11px]"
                    value={zoom}
                    onChange={(e) => setZoom(Number(e.target.value))}
                    title="画布缩放"
                  >
                    <option value={0}>适应</option>
                    {CANVAS_ZOOMS.map((z) => (
                      <option key={z} value={z}>
                        {z}×
                      </option>
                    ))}
                  </select>
                  <button
                    className={cn("btn h-6 px-2 text-[11px]", onion && "border-accent/70 text-accent")}
                    onClick={() => setOnion((v) => !v)}
                    title="洋葱皮：前帧红色叠影 / 后帧蓝色叠影（Aseprite 习惯）"
                  >
                    <Icon name="layers" size={12} />
                    洋葱皮
                  </button>
                  <button
                    className="btn btn-ghost h-6 px-2 text-[11px] text-bad"
                    onClick={() => {
                      if (window.confirm(`确认删除动画「${anim.name}」？帧文件将一并删除。`)) {
                        void run(() => animationApi.delete(anim.id), "动画已删除。");
                      }
                    }}
                  >
                    <Icon name="trash" size={12} />
                  </button>
                </div>
              )
            }
            className="min-h-0 flex-1"
            scroll={false}
          >
            {!anim ? (
              <EmptyState
                icon="animation"
                title="选择或生成一个动画"
                description="从左侧选择动画，或点击「生成」从已有角色创建 Idle / Walk / Run / Attack / Skill 等动画。"
                actions={
                  <button className="btn btn-primary" onClick={() => setGenModal(true)}>
                    <Icon name="generate" size={13} />
                    从角色生成动画
                  </button>
                }
              />
            ) : (
              <div className="checkerboard relative flex h-full min-h-[220px] items-center justify-center overflow-hidden">
                {/* 洋葱皮：前帧红色调 / 后帧蓝色调（Aseprite 惯例），仅缩放模式下隐藏避免错位 */}
                {onion && frames.length > 1 && zoom === 0 && (
                  <>
                    <img
                      src={convertFileSrc(frames[(safeIdx - 1 + frames.length) % frames.length].imagePath)}
                      className="pixelated pointer-events-none absolute max-h-[80%] max-w-[80%] opacity-30"
                      style={{ transform: "translateX(-12%)", filter: "sepia(1) saturate(5) hue-rotate(-45deg)" }}
                      alt="prev"
                    />
                    <img
                      src={convertFileSrc(frames[(safeIdx + 1) % frames.length].imagePath)}
                      className="pixelated pointer-events-none absolute max-h-[80%] max-w-[80%] opacity-30"
                      style={{ transform: "translateX(12%)", filter: "sepia(1) saturate(5) hue-rotate(150deg)" }}
                      alt="next"
                    />
                  </>
                )}
                {frames[safeIdx] && (
                  <img
                    key={frames[safeIdx].imagePath + safeIdx}
                    src={convertFileSrc(frames[safeIdx].imagePath)}
                    className={zoom > 0 ? "pixelated" : "pixelated max-h-[85%] max-w-[85%]"}
                    style={
                      zoom > 0
                        ? { width: frames[safeIdx].width * zoom, height: frames[safeIdx].height * zoom }
                        : undefined
                    }
                    alt={`frame ${safeIdx + 1}`}
                  />
                )}
                <span className="absolute bottom-1.5 right-2 border border-line bg-base/80 px-1.5 py-0.5 font-mono text-[10px] text-inkdim">
                  {safeIdx + 1} / {frames.length}
                  {zoom > 0 && ` · ${zoom}×`}
                </span>
                <span className="absolute bottom-1.5 left-2 hidden font-mono text-[9px] text-inkfaint md:block">
                  Space 播放 · ←/→ 切帧 · Ctrl+D 复制帧
                </span>
              </div>
            )}
          </Panel>

          {/* Timeline */}
          {anim && (
            <Panel
              title={
                <>
                  <Icon name="clock" size={12} />
                  时间轴
                </>
              }
              actions={
                <div className="flex items-center gap-1">
                  <button
                    className="btn h-6 px-2 text-[11px]"
                    disabled={busy}
                    onClick={() => {
                      void openDialog({ title: "添加帧图片", multiple: false, filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "gif", "bmp", "webp"] }] }).then(
                        (p) => {
                          if (typeof p === "string")
                            void run(() => animationApi.addFrameFromFile(anim.id, p), "已添加帧。");
                        }
                      );
                    }}
                  >
                    <Icon name="plus" size={12} />
                    添加帧
                  </button>
                  <button className="btn btn-ghost h-6 px-1.5 text-[11px]" onClick={() => setPlaying((p) => !p)}>
                    <Icon name={playing ? "pause" : "play"} size={11} />
                  </button>
                </div>
              }
              className="h-[118px] shrink-0"
              bodyClassName="flex items-center gap-1.5 overflow-x-auto p-1.5"
            >
              {frames.map((f, i) => (
                <button
                  key={f.id}
                  className={cn(
                    "group relative shrink-0 border p-0.5",
                    i === safeIdx ? "border-accent" : "border-line hover:border-linestrong"
                  )}
                  onClick={() => {
                    setPlaying(false);
                    setFrameIdx(i);
                  }}
                  title={`第 ${i + 1} 帧 · ${f.width}×${f.height} · ${f.durationMs}ms`}
                >
                  <AssetImage path={f.imagePath} alt={`f${i + 1}`} className="h-[72px] w-[72px]" />
                  <span className="absolute left-0.5 top-0.5 bg-base/85 px-1 font-mono text-[9px] text-inkdim">
                    {i + 1}
                  </span>
                  {/* 选中帧操作 */}
                  {i === safeIdx && (
                    <span className="absolute -bottom-0.5 right-0.5 flex gap-0.5">
                      <span
                        className="flex h-4 w-4 items-center justify-center border border-line bg-panel text-inkdim hover:text-accent"
                        title="上移（重排）"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (i > 0) void run(() => animationApi.moveFrame(anim.id, i, i - 1));
                        }}
                      >
                        ◀
                      </span>
                      <span
                        className="flex h-4 w-4 items-center justify-center border border-line bg-panel text-inkdim hover:text-accent"
                        title="下移（重排）"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (i < frames.length - 1) void run(() => animationApi.moveFrame(anim.id, i, i + 1));
                        }}
                      >
                        ▶
                      </span>
                      <span
                        className="flex h-4 w-4 items-center justify-center border border-line bg-panel text-inkdim hover:text-accent"
                        title="复制帧"
                        onClick={(e) => {
                          e.stopPropagation();
                          void run(() => animationApi.duplicateFrame(f.id), "已复制帧。");
                        }}
                      >
                        <Icon name="copy" size={9} />
                      </span>
                      <span
                        className="flex h-4 w-4 items-center justify-center border border-line bg-panel text-inkdim hover:text-warn"
                        title="AI 重生成此帧"
                        onClick={(e) => {
                          e.stopPropagation();
                          setRegenFrame(f.id);
                        }}
                      >
                        <Icon name="wand" size={9} />
                      </span>
                      <span
                        className="flex h-4 w-4 items-center justify-center border border-line bg-panel text-inkdim hover:text-bad"
                        title="删除帧"
                        onClick={(e) => {
                          e.stopPropagation();
                          void run(() => animationApi.deleteFrame(f.id), "帧已删除。");
                        }}
                      >
                        <Icon name="close" size={9} />
                      </span>
                    </span>
                  )}
                </button>
              ))}
              {frames.length === 0 && (
                <div className="px-4 text-[11px] text-inkfaint">暂无帧，点击「添加帧」导入图片。</div>
              )}
            </Panel>
          )}
        </div>

        {/* 右：Inspector */}
        <div className="w-[240px] shrink-0">
          <Panel
            title={
              <>
                <Icon name="settings" size={12} />
                属性面板
              </>
            }
            className="h-full"
            bodyClassName="space-y-3 p-2.5"
          >
            {anim && (
              <>
                <div>
                  <div className="mb-1 text-[11px] text-inkdim">动画属性</div>
                  <div className="space-y-1.5">
                    <Field label="名称">
                      <input
                        className="input"
                        defaultValue={anim.name}
                        key={anim.id + anim.updatedAt}
                        onBlur={(e) => {
                          if (e.target.value.trim() && e.target.value !== anim.name) {
                            void run(
                              () => animationApi.update(anim.id, e.target.value, fps, anim.loop),
                              "动画属性已更新。"
                            );
                          }
                        }}
                      />
                    </Field>
                    <div className="grid grid-cols-2 gap-1.5">
                      <Field label="FPS">
                        <select
                          className="select w-full"
                          value={anim.fps}
                          onChange={(e) =>
                            void run(
                              () => animationApi.update(anim.id, anim.name, Number(e.target.value), anim.loop),
                              "FPS 已更新。"
                            )
                          }
                        >
                          {FPS_OPTIONS.map((f) => (
                            <option key={f} value={f}>
                              {f}
                            </option>
                          ))}
                        </select>
                      </Field>
                      <Field label="循环">
                        <select
                          className="select w-full"
                          value={anim.loop ? "1" : "0"}
                          onChange={(e) =>
                            void run(
                              () =>
                                animationApi.update(anim.id, anim.name, anim.fps, e.target.value === "1"),
                              "循环设置已更新。"
                            )
                          }
                        >
                          <option value="1">循环</option>
                          <option value="0">单次</option>
                        </select>
                      </Field>
                    </div>
                  </div>
                </div>

                <div className="border border-line bg-elevated/40 px-2 py-1.5 text-[10px] leading-relaxed text-inkfaint">
                  <div className="flex justify-between">
                    <span>帧数</span>
                    <span className="font-mono text-inkdim">{frames.length}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>帧尺寸</span>
                    <span className="font-mono text-inkdim">
                      {frames[0] ? `${frames[0].width}×${frames[0].height}` : "—"}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span>来源</span>
                    <span className="truncate text-inkdim">
                      {anim.sourceAssetId ? "角色资产参考" : "文字描述生成"}
                    </span>
                  </div>
                </div>

                {/* Sprite Sheet Builder */}
                <div>
                  <div className="mb-1 text-[11px] text-inkdim">精灵表构建</div>
                  <div className="grid grid-cols-2 gap-1.5">
                    <Field label="列">
                      <input
                        className="input font-mono"
                        type="number"
                        min={1}
                        value={cols || Math.max(1, frames.length)}
                        onChange={(e) => setCols(Math.max(1, Number(e.target.value)))}
                      />
                    </Field>
                    <Field label="行">
                      <input
                        className="input font-mono"
                        type="number"
                        min={1}
                        value={rows}
                        onChange={(e) => setRows(Math.max(1, Number(e.target.value)))}
                      />
                    </Field>
                    <Field label="间距">
                      <input
                        className="input font-mono"
                        type="number"
                        min={0}
                        value={padding}
                        onChange={(e) => setPadding(Math.max(0, Number(e.target.value)))}
                      />
                    </Field>
                    <Field label="边距">
                      <input
                        className="input font-mono"
                        type="number"
                        min={0}
                        value={margin}
                        onChange={(e) => setMargin(Math.max(0, Number(e.target.value)))}
                      />
                    </Field>
                  </div>
                  <div className="mt-1.5 flex gap-1.5">
                    <button className="btn flex-1 px-1 text-[11px]" disabled={busy || frames.length === 0} onClick={() => void buildSheet()}>
                      <Icon name="layers" size={11} />
                      生成预览
                    </button>
                    <button className="btn flex-1 px-1 text-[11px]" disabled={busy || frames.length === 0} onClick={() => void exportSheet()}>
                      <Icon name="export" size={11} />
                      导出 PNG+JSON
                    </button>
                  </div>
                  {sheet && (
                    <div className="mt-1.5 border border-line">
                      <img
                        src={convertFileSrc(sheet.pngPath)}
                        className="checkerboard pixelated h-auto w-full"
                        alt="sprite sheet"
                      />
                      <div className="border-t border-line px-1.5 py-1 text-[9px] text-inkfaint">
                        {sheet.width}×{sheet.height} · {sheet.columns}×{sheet.rows} · 单元 {sheet.cellWidth}×
                        {sheet.cellHeight}
                      </div>
                    </div>
                  )}
                </div>

                {/* GIF 导出 */}
                <div className="grid grid-cols-2 gap-1.5">
                  <button
                    className="btn"
                    disabled={busy || frames.length === 0}
                    onClick={() => void exportGif()}
                  >
                    <Icon name="export" size={12} />
                    导出 GIF
                  </button>
                  <button
                    className="btn"
                    disabled={busy || frames.length === 0}
                    onClick={async () => {
                      const picked = await openDialog({ directory: true, title: "选择 Godot 导出目录" });
                      if (typeof picked === "string") {
                        await run(
                          () => animationApi.exportGodot(anim.id, picked),
                          "Godot SpriteFrames（.tres + PNG）已导出。"
                        );
                      }
                    }}
                    title="生成 Godot 4 可直接使用的 SpriteFrames 资源"
                  >
                    <Icon name="export" size={12} />
                    Godot .tres
                  </button>
                </div>

                {/* 选中帧时长（逐帧变速，参考 Aseprite） */}
                {frames[safeIdx] && (
                  <Field label={`第 ${safeIdx + 1} 帧时长 (ms)`} hint="当前值见占位符">
                    <div className="flex gap-1.5">
                      <input
                        className="input font-mono"
                        value={durationInput}
                        placeholder={String(frames[safeIdx].durationMs)}
                        onChange={(e) => setDurationInput(e.target.value.replace(/[^0-9]/g, ""))}
                      />
                      <button
                        className="btn shrink-0"
                        disabled={!durationInput || busy}
                        onClick={() =>
                          void run(
                            () =>
                              animationApi.updateFrameDuration(frames[safeIdx].id, Number(durationInput)),
                            "帧时长已更新。"
                          )
                        }
                      >
                        应用
                      </button>
                    </div>
                  </Field>
                )}

                {/* 文件位置 */}
                <div className="border border-line bg-elevated/40 px-2 py-1.5">
                  <div className="mb-1 text-[10px] uppercase tracking-wider text-inkfaint">所在文件夹</div>
                  <code className="block break-all text-[9px] leading-relaxed text-inkdim">
                    {"animations/" + anim.id + "/frames/"}
                  </code>
                  <button
                    className="btn mt-1.5 h-5 w-full px-1 text-[10px]"
                    onClick={() =>
                      void settingsApi.reveal(current.path + "\\animations\\" + anim.id).catch((e) =>
                        pushToast("error", e instanceof Error ? e.message : String(e))
                      )
                    }
                  >
                    <Icon name="folder" size={10} />
                    在资源管理器中打开
                  </button>
                </div>

                {anim.prompt && (
                  <div>
                    <div className="mb-1 text-[11px] text-inkdim">生成提示词</div>
                    <pre className="max-h-[110px] overflow-y-auto whitespace-pre-wrap break-words border border-line bg-input px-2 py-1.5 font-mono text-[9px] leading-relaxed text-inkdim">
                      {anim.prompt}
                    </pre>
                  </div>
                )}
              </>
            )}
            {!anim && <div className="text-center text-[11px] text-inkfaint">未选择动画</div>}
          </Panel>
        </div>
      </div>

      {genModal && <GenerateAnimationModal onClose={() => setGenModal(false)} />}
      {regenFrame && anim && (
        <RegenerateFrameModal animation={anim} frameId={regenFrame} onClose={() => setRegenFrame(null)} />
      )}
    </div>
  );
}
