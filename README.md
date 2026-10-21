# PixelForge AI — AI 游戏资产工作台

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/misaki10086/pixelforge-ai?color=blue)](https://github.com/misaki10086/pixelforge-ai/releases/latest)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=white)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D6?logo=windows&logoColor=white)](#)

面向独立游戏开发者的 AI 游戏资产生成工作台（Windows 11 桌面应用）。

核心理念不是「AI 画图网站」，而是完整的生产管线：

**AI 生成 → 资产处理 → 动画 → 编辑 → 管理 → 游戏引擎导出**

## 下载

不用编译，直接下 exe：**[最新版本 Releases](https://github.com/misaki10086/pixelforge-ai/releases/latest)**

| 文件 | 说明 |
| --- | --- |
| `PixelForge-AI_x.y.z_x64-setup.exe` | 安装版（推荐）：按当前用户安装，免管理员权限 |
| `PixelForge-AI_x.y.z_x64-portable.zip` | 免安装版：解压后双击 `PixelForge AI.exe` 即可 |

系统要求：Windows 10 / 11 64 位；需要 WebView2 运行时（Windows 11 已内置）。
本仓库不内置任何 API Key，首次使用需在「设置 → AI 服务商」里填入你自己的 Key。

## 截图

> 截图待补充：把图片放进 `docs/screenshots/`，命名 01-dashboard / 02-generate / 03-assets / 04-animation（.png），
> 然后取消下方注释即可。

<!--
| 概览 | 生成 |
| --- | --- |
| ![概览](docs/screenshots/01-dashboard.png) | ![生成](docs/screenshots/02-generate.png) |

| 资产库 | 动画时间轴 |
| --- | --- |
| ![资产库](docs/screenshots/03-assets.png) | ![动画](docs/screenshots/04-animation.png) |
-->

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2（Rust） |
| 前端 | React 18 + TypeScript + Tailwind CSS + Zustand |
| 数据库 | SQLite（rusqlite，WAL 模式） |
| AI 调用 | Rust 侧 Provider 适配器（reqwest），前端零 SDK 依赖 |
| 凭据 | Windows 凭据管理器（keyring），API Key 不落盘明文 |

## 架构约束

```
React UI ──► Service (invoke) ──► Tauri Command ──► ┌ database (SQLite 元数据)
                                                    ├ filesystem (项目目录 / 图片)
                                                    ├ providers (ProviderManager → Adapter)
                                                    ├ tasks (异步任务队列)
                                                    └ export (Generic / Godot / Unity)
```

- 前端**禁止**直接访问文件系统、SQLite 或任何 AI SDK，只能通过 `src/services`。
- Provider 与 UI 完全解耦：`AIProvider` trait 统一 `generate_image / edit_image / health_check / list_models`，
  内置 OpenAI、Gemini、Custom（OpenAI 兼容 / Gemini 兼容，可自定义 Base URL、Headers、模型）三种适配器。
- 每次生成/编辑都创建 `asset_versions` 记录，**永不覆盖旧版本**。
- 数据库只存元数据；图片按 `assets/{类型}/`、缩略图按 `thumbnails/` 存放于项目目录。

## 项目目录结构

```
project/
  project.json          # 项目元数据
  assets/
    characters/ enemies/ npc/ items/ weapons/
    environment/ tiles/ vfx/ ui/
    raw/                # 未处理原始输出
  animations/ exports/ references/ thumbnails/
```

## 开发

```bash
npm install
npm run tauri dev      # 启动桌面应用（开发模式）
npm run typecheck      # tsc --noEmit
npm run lint           # eslint
npm test               # vitest
cd src-tauri && cargo test
npm run tauri build    # 构建 NSIS 安装包（Windows）
```

首次启动：在「设置 → Providers」添加 Provider（OpenAI / Gemini / 自定义 OpenAI 兼容接口），
API Key 会写入 Windows 凭据管理器；在「Projects」页创建项目；然后即可在「Generate」页生成角色。

## 阶段状态

### ✅ 第一阶段（已完成）
- Tauri 2 + React + TypeScript + Tailwind + SQLite 持久化
- 深色专业工具主题（含 Light 辅助主题）
- Dashboard（统计 / 最近资产 / 最近任务 / Provider 状态）
- 项目系统（创建 / 打开 / 重命名 / 复制 / 删除，完整目录结构 + project.json）
- Provider Manager（OpenAI / Gemini / Custom；测试连接、获取模型列表、凭据管理器存 Key）
- Character Generator（描述 / 风格 / 分辨率 / 视角 / 朝向 / 调色板 / 描边 / 动画帧数 / 高级设置）
- 异步任务队列（进度 / 取消 / 重试 / 失败原因，右下角任务 Dock）
- Asset Gallery（Grid / Compact / List、按类型与 Provider 过滤、搜索、本地图片导入）
- 资产版本（还原为新版本 / 对比 / 删除 / 元数据编辑）
- 导出：PNG + JSON 元数据；Godot / Unity 基础导出（PNG + 引擎侧车文件）

### ✅ 第二阶段（已完成）
- **动画系统（真实可用）**：
  - **从角色生成动画**：选择已有角色资产作为严格参考，走图生图（edit_image）生成
    Idle / Walk / Run / Attack / Hurt / Death / Jump / Skill 动画，保持角色外观一致
  - **逐帧动作逻辑模板（skill）**：每种动画类型内置关键帧逻辑
    （如 Walk = 接触步 → 过渡高点 → 反向接触步 → 过渡低点；Attack = 蓄力 → 挥击 → 命中 → 回收），
    多余帧自动在关键帧间过渡，保证动作在逻辑上成立
  - **自动切帧建档**：生成结果自动切分为独立帧，存入 `animations/{id}/frames/`
  - **时间线**：帧缩略图条、选择、添加（本地图片）、复制、删除、左右重排
  - **单帧 AI 重生成**：只重画选中帧（以当前帧为参考 + 动作逻辑），旧帧自动备份到 `history/`
  - **播放控制**：循环 / 往返（Ping-Pong）/ 单次三种模式，FPS 1–60 预设
  - **Onion Skin**：前帧红色叠影 / 后帧蓝色叠影（Aseprite 惯例）
  - **快捷键**：Space 播放暂停 · ←/→ 切帧 · Ctrl+D 复制帧
  - **逐帧时长**：可单独调整每帧持续时间实现逐帧变速
  - **画布缩放**：适应 / 1× / 2× / 4× / 8×
- **Sprite Sheet Builder**：列 × 行 × Padding × Margin 实时构建预览，
  导出 `sheet.png` + `sheet.json`（帧 name/x/y/w/h/duration）
- **智能切帧**：按透明间隙检测帧边界（帧内 1–2px 断缝自动合并），
  检测失败自动回退均分 —— AI 输出帧距不均时远比均分可靠
- **GIF 导出**：帧序列 + FPS → 动画 GIF
- **Sprite Sheet 拆分**：资产详情页可直接把横向 Sheet 拆分为动画（自动 / 网格两种模式）
- **颜色 / 风格**：
  - 中位切分调色板提取（8 / 16 / 32 / 64 色，可复制 hex 用于跨资产风格统一）
  - 预设调色板一键套用：PICO-8 / Sweetie 16 / DawnBringer 16 / Endesga 32 / Game Boy DMG / CGA
  - 调色板量化（可选 Bayer 4×4 抖动）生成新版本
  - **Palette-Locked 生成**：选择带色值的调色板时自动把精确 hex 列表注入提示词
  - Pixelize（分辨率归一化 + 调色板缩减 + 抖动）一步处理为新版本
- **Godot 4 SpriteFrames 导出**：`sheet.png` + `.tres`（AtlasTexture 子资源），
  可直接拖入 Godot 挂到 AnimatedSprite2D
- **文件夹逻辑**：资产按 `assets/{类型}/`、动画按 `animations/{id}/{frames|history|sheet}` 组织，
  资产详情与动画 Inspector 均显示所在目录并支持一键在资源管理器中打开

### 🚧 后续阶段
- 第三阶段：AI Edit（含 Mask）、参考图（图片级）、Style Library、Tile / VFX / Environment 生成器
- 第四阶段：Unity 完整导出（Slice 配置）、Style Consistency 全自动、高级动画工具（多层 / Tag）

未实现的高级功能在 UI 中均以 **Coming Soon** 标注。

## 设计参考与致谢

本项目在打磨交互与算法时参考了以下开源项目 / 社区成果：

| 参考 | 借鉴点 |
| --- | --- |
| [Pixelorama](https://github.com/Orama-Interactive/Pixelorama) | 播放模式（Ping-Pong 循环）、洋葱皮可配思路 |
| [Aseprite](https://github.com/aseprite/aseprite)（源码可用） | 逐帧时长、洋葱皮前红后蓝惯例、快捷键习惯 |
| [Retro Diffusion / pixel-art-fixer](https://github.com/Retro-Diffusion/pixel-art-fixer)（MIT） | 智能切帧（间隙检测优先、均分兜底）、Palette-Locked 生成思路 |
| [Godot 文档](https://docs.godotengine.org/en/stable/classes/class_atlastexture.html) | SpriteFrames `.tres` / AtlasTexture 导出格式 |
| Lospec 社区调色板 | PICO-8（© Lexaloffle）、Sweetie 16（© GrafxKid）、DB16（© DawnBringer）、EDG32（© Endesga） |

## 安全

- API Key 仅存于 Windows 凭据管理器（服务名 `PixelForge AI`）；
- 不写入 SQLite、不写入前端源码、不写入任何 JSON/配置文件；
- `.gitignore` 已排除 `.env` 与密钥文件。

## 许可证

本项目基于 [MIT License](LICENSE) 开源，Copyright (c) 2026 misaki10086。

README 中「设计参考与致谢」所列的第三方项目、调色板与格式规范，版权归各自作者所有，
本项目仅借鉴其交互惯例与算法思路，未复制其源代码。
