# Blender Link 工具箱 — Codex 交接文档

> 生成时间：2026-07-21  
> 仓库：`D:\Development\tauri\blender_addon_link`  
> 用途：Cursor 额度用尽，转 Codex 继续第三轮开发。  
> 本文档与第三轮半成品（图标 + Rust WIP）一并提交，前端接线与验证仍待完成。

---

## 1. 项目概览

| 项 | 内容 |
| --- | --- |
| 产品名 | **Blender Link 工具箱**（窗口标题同名） |
| 定位 | Windows 桌面小工具：多版本 Blender 插件 junction 链接 + 启动分析 + .blend 分析 + 渲染队列 + 设置/清理 |
| 技术栈 | **Tauri 2** + **Vue 3** + **TypeScript** + **Vuetify 3** + **Pinia**（`pinia-plugin-persistedstate` v4） |
| 当前版本号 | **0.2.0**（`package.json` / `src-tauri/Cargo.toml` / `tauri.conf.json` / 设置页「关于」均为 0.2.0） |
| 第三轮目标版本 | **0.3.0**（未开始 bump） |
| 远端 | `origin` → `https://github.com/xmx-emm/blender_addon_link.git` |
| 分支 | `master`，跟踪 `origin/master` |

### 目录要点

- Rust：`src-tauri/src/` — `lib.rs`（注册 command）、`link.rs`、`detect.rs`、`blend.rs`、`startup.rs`、`render.rs`、`maintenance.rs`、`procutil.rs`
- 前端：`src/` — `App.vue`（五页 rail）、`stores.ts`、`data.ts`、`pages/{PageLink,PageStartup,PageBlend,PageRender,PageSettings}.vue`、`components/LinkAddonCard.vue`
- UI：全中文；`card-soft` 卡片、tonal 按钮、反馈走 `ui.ok` / `ui.error` snackbar

---

## 2. 已完成：v0.2.0 两轮优化（已 commit + push）

**Commit：** `34c7526` — `feat(core): v0.2.0 大版本升级——五页工具箱架构`  
**已 push 到** `origin/master`（首次直连失败，用临时代理 `127.0.0.1:7890` 成功）。

### 第一轮（架构 + 五页）

- 五页导航：插件链接 / 启动分析 / 文件分析 / 渲染队列 / 设置
- Rust 模块化拆分；NTFS junction 链接；Blender 探测（注册表 / Program Files / Steam）
- `.blend` 本地解析：支持 5.0 `LargeBHead8` 新头、zstd/gzip、`REND`/`TEST` 块
- 启动分析：预热 + 多轮中位数 + 逐插件计时（`startup-progress`）
- 渲染队列：单任务顺序执行、`render-log`/`render-progress`、KeepAwake、`taskkill /T`
- 维护：缓存扫描清理、孤立数据 `orphans_purge`（备份 `.bak`）

### 第二轮（收尾子代理 [第二轮收尾](a585b0d6-a636-430c-9cdd-b6020411719b)）

- 插件元信息：解析 `bl_info` / `blender_manifest.toml`，真实名称、版本、最低 Blender 版本警告
- 设置页磁盘清理：autosave / 资产库索引 / `%TEMP%` Blender 临时文件
- 渲染：任务编辑、断点续渲、系统通知、防休眠
- 文件分析：一键清理孤立数据（本机 Blender 5.2 实测过）
- 窗口状态记忆；前端分包（vuetify/vue 独立 chunk）
- **验证（第二轮结束时）：** `cargo test` 8 绿、`cargo check` 无警告、`npm run build` 过、`npx vue-tsc --noEmit` 零错误

---

## 3. 第三轮任务清单与完成状态

任务来源：用户勾选「下一轮」要做的功能；由子代理 [第三轮开发](7288db07-12f8-4fff-a8bd-8bd654dc6d95) 执行。  
**该子代理在写完部分 Rust 后因 Cursor 账单未付中断**（`unpaid invoice`），未完成前端、接线、验证、收尾 commit。

| ID | 任务 | 状态 | 说明 |
| --- | --- | --- | --- |
| **A** | Git：提交 v0.2.0 并 push | **完成** | `34c7526` 已在远端 |
| **B** | 应用图标：复制素材 + `npx tauri icon` | **完成（未 commit）** | `src-tauri/app-icon.png` + `icons/` 全套已生成（含 android/ios） |
| **C** | 渲染队列并行（并发 1–4） | **进行中** | 见下「C 细节」 |
| **D** | 配置迁移助手 | **进行中** | 见下「D/E 细节」 |
| **E** | 打包贴图解包 + 丢失文件检查 | **进行中** | 见下「D/E 细节」 |
| **F** | 启动分析：独立进程精确复测 | **未开始** | `startup.rs` / `PageStartup.vue` 无 `startup_retest` |
| **G** | 版本列表自动并入新默认 | **未开始** | `stores.ts` 无 `defaults_merged` / `merge_default_versions` |
| **H** | 版本升至 0.3.0 + README | **未开始** | 仍为 0.2.0 |
| **V** | 第三轮全量验证 | **未开始** | 当前工作区相对 v0.2.0 有破坏性半成品，需修完再测 |
| **Z** | 第三轮 commit + push | **未开始** | 图标 + Rust 半成品均未提交 |

