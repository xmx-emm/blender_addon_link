use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use crate::procutil::{hidden_command, run_with_timeout};

/// 递归统计目录大小与文件数
fn dir_stats(path: &Path) -> (u64, u64) {
    let mut bytes = 0u64;
    let mut files = 0u64;
    if path.is_file() {
        return (path.metadata().map(|m| m.len()).unwrap_or(0), 1);
    }
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let (b, f) = dir_stats(&p);
                bytes += b;
                files += f;
            } else if let Ok(m) = e.metadata() {
                bytes += m.len();
                files += 1;
            }
        }
    }
    (bytes, files)
}

fn appdata_blender_root() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    Some(Path::new(&appdata).join("Blender Foundation").join("Blender"))
}

/// Blender stores user configuration in `major.minor` directories. Keep
/// command-provided components constrained to that shape before joining them
/// to APPDATA-derived paths.
fn is_version_component(value: &str) -> bool {
    let mut parts = value.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(major), Some(minor), None) => {
            !major.is_empty()
                && !minor.is_empty()
                && major.chars().all(|c| c.is_ascii_digit())
                && minor.chars().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

fn cache_dir() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let p = Path::new(&local)
        .join("Blender Foundation")
        .join("Blender")
        .join("Cache");
    p.is_dir().then_some(p)
}

/// %TEMP% 下属于 Blender 的临时产物
fn temp_entries() -> Vec<PathBuf> {
    let tmp = std::env::temp_dir();
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(&tmp) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            if name.starts_with("blender")
                || name.ends_with(".crash.txt")
                || name.ends_with(".blend@")
            {
                out.push(e.path());
            }
        }
    }
    out
}

#[derive(Serialize)]
pub struct CleanupTarget {
    pub id: String,
    pub label: String,
    pub path: String,
    pub bytes: u64,
    pub files: u64,
}

/// 扫描可安全清理的 Blender 产物（自动保存 / 缓存 / 临时文件）
#[tauri::command]
pub async fn scan_cleanup() -> Result<Vec<CleanupTarget>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut out: Vec<CleanupTarget> = vec![];
        if let Some(root) = appdata_blender_root() {
            if let Ok(rd) = std::fs::read_dir(&root) {
                let mut vers: Vec<String> = rd
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                vers.sort();
                for v in vers {
                    let autosave = root.join(&v).join("autosave");
                    if autosave.is_dir() {
                        let (bytes, files) = dir_stats(&autosave);
                        if files > 0 {
                            out.push(CleanupTarget {
                                id: format!("autosave:{v}"),
                                label: format!("自动保存文件（Blender {v}）"),
                                path: autosave.to_string_lossy().to_string(),
                                bytes,
                                files,
                            });
                        }
                    }
                }
            }
        }
        if let Some(cache) = cache_dir() {
            let (bytes, files) = dir_stats(&cache);
            if files > 0 {
                out.push(CleanupTarget {
                    id: "cache".into(),
                    label: "资产库索引缓存（会自动重建）".into(),
                    path: cache.to_string_lossy().to_string(),
                    bytes,
                    files,
                });
            }
        }
        let temps = temp_entries();
        if !temps.is_empty() {
            let mut bytes = 0u64;
            let mut files = 0u64;
            for t in &temps {
                let (b, f) = dir_stats(t);
                bytes += b;
                files += f;
            }
            out.push(CleanupTarget {
                id: "temp".into(),
                label: "临时文件与崩溃日志（%TEMP%）".into(),
                path: std::env::temp_dir().to_string_lossy().to_string(),
                bytes,
                files,
            });
        }
        Ok(out)
    })
    .await
    .map_err(|e| format!("扫描线程异常: {e}"))?
}

#[derive(Serialize)]
pub struct CleanupResult {
    pub freed: u64,
    pub deleted: u64,
    pub errors: Vec<String>,
}

fn delete_entry(p: &Path, res: &mut CleanupResult) {
    let (bytes, files) = dir_stats(p);
    let r = if p.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    };
    match r {
        Ok(()) => {
            res.freed += bytes;
            res.deleted += files;
        }
        Err(e) => res.errors.push(format!("{}: {e}", p.to_string_lossy())),
    }
}

/// 只清空目录内容，保留目录本身
fn delete_children(dir: &Path, res: &mut CleanupResult) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            delete_entry(&e.path(), res);
        }
    }
}

