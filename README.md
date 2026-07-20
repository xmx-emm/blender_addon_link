# Blender Link 工具箱

[![](https://i1.hdslb.com/bfs/archive/4ddb37a9a11f4132c8bfcf5e2f196d12af1e5068.jpg)](https://www.bilibili.com/video/BV1NAmBYGEAQ)

Windows 桌面小工具（Tauri 2 + Vue 3 + Vuetify 3），围绕 Blender 多版本日常使用的四件套：

| 页面 | 功能 |
| --- | --- |
| 插件链接 | 把插件开发目录通过 NTFS junction 链接到各版本 Blender（4.2 ~ 5.2+），一份代码多版本调试；支持传统插件与新版扩展，自动识别、批量链接、冲突替换；读取 bl_info / manifest 显示插件真实名称、版本与最低 Blender 版本，低于要求时给出兼容警告 |
| 启动分析 | 测量正常启动 vs 纯净启动（`--factory-startup`）耗时，并逐个插件计时，条形图找出拖慢启动的插件 |
| 文件分析 | 本地解析 .blend 二进制（支持 5.0 新格式 / zstd / gzip），展示体积构成、最大数据块、缩略图与优化建议；一键清理孤立数据（自动备份 .bak 后用 Blender 后台 `orphans_purge` 并回报省了多少） |
| 渲染队列 | 多任务批量后台渲染，实时帧/采样进度与日志，支持帧区间、场景、引擎、输出路径覆盖；任务可编辑、失败/取消后可从中断帧继续；渲染期间防休眠，全部完成发系统通知，可选自动关机 |
| 设置 | 自动探测已安装 Blender（注册表 / Program Files / Steam）；磁盘清理：各版本自动保存文件、资产库索引缓存、%TEMP% 临时文件与崩溃日志 |

## 使用要点

- 首次使用先到「设置」页点「自动探测已安装的 Blender」（扫描注册表 / Program Files / Steam），启动分析与渲染队列依赖 blender.exe 路径。
- 断开链接只会删除链接本身，不会碰源码目录；但不要在 Blender 偏好设置里"卸载"链接安装的扩展——那可能连源目录一起删。
- 拖拽即用：插件文件夹拖进窗口 → 插件链接；.blend 文件拖进窗口 → 文件分析 / 渲染队列。

## 开发

```
npm install
npm run tauri     # 开发运行
npm run tb        # 打包（tauri build）
```

打包依赖（首次）：

```
1. WiX Toolset  %LocalAppData%\tauri\WixTools314
   https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip
2. NSIS         %LocalAppData%\tauri\NSIS
   https://github.com/tauri-apps/binary-releases/releases/download/nsis-3/nsis-3.zip
   https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.4.1/nsis_tauri_utils.dll
```

## 技术备注

- 链接使用 junction（无需管理员权限），Rust 端 `junction` crate 创建，删除前校验 reparse point，拒绝误删真实目录。
- .blend 解析自研：识别 `BLENDER` 12 字节经典头（BHead4/SmallBHead8）与 Blender 5.0 起的 17 字节新头（LargeBHead8），压缩支持 zstd（3.0+ 默认）与 gzip（2.9x 及更早）；`REND` 块直读帧范围/场景名，`TEST` 块提取缩略图，DATA 块归属到前一个 ID 块以统计打包贴图体积。
- 启动分析先预热一轮（排除杀毒/磁盘缓存的冷启动干扰）再多轮取中位数；逐插件计时在 `--factory-startup` 环境用 `addon_utils.enable` 逐个测量，扩展先 `extensions_refresh()`。
- 渲染队列逐行解析 Blender 输出（`Fra:` / `Sample x/y` / `Rendering x / y samples` / `Saved:`），兼容 4.x 经典格式与 5.x 新日志格式；取消时用 `taskkill /T` 杀整棵进程树；渲染中通过 `SetThreadExecutionState` 防止系统休眠。
- 窗口大小与位置自动记忆（tauri-plugin-window-state）。
