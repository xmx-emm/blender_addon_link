use serde::Serialize;
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

/// 清理 .blend 孤立数据：先备份，再用 Blender 后台 orphans_purge 并保存
#[tauri::command]
pub async fn purge_orphans(exe: String, path: String) -> Result<PurgeResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let src = Path::new(&path);
        if !src.is_file() {
            return Err(format!(".blend 文件不存在: {path}"));
        }
        if !Path::new(&exe).is_file() {
            return Err(format!("blender.exe 不存在: {exe}"));
        }
        let old_size = src.metadata().map(|m| m.len()).unwrap_or(0);

        // 备份为 .bak（已存在则带时间戳）
        let mut backup = PathBuf::from(format!("{path}.bak"));
        if backup.exists() {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            backup = PathBuf::from(format!("{path}.{ts}.bak"));
        }
        std::fs::copy(src, &backup).map_err(|e| format!("备份失败: {e}"))?;

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
        let removed = out
            .stdout
            .lines()
            .find_map(|l| l.trim().strip_prefix(PURGE_MARKER))
            .and_then(|s| s.trim().parse::<i64>().ok());
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
