# Blender Link 工具箱 — Codex 交接文档

> 更新时间：2026-07-21  
> 仓库：`D:\Development\tauri\blender_addon_link`  
> 远端：`origin` → `https://github.com/xmx-emm/blender_addon_link.git`（分支 `master`）

---

## 1. 项目概览

| 项 | 内容 |
| --- | --- |
| 产品名 | **Blender Link 工具箱** |
| 定位 | Windows 桌面小工具：多版本 Blender 插件 junction 链接 + 启动分析 + .blend 分析 + 渲染队列 + 设置/清理 |
| 技术栈 | **Tauri 2** + **Vue 3** + **TypeScript** + **Vuetify 3** + **Pinia**（`pinia-plugin-persistedstate` v4） |
| **当前版本号** | **1.0.0**（`package.json` / `Cargo.toml` / `tauri.conf.json` / 设置页关于） |
| UI 抬头 | 已对齐本机 `D:\Development\tauri\mxtools` 的 `AppTopBar`：`decorations: false` + 自定义顶栏（拖拽 / 最小化 / 最大化 / 关闭） |

### 目录要点

- Rust：`src-tauri/src/` — `lib.rs`、`link.rs`、`detect.rs`、`blend.rs`、`startup.rs`、`render.rs`、`maintenance.rs`、`procutil.rs`
- 前端：`src/` — `App.vue`、`components/AppTopBar.vue`、`stores.ts`、`data.ts`、`pages/*`
- UI：全中文；`card-soft`；反馈走 `ui.ok` / `ui.error`

---

## 2. 已完成（可运行基线）

### v0.2.0 两轮（已 commit + push）

- Commit `34c7526`：五页架构、junction、探测、.blend 解析、启动分析、渲染队列、清理/孤立数据等
- 第二轮：插件元信息、磁盘清理、任务编辑/断点续渲、系统通知、防休眠、窗口状态记忆

### 半成品入库（已 commit + push）

- Commit `be6db7d` / `f7ea97b`：`HANDOFF.md`、新图标、`render.rs` 并行后端 WIP、`maintenance.rs` 迁移/解包 WIP  
  ⚠ **前端尚未接线**，`lib.rs` 也未注册新 command；直接跑「取消渲染」会坏。

### 本轮已改（工作区 / 待 commit）

- 版本升到 **1.0.0**
- 参考 mxtools 加自定义顶栏：`src/components/AppTopBar.vue`，`App.vue` 改为顶栏 + 左侧 rail + 内容区布局
- `tauri.conf.json`：`decorations: false`
- `capabilities/default.json`：去掉 `deny-start-dragging`，补窗口 minimize/maximize/close/start-dragging/is-maximized

---

## 3. 未完成清单（第三轮 + 接线）

| ID | 任务 | 状态 | 说明 |
| --- | --- | --- | --- |
| **C** | 渲染队列并行（并发 1–4） | **半成品** | Rust 已有 `REGISTRY`/`CANCELLED`、`render_cancel(job_id)`、`render_cancel_all`；**`lib.rs` 未注册 `render_cancel_all`**；前端仍顺序单任务、`invoke('render_cancel')` 无参（**与后端不兼容**）；`PageRender` 无并发选择/单任务取消/按任务日志 |
| **D** | 配置迁移助手 | **半成品** | Rust 已有 `migrate_config`；**未注册 lib.rs**；设置页无 UI；`data.ts` 无类型 |
| **E** | 打包贴图解包 + 丢失文件检查 | **半成品** | Rust 已有 `check_blend_files` / `unpack_blend`；**未注册**；文件分析页无 UI |
| **F** | 启动分析：独立进程精确复测 | **未开始** | 需 `startup_retest` + `PageStartup` UI |
| **G** | 版本列表自动并入新默认 | **未开始** | `defaults_merged` / `merge_default_versions` + `App` onMounted |
| **V** | 全量验证 | **待做** | C–G 接线后：`cargo check` / `cargo test` / `npm run build` / `npx vue-tsc --noEmit` |
| **Z** | commit + push 1.0.0 功能 | **待做** | 顶栏 + 未完成功能做完再提交 |