/// 按扫描结果的 id 执行清理（路径由后端重新推导，不接受任意路径）
#[tauri::command]
pub async fn run_cleanup(ids: Vec<String>) -> Result<CleanupResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut res = CleanupResult {
            freed: 0,
            deleted: 0,
            errors: vec![],
        };
        for id in ids {
            if let Some(v) = id.strip_prefix("autosave:") {
                if !is_version_component(v) {
                    continue;
                }
                if let Some(root) = appdata_blender_root() {
                    let dir = root.join(v).join("autosave");
                    if dir.is_dir() {
                        delete_children(&dir, &mut res);
                    }
                }
            } else if id == "cache" {
                if let Some(cache) = cache_dir() {
                    delete_children(&cache, &mut res);
                }
            } else if id == "temp" {
                for t in temp_entries() {
                    delete_entry(&t, &mut res);
                }
            }
        }
        Ok(res)
    })
    .await
    .map_err(|e| format!("清理线程异常: {e}"))?
}

#[derive(Serialize)]
pub struct PurgeResult {
    pub removed: i64,
    pub old_size: u64,
    pub new_size: u64,
    pub backup: String,
}

const PURGE_MARKER: &str = "@@BL_PURGE@@";

/// 校验 exe 与 .blend 都存在
fn check_exe_and_blend(exe: &str, path: &str) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err(format!(".blend 文件不存在: {path}"));
    }
    if !Path::new(exe).is_file() {
        return Err(format!("blender.exe 不存在: {exe}"));
    }
    Ok(())
}

/// 把文件备份为 .bak（已存在则带时间戳），返回备份路径
fn backup_file(path: &str) -> Result<PathBuf, String> {
    let source = Path::new(path);
    let mut source_file = File::open(source).map_err(|e| format!("备份失败: {e}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    // Reserve the destination with create_new so concurrent operations cannot
    // overwrite an existing backup, even when they happen in one timestamp.
    for attempt in 0..1000u32 {
        let backup = if attempt == 0 {
            PathBuf::from(format!("{path}.bak"))
        } else {
            PathBuf::from(format!("{path}.{stamp}-{attempt}.bak"))
        };
        let mut destination = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
        {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("备份失败: {e}")),
        };
        if let Err(e) = io::copy(&mut source_file, &mut destination) {
            let _ = std::fs::remove_file(&backup);
            return Err(format!("备份失败: {e}"));
        }
        return Ok(backup);
    }
    Err("备份失败：无法生成唯一的备份文件名".into())
}

/// 从 Blender stdout 里取 marker 后面的内容
fn find_marker<'a>(stdout: &'a str, marker: &str) -> Option<&'a str> {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix(marker))
        .map(str::trim)
}

/// 清理 .blend 孤立数据：先备份，再用 Blender 后台 orphans_purge 并保存
#[tauri::command]
pub async fn purge_orphans(exe: String, path: String) -> Result<PurgeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        check_exe_and_blend(&exe, &path)?;
        let src = Path::new(&path);
        let old_size = src.metadata().map(|m| m.len()).unwrap_or(0);
        let backup = backup_file(&path)?;

        let expr = format!(
            "import bpy, sys\n\
             n = bpy.data.orphans_purge(do_recursive=True)\n\
             bpy.ops.wm.save_mainfile()\n\
             print('{PURGE_MARKER}' + str(n))\n\
             sys.stdout.flush()\n\
             sys.exit(0)"
        );
        let mut cmd = hidden_command(&exe);
        // factory-startup：避免用户插件的 load/save 钩子在清理过程中改动文件
        cmd.args(["-b", "--factory-startup", &path, "--python-expr", &expr]);
        let out = run_with_timeout(cmd, 600, None)?;
        let removed =
            find_marker(&out.stdout, PURGE_MARKER).and_then(|s| s.parse::<i64>().ok());
        let Some(removed) = removed else {
            return Err(format!(
                "清理未完成（退出码 {}）。原文件未受影响，备份在 {}",
                out.code,
                backup.to_string_lossy()
            ));
        };
        let new_size = src.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(PurgeResult {
            removed,
            old_size,
            new_size,
            backup: backup.to_string_lossy().to_string(),
        })
    })
    .await
    .map_err(|e| format!("清理线程异常: {e}"))?
}

