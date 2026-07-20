use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::procutil::{hidden_command, run_with_timeout};

/// 版本目录形如 "4.2" / "5.10"
fn is_version_dir(name: &str) -> bool {
    let mut parts = name.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(a), Some(b), None) => {
            !a.is_empty()
                && !b.is_empty()
                && a.chars().all(|c| c.is_ascii_digit())
                && b.chars().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// 扫描 %APPDATA%\Blender Foundation\Blender\ 下的版本配置目录
#[tauri::command]
pub fn detect_config_versions() -> Result<Vec<String>, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| format!("读取 APPDATA 失败: {e}"))?;
    let root = Path::new(&appdata).join("Blender Foundation").join("Blender");
    let mut found: Vec<String> = vec![];
    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if e.path().is_dir() && is_version_dir(&name) {
                found.push(name);
            }
        }
    }
    found.sort_by(|a, b| version_key(a).cmp(&version_key(b)));
    Ok(found)
}

fn version_key(v: &str) -> (u32, u32) {
    let mut it = v.split('.');
    let a = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let b = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    (a, b)
}

#[derive(Serialize, Clone)]
pub struct BlenderExe {
    pub version: String,
    pub path: String,
    pub source: String,
}

fn exe_version(path: &Path) -> Option<String> {
    // `blender.exe -v` 输出首行形如 "Blender 5.2.0"
    let mut cmd = hidden_command(path.to_string_lossy().as_ref());
    cmd.arg("-v");
    let out = run_with_timeout(cmd, 20, None).ok()?;
    for line in out.stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Blender ") {
            let ver = rest.split_whitespace().next()?;
            let mut it = ver.split('.');
            let major = it.next()?;
            let minor = it.next()?;
            return Some(format!("{major}.{minor}"));
        }
    }
    None
}

fn push_exe(found: &mut BTreeMap<String, BlenderExe>, path: PathBuf, source: &str, version_hint: Option<String>) {
    if !path.is_file() {
        return;
    }
    let key = path.to_string_lossy().to_lowercase();
    if found.contains_key(&key) {
        return;
    }
    let version = version_hint
        .filter(|v| is_version_dir(v))
        .or_else(|| exe_version(&path));
    if let Some(version) = version {
        found.insert(
            key,
            BlenderExe {
                version,
                path: path.to_string_lossy().to_string(),
                source: source.to_string(),
            },
        );
    }
}

/// 从目录名 "Blender 4.2" 提取 "4.2"
fn version_from_dir_name(name: &str) -> Option<String> {
    let v = name.trim().rsplit(' ').next()?;
    if is_version_dir(v) {
        Some(v.to_string())
    } else {
        None
    }
}

fn scan_program_files(found: &mut BTreeMap<String, BlenderExe>) {
    for env in ["ProgramFiles", "ProgramFiles(x86)"] {
        let Ok(pf) = std::env::var(env) else { continue };
        let root = Path::new(&pf).join("Blender Foundation");
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let dir = e.path();
            if !dir.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            let exe = dir.join("blender.exe");
            push_exe(found, exe, "安装目录", version_from_dir_name(&name));
        }
    }
}

fn scan_registry(found: &mut BTreeMap<String, BlenderExe>) {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    let roots = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for root in roots {
        let Ok(uninstall) = hklm.open_subkey_with_flags(root, KEY_READ) else {
            continue;
        };
        for sub in uninstall.enum_keys().flatten() {
            let Ok(k) = uninstall.open_subkey_with_flags(&sub, KEY_READ) else {
                continue;
            };
            let name: String = k.get_value("DisplayName").unwrap_or_default();
            if !name.starts_with("Blender") {
                continue;
            }
            let loc: String = k.get_value("InstallLocation").unwrap_or_default();
            if loc.is_empty() {
                continue;
            }
            let ver: String = k.get_value("DisplayVersion").unwrap_or_default();
            let hint = {
                let mut it = ver.split('.');
                match (it.next(), it.next()) {
                    (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => Some(format!("{a}.{b}")),
                    _ => None,
                }
            };
            push_exe(found, Path::new(&loc).join("blender.exe"), "注册表", hint);
        }
    }
}

fn scan_steam(found: &mut BTreeMap<String, BlenderExe>) {
    // 默认库 + libraryfolders.vdf 中登记的其它库
    let mut libs: Vec<PathBuf> = vec![];
    for env in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(pf) = std::env::var(env) {
            libs.push(Path::new(&pf).join("Steam"));
        }
    }
    let vdfs: Vec<PathBuf> = libs
        .iter()
        .map(|l| l.join("steamapps").join("libraryfolders.vdf"))
        .collect();
    for vdf in vdfs {
        let Ok(text) = std::fs::read_to_string(&vdf) else { continue };
        for line in text.lines() {
            let line = line.trim();
            // 行形如: "path"		"D:\\SteamLibrary"
            if let Some(rest) = line.strip_prefix("\"path\"") {
                let p = rest.trim().trim_matches('"').replace("\\\\", "\\");
                if !p.is_empty() {
                    libs.push(PathBuf::from(p));
                }
            }
        }
    }
    for lib in libs {
        let exe = lib
            .join("steamapps")
            .join("common")
            .join("Blender")
            .join("blender.exe");
        push_exe(found, exe, "Steam", None);
    }
}

/// 探测本机已安装的 blender.exe（注册表 / Program Files / Steam）
#[tauri::command]
pub fn detect_blender_executables() -> Result<Vec<BlenderExe>, String> {
    let mut found: BTreeMap<String, BlenderExe> = BTreeMap::new();
    scan_registry(&mut found);
    scan_program_files(&mut found);
    scan_steam(&mut found);
    let mut list: Vec<BlenderExe> = found.into_values().collect();
    list.sort_by(|a, b| version_key(&a.version).cmp(&version_key(&b.version)));
    Ok(list)
}

/// 校验手选的 exe 并返回其 X.Y 版本
#[tauri::command]
pub fn probe_blender_exe(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.is_file() {
        return Err("文件不存在".into());
    }
    exe_version(p).ok_or_else(|| "无法识别该文件的 Blender 版本（请选择 blender.exe）".into())
}

/// 用资源管理器打开目录（选中文件亦可）
#[tauri::command]
pub fn open_in_explorer(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("路径不存在: {path}"));
    }
    let mut cmd = hidden_command("explorer.exe");
    if p.is_file() {
        cmd.arg("/select,").arg(path);
    } else {
        cmd.arg(path);
    }
    cmd.spawn().map_err(|e| format!("打开资源管理器失败: {e}"))?;
    Ok(())
}

/// 以 GUI 方式启动 Blender（detached）
#[tauri::command]
pub fn launch_blender(exe: &str) -> Result<(), String> {
    let p = Path::new(exe);
    if !p.is_file() {
        return Err(format!("blender.exe 不存在: {exe}"));
    }
    // 优先用同目录的 blender-launcher.exe，避免残留控制台
    let launcher = p.with_file_name("blender-launcher.exe");
    let target = if launcher.is_file() { launcher } else { p.to_path_buf() };
    hidden_command(target.to_string_lossy().as_ref())
        .spawn()
        .map_err(|e| format!("启动 Blender 失败: {e}"))?;
    Ok(())
}