### 破坏性注意（最高优先）

当前后端 `render_cancel(job_id: String)`，前端仍无参调用 → **取消渲染会失败**。接手第一件事：做完 C 的前后端契约，或临时兼容（不推荐）。

### C 已做 / 未做细节

- 已做：`render.rs` 多任务注册表、定向取消、`render_cancel_all`
- 未做：`lib.rs` 注册；`stores.ts` 并发池 + `log_buffers` + persist `concurrency`；`PageRender` UI

### D/E 已做 / 未做细节

- 已做：`maintenance.rs` 的 `migrate_config` / `check_blend_files` / `unpack_blend` + 若干单测
- 未做：`lib.rs` 注册三函数；Settings 迁移卡片；Blend「检查外部文件 / 解包」对话框；TS 类型

### F/G 实现要点（摘自原计划）

- **F：** 每个慢插件单独起 Blender `--factory-startup` 只 enable 一个；复用 `startup-progress` / `startup_cancel`；并排显示顺序 vs 独立进程，差异 >30% 标注依赖分摊
- **G：** 持久化 `defaults_merged`；缺的默认版本并入列表，不删用户自加项

---

## 4. 验证状态

| 检查 | v0.2.0 基线 | 半成品 / 顶栏后 |
| --- | --- | --- |
| `cargo test` | 8 通过（半成品预计约 10） | **需复测** |
| `cargo check` | 通过 | **需复测** |
| `npm run build` | 通过 | **需复测**（顶栏引入 png） |
| `vue-tsc --noEmit` | 零错误 | **需复测** |

```powershell
cd D:\Development\tauri\blender_addon_link\src-tauri; cargo check; cargo test
cd D:\Development\tauri\blender_addon_link; npm run build; npx vue-tsc --noEmit
```

---

## 5. Codex / 接手建议优先级

1. **修好 `render_cancel` + 完成并行渲染前端**（含 `lib.rs` 注册 `render_cancel_all`）
2. 注册并接上 `migrate_config` / `check_blend_files` / `unpack_blend` + UI
3. `startup_retest` + PageStartup
4. `merge_default_versions`
5. 四项验证全绿 → commit（说明：顶栏 mxtools 风格 + 1.0.0 + 未完成功能若本轮一并完成）→ push

push 若直连失败可用临时代理（勿写死 git config）：

```powershell
git -c http.proxy=http://127.0.0.1:7890 -c https.proxy=http://127.0.0.1:7890 push origin master
```

---

## 6. 注意事项

| 注意 | 说明 |
| --- | --- |
| PowerShell | 串联用 `;`，不要 `&&` |
| **不要**默认跑 | `tauri dev` / `npm run tb`（除非用户要求） |
| UI | 全中文；顶栏参考 `D:\Development\tauri\mxtools\src\components\AppTopBar.vue` |
| Blender 5.0 .blend | 17 字节头 `LargeBHead8`；默认 zstd；见 `blend.rs` |
| 取消渲染 | `taskkill /T /F` |
| 防休眠 | `KeepAwake` RAII |
| Tauri 2 参数 | JS camelCase ↔ Rust snake_case；`migrate_config` 若标了 `rename_all` 需对齐 |

---

## 7. 用户决策

| 决策 | 内容 |
| --- | --- |
| 版本 | 升到 **1.0.0**（不再用原计划 0.3.0） |
| 顶栏 | 参考本机 **mxtools** 自定义标题栏 |
| 不打包 | 本阶段不强制 `tauri build` |
| 第三轮功能 | 并行渲染、配置迁移、解包/丢失检查、启动独立进程复测、自动并入默认版本 |

---

## 8. 一句话

版本已到 **1.0.0**，顶栏已按 mxtools 改完；**第三轮功能仍卡在「Rust 半成品未接线」**——从修好 `render_cancel` 前后端契约并完成并行渲染前端开始，再把 D/E/F/G 接完验证后提交推送。