// ---- 配置迁移 ----

/// 值得跨版本迁移的用户配置文件
const MIGRATE_FILES: &[&str] = &["userpref.blend", "startup.blend", "bookmarks.txt"];

/// 推导迁移的源/目标 config 目录（纯函数，便于测试）
fn migrate_config_dirs(root: &Path, from: &str, to: &str) -> (PathBuf, PathBuf) {
    (root.join(from).join("config"), root.join(to).join("config"))
}

/// 把旧版本的 userpref.blend / startup.blend / bookmarks.txt 复制到新版本 config 目录。
/// 目标已存在的文件先备份为 .bak。返回复制成功的文件名列表。
#[tauri::command(rename_all = "snake_case")]
pub async fn migrate_config(from_version: String, to_version: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if from_version == to_version {
            return Err("源版本与目标版本相同".into());
        }
        if !is_version_component(&from_version) || !is_version_component(&to_version) {
            return Err("版本目录必须是 major.minor 格式".into());
        }
        let root = appdata_blender_root().ok_or("读取 APPDATA 环境变量失败")?;
        let (src_dir, dst_dir) = migrate_config_dirs(&root, &from_version, &to_version);
        if !src_dir.is_dir() {
            return Err(format!(
                "源版本没有配置目录：{}（该版本可能还没启动过）",
                src_dir.to_string_lossy()
            ));
        }
        std::fs::create_dir_all(&dst_dir).map_err(|e| format!("创建目标目录失败: {e}"))?;
        let mut copied: Vec<String> = vec![];
        for name in MIGRATE_FILES {
            let src = src_dir.join(name);
            if !src.is_file() {
                continue;
            }
            let dst = dst_dir.join(name);
            if dst.exists() {
                let bak = dst_dir.join(format!("{name}.bak"));
                std::fs::copy(&dst, &bak).map_err(|e| format!("备份目标 {name} 失败: {e}"))?;
            }
            std::fs::copy(&src, &dst).map_err(|e| format!("复制 {name} 失败: {e}"))?;
            copied.push(name.to_string());
        }
        if copied.is_empty() {
            return Err(
                "源版本配置目录里没有可迁移的文件（userpref.blend / startup.blend / bookmarks.txt）"
                    .into(),
            );
        }
        Ok(copied)
    })
    .await
    .map_err(|e| format!("迁移线程异常: {e}"))?
}

// ---- 打包贴图解包 + 丢失文件检查 ----

#[derive(Serialize, Deserialize)]
pub struct PackedFile {
    pub name: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize)]
pub struct MissingFile {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct FileCheckResult {
    pub packed: Vec<PackedFile>,
    pub missing: Vec<MissingFile>,
}

const CHECK_MARKER: &str = "@@BL_CHECK@@";
const UNPACK_MARKER: &str = "@@BL_UNPACK@@";

/// 检查脚本：列出打包文件与外链丢失文件（只读，不保存）
const CHECK_SCRIPT: &str = r#"import bpy, json, os, sys
packed = []
missing = []
for coll_name in ('images', 'sounds', 'volumes', 'libraries', 'fonts'):
    coll = getattr(bpy.data, coll_name, None)
    if coll is None:
        continue
    for d in coll:
        pf = getattr(d, 'packed_file', None)
        if pf is not None:
            packed.append({'name': d.name, 'size': int(pf.size)})
            continue
        fp = getattr(d, 'filepath', '') or ''
        if not fp or fp.startswith('<builtin'):
            continue
        try:
            ap = bpy.path.abspath(fp, library=getattr(d, 'library', None))
        except Exception:
            ap = fp
        if not os.path.exists(ap):
            missing.append({'name': d.name, 'path': ap})
print('@@BL_CHECK@@' + json.dumps({'packed': packed, 'missing': missing}))
sys.stdout.flush()
sys.exit(0)
"#;

/// 检查 .blend 的打包文件与丢失的外部文件（只读，不修改文件）
#[tauri::command]
pub async fn check_blend_files(exe: String, path: String) -> Result<FileCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        check_exe_and_blend(&exe, &path)?;
        let mut cmd = hidden_command(&exe);
        cmd.args(["-b", "--factory-startup", &path, "--python-expr", CHECK_SCRIPT]);
        let out = run_with_timeout(cmd, 600, None)?;
        let Some(json) = find_marker(&out.stdout, CHECK_MARKER) else {
            return Err(format!(
                "检查未完成（退出码 {}）。stderr 末行：{}",
                out.code,
                out.stderr.lines().last().unwrap_or("")
            ));
        };
        serde_json::from_str::<FileCheckResult>(json).map_err(|e| format!("解析检查结果失败: {e}"))
    })
    .await
    .map_err(|e| format!("检查线程异常: {e}"))?
}