### C — 渲染并行（细节）

**已做（Rust `render.rs`）：**

- `RUNNING`/`CANCEL`/`CHILD_PID` → `REGISTRY: Mutex<HashMap<job_id, pid>>` + `CANCELLED: Mutex<HashSet>`
- `render_run`：占位注册、清理 cancel、结束兜底移除
- `render_cancel(job_id: String)`：定向取消 + `taskkill /T`
- 新增 `render_cancel_all()`

**未做：**

- `lib.rs` **未注册** `render_cancel_all`
- 前端 `stores.ts` 仍是顺序单任务：`log_lines`、`invoke('render_cancel')` **无参**（与后端签名已不兼容！）
- `PageRender.vue`：无并发选择、无单任务取消、无按任务日志 tab
- persist 未加 `concurrency`

⚠ **当前工作区若直接跑应用，取消渲染会坏**：前端仍 `invoke('render_cancel')`，后端已要求 `job_id`。

### D/E — 配置迁移 / 解包检查（细节）

**已做（Rust `maintenance.rs`）：**

- 抽出 `backup_file` / `check_exe_and_blend` / `find_marker`
- `migrate_config(from_version, to_version)`：复制 `userpref.blend` / `startup.blend` / `bookmarks.txt`，目标先 `.bak`
- `check_blend_files` / `unpack_blend`（`--factory-startup` + marker；解包 `USE_LOCAL`）
- 单测：`migrate_dirs_derived_from_versions`、`find_marker_extracts_payload`

**未做：**

- `lib.rs` **未注册** `migrate_config` / `check_blend_files` / `unpack_blend`
- `PageSettings.vue` 无迁移卡片
- `PageBlend.vue` 无「检查外部文件」/ 解包 UI
- `data.ts` 无对应 TS 类型

### F/G/H — 未开始要点（供直接实现）

- **F：** `startup_retest(app, exe, version, modules)`：每个模块单独起 Blender（`--factory-startup`），只 enable 一个模块计时；复用 `startup-progress` / `startup_cancel`；前端对最慢 `min(5, n)` 个并排显示顺序 vs 独立进程，差异 >30% 标注依赖分摊说明。
- **G：** `useBlenderAddonStore` 加 `defaults_merged`；`merge_default_versions()`；`App.vue` `onMounted` 调用；缺的默认版本并入 `blender_version_list`，不删用户自加项。
- **H：** `package.json`、`Cargo.toml`、`tauri.conf.json`、设置页关于文案、`README.md` 功能表同步到 0.3.0。

---

## 4. 验证状态

| 检查 | 第二轮结束（已进 `34c7526`） | 第三轮当前工作区 |
| --- | --- | --- |
| `cd src-tauri; cargo test` | 8 通过 | **未复测**（maintenance 增测预计共 ~10；需先确认能编过） |
| `cd src-tauri; cargo check` | 通过 | **未复测** |
| 根目录 `npm run build` | 通过 | **未复测**（前端未改并行逻辑） |
| `npx vue-tsc --noEmit` | 零错误 | **未复测** |

接手后务必在 C–H 做完、`lib.rs` 注册齐全后再跑上述四项，全绿再 commit。

---

## 5. Git 状态（写本文档时）

```
分支：master = origin/master @ 34c7526
工作区：脏，未 staged

已修改：
  src-tauri/src/render.rs          （并行后端）
  src-tauri/src/maintenance.rs     （迁移/检查/解包 + 测试）
  src-tauri/icons/*                （图标二进制大批更新）

未跟踪：
  src-tauri/app-icon.png
  src-tauri/icons/64x64.png
  src-tauri/icons/android/
  src-tauri/icons/ios/

未提交第三轮功能；未 push 第三轮。
```

建议：功能做完后 **一次 commit** 覆盖「图标 + C–H + 0.3.0」，或拆成「图标」与「0.3.0 功能」两次；push 若直连 GitHub 失败，可用：

```powershell
git -c http.proxy=http://127.0.0.1:7890 -c https.proxy=http://127.0.0.1:7890 push origin master
```

