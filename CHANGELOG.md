# 更新日志

## 1.0.1 - 2026-08-11

### 修复

- 修复渲染队列取消按钮没有传递任务 ID，以及任务刚启动时取消请求可能丢失的问题。
- 避免 Blender 4.2+ 的备用插件目录已有安装时再次创建双重安装。
- 限制维护命令只能访问 `major.minor` 版本目录，阻止构造路径穿越。
- 让 `.blend` 备份使用不可覆盖的唯一文件名，避免快速重复操作覆盖旧备份。
- 兼容旧版本持久化的渲染任务数据，升级后自动补齐缺失字段。
- “关于”信息直接读取前端包版本，减少后续发布时的版本号漂移。

### 验证

- `npm run typecheck`
- `npm run build`
- `npm run rust:test`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

### 已知限制

- 配置迁移、外部文件检查/解包和并行渲染后端命令已存在，但当前版本尚未提供对应的完整前端工作流。