#[derive(Serialize)]
pub struct UnpackResult {
    pub unpacked: i64,
    pub old_size: u64,
    pub new_size: u64,
    pub backup: String,
}

/// 解包脚本：USE_LOCAL 把打包文件写到 .blend 旁的 textures/ 等目录，然后保存
const UNPACK_SCRIPT: &str = r#"import bpy, sys
def count_packed():
    n = 0
    for coll_name in ('images', 'sounds', 'volumes', 'libraries', 'fonts'):
        coll = getattr(bpy.data, coll_name, None)
        if coll is None:
            continue
        for d in coll:
            if getattr(d, 'packed_file', None) is not None:
                n += 1
    return n
before = count_packed()
bpy.ops.file.unpack_all(method='USE_LOCAL')
after = count_packed()
bpy.ops.wm.save_mainfile()
print('@@BL_UNPACK@@' + str(before - after))
sys.stdout.flush()
sys.exit(0)
"#;

/// 解包全部打包文件（USE_LOCAL：写到 .blend 同目录 textures/ 等），保存前先备份 .bak
#[tauri::command]
pub async fn unpack_blend(exe: String, path: String) -> Result<UnpackResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        check_exe_and_blend(&exe, &path)?;
        let src = Path::new(&path);
        let old_size = src.metadata().map(|m| m.len()).unwrap_or(0);
        let backup = backup_file(&path)?;

        let mut cmd = hidden_command(&exe);
        cmd.args(["-b", "--factory-startup", &path, "--python-expr", UNPACK_SCRIPT]);
        let out = run_with_timeout(cmd, 600, None)?;
        let unpacked =
            find_marker(&out.stdout, UNPACK_MARKER).and_then(|s| s.parse::<i64>().ok());
        let Some(unpacked) = unpacked else {
            return Err(format!(
                "解包未完成（退出码 {}）。原文件未受影响，备份在 {}",
                out.code,
                backup.to_string_lossy()
            ));
        };
        let new_size = src.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(UnpackResult {
            unpacked,
            old_size,
            new_size,
            backup: backup.to_string_lossy().to_string(),
        })
    })
    .await
    .map_err(|e| format!("解包线程异常: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::{
        backup_file, find_marker, is_version_component, migrate_config_dirs, CHECK_MARKER,
        PURGE_MARKER,
    };
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn version_component_rejects_path_syntax() {
        assert!(is_version_component("4.2"));
        assert!(!is_version_component("4.2.0"));
        assert!(!is_version_component(".."));
        assert!(!is_version_component("4.2\\..\\other"));
        assert!(!is_version_component("volume:other"));
    }

    #[test]
    fn backup_file_never_overwrites_an_existing_backup() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("blender_link_backup_{stamp}"));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("scene.blend");
        fs::write(&source, b"first").unwrap();

        let first = backup_file(&source.to_string_lossy()).unwrap();
        fs::write(&source, b"second").unwrap();
        let second = backup_file(&source.to_string_lossy()).unwrap();

        assert_ne!(first, second);
        assert_eq!(fs::read(&first).unwrap(), b"first");
        assert_eq!(fs::read(&second).unwrap(), b"second");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migrate_dirs_derived_from_versions() {
        let root = Path::new("fixtures").join("Blender");
        let (src, dst) = migrate_config_dirs(&root, "4.2", "5.2");
        assert_eq!(src, root.join("4.2").join("config"));
        assert_eq!(dst, root.join("5.2").join("config"));
    }

    #[test]
    fn find_marker_extracts_payload() {
        let stdout = "Blender 5.2.0\nRead blend ok\n  @@BL_PURGE@@42  \nBlender quit\n";
        assert_eq!(find_marker(stdout, PURGE_MARKER), Some("42"));
        assert_eq!(find_marker(stdout, CHECK_MARKER), None);
    }
}