（本机 Clash 常见端口 7890；**不要改 git config 写死代理**，除非用户要求。）

PowerShell 提交消息勿用 bash heredoc，用：

```powershell
git commit -F .git/COMMIT_MSG_TMP.txt
```

---

## 6. Codex 接手建议优先级

1. **先修破坏性半成品（最高优先）**  
   - 要么立刻做完 C 的前端 + `lib.rs` 注册 `render_cancel_all`；  
   - 要么临时把 `render_cancel` 兼容旧无参调用（不推荐，直接做完 C 更好）。  
2. **接线 D/E：** `lib.rs` 注册三个 maintenance command → Settings/Blend UI + `data.ts` 类型。  
3. **实现 F：** `startup_retest` + PageStartup UI。  
4. **实现 G：** `merge_default_versions` + App onMounted。  
5. **H：** bump 0.3.0 + README。  
6. **验证：** `cargo check`；`cargo test`；`npm run build`；`npx vue-tsc --noEmit`。  
7. **commit + push**（中文 conventional commit，说明并行渲染/迁移/解包/复测/图标/0.3.0）。

实现时注意：

- 前端 invoke 多词参数：Tauri 2 默认 JS camelCase ↔ Rust snake_case；`migrate_config` 已标 `rename_all = "snake_case"`，前端需与之一致。
- UI 风格对齐现有页；禁止静默失败。
- `maintenance.rs` 里对 `PackedFile`/`MissingFile` 手写了 `Deserialize`（冗余），可改为 `#[derive(Serialize, Deserialize)]` 简化，编译前顺手清一下更稳妥。

---

## 7. 关键路径与注意事项

| 注意 | 说明 |
| --- | --- |
| PowerShell | 命令串联用 `;`，**不要**用 `&&` |
| stderr 噪音 | `cargo`/`npm` 写 stderr 时 PowerShell 可能报无害 `NativeCommandError`，可忽略 |
| **不要**跑 | `npm run tauri` / `tauri dev` / `npm run tb`（用户明确不打包、不要求则不启 GUI 开发服） |
| UI 语言 | 全中文 |
| Blender 5.0 .blend | 新 17 字节头 `LargeBHead8`；压缩默认 zstd；解析在 `blend.rs`，勿按旧 12 字节头假设 |
| 取消渲染 | Windows 用 `taskkill /T /F` 杀进程树 |
| 防休眠 | `SetThreadExecutionState`，RAII `KeepAwake`；并行时每渲染线程各持一份 |
| 持久化 store | `blender_addon` / render 的 persist pick 字段变更时注意旧数据缺字段默认值 |
| `.gitignore` | 已排除 `node_modules` / `dist` / `target`，勿加进提交 |

常用验证命令：

```powershell
cd D:\Development\tauri\blender_addon_link\src-tauri; cargo check; cargo test
cd D:\Development\tauri\blender_addon_link; npm run build; npx vue-tsc --noEmit
```

---

## 8. 用户决策记录

| 决策 | 内容 |
| --- | --- |
| 不打包 | 本阶段不做安装包 / 不跑 `tauri build` |
| 图标 | 已生成（Blender 橙链条主题），**待随第三轮一并 commit/接入**（`tauri icon` 已写入 `src-tauri/icons/`，`tauri.conf.json` 的 icon 路径原已指向该目录，一般无需再改路径） |
| 下一轮范围 | **并行渲染**、**配置迁移**、**解包/丢失检查**、**启动独立进程复测**、**自动并入默认版本**，并升 **0.3.0** |

---

## 9. 相关转录（便于追溯）

| 轮次 | 路径 / ID |
| --- | --- |
| 第三轮开发子代理 | `.../agent-transcripts/e1028f06-30e7-477e-9f84-55d60e2dc49f/subagents/7288db07-12f8-4fff-a8bd-8bd654dc6d95.jsonl` |
| 第二轮收尾 | `.../subagents/a585b0d6-a636-430c-9cdd-b6020411719b.jsonl` |
| 父会话 | `e1028f06-30e7-477e-9f84-55d60e2dc49f` |

第三轮子代理最后进度：任务 A、B 完成 → C 后端改完 → 正在改/已写入 D+E 的 `maintenance.rs` → **因 unpaid invoice 中断**，未做前端与后续。

---

## 10. 一句话给 Codex

从 **修好 `render_cancel` 前后端契约并完成并行渲染前端** 开始，接着注册并接上 `migrate_config` / `check_blend_files` / `unpack_blend`，再做 `startup_retest` 与 `merge_default_versions`，bump **0.3.0**，四项验证全绿后 commit（含图标）并 push。
