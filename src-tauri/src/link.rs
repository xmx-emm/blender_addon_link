use std::fs;
use std::path::Path;

/// 创建 NTFS junction：from 为真实插件目录，to 为 Blender 插件目录下的链接路径
#[tauri::command]
pub fn link_dir(from: &str, to: &str) -> Result<(), String> {
    let src = Path::new(from);
    if !src.is_dir() {
        return Err(format!("源目录不存在: {from}"));
    }
    let dst = Path::new(to);
    if dst.exists() {
        return Err(format!("目标已存在: {to}"));
    }
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {e}"))?;
        }
    }
    junction::create(src, dst).map_err(|e| format!("创建链接失败: {e}"))
}

/// 仅删除链接（junction/symlink），拒绝删除真实目录
#[tauri::command]
pub fn unlink_dir(ud: &str) -> Result<bool, String> {
    let p = Path::new(ud);
    let meta = fs::symlink_metadata(p).map_err(|e| format!("读取目录信息失败: {e}"))?;
    if !meta.file_type().is_symlink() {
        return Err("目标不是链接目录，为安全起见已拒绝删除".into());
    }
    // RemoveDirectory 只移除 junction 本身，不影响链接目标内容
    fs::remove_dir(p).map_err(|e| format!("删除链接失败: {e}"))?;
    Ok(!p.exists())
}

/// 删除真实目录（前端需二次确认后才调用）
#[tauri::command]
pub fn remove_real_dir(path: &str) -> Result<bool, String> {
    let p = Path::new(path);
    let meta = fs::symlink_metadata(p).map_err(|e| format!("读取目录信息失败: {e}"))?;
    if meta.file_type().is_symlink() {
        // 是链接就走安全删除
        return unlink_dir(path);
    }
    fs::remove_dir_all(p).map_err(|e| format!("删除目录失败: {e}"))?;
    Ok(!p.exists())
}

#[tauri::command]
pub fn is_symbolic_link(path: &str) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(serde::Serialize)]
pub struct AddonScan {
    pub path: String,
    pub is_extension: bool,
}

#[derive(serde::Serialize, Default)]
pub struct AddonMeta {
    pub name: String,
    pub version: String,
    /// 插件声明的最低 Blender 版本，如 "4.2"
    pub blender_min: String,
}

/// 从引号字符串值中取内容：key = "value" / "key": "value" / 'key': 'value'
fn quoted_value_after(text: &str, from: usize) -> Option<String> {
    let rest = &text[from..];
    let start = rest.find(['"', '\''])?;
    let quote = rest.as_bytes()[start] as char;
    let start = start + 1;
    let end = rest[start..].find(quote)? + start;
    Some(rest[start..end].to_string())
}

/// 提取 "(4, 2, 0)" 形式的版本元组，返回 "4.2.0"
fn tuple_after(text: &str, from: usize) -> Option<String> {
    let rest = &text[from..];
    let open = rest.find('(')?;
    let close = rest[open..].find(')')? + open;
    let nums: Vec<String> = rest[open + 1..close]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        .collect();
    if nums.is_empty() {
        None
    } else {
        Some(nums.join("."))
    }
}

fn parse_manifest(text: &str) -> AddonMeta {
    let mut meta = AddonMeta::default();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, _)) = line.split_once('=') else { continue };
        match key.trim() {
            "name" => meta.name = quoted_value_after(line, 0).unwrap_or_default(),
            "version" => meta.version = quoted_value_after(line, 0).unwrap_or_default(),
            "blender_version_min" => {
                meta.blender_min = quoted_value_after(line, 0).unwrap_or_default()
            }
            _ => {}
        }
    }
    meta
}

fn parse_bl_info(text: &str) -> AddonMeta {
    let mut meta = AddonMeta::default();
    // 只在 bl_info 字典附近搜索，避免匹配到代码其他位置
    let Some(start) = text.find("bl_info") else { return meta };
    let region = &text[start..text.len().min(start + 4000)];
    for key in ["\"name\"", "'name'"] {
        if let Some(p) = region.find(key) {
            if let Some(v) = quoted_value_after(region, p + key.len()) {
                meta.name = v;
                break;
            }
        }
    }
    for key in ["\"version\"", "'version'"] {
        if let Some(p) = region.find(key) {
            if let Some(v) = tuple_after(region, p + key.len()) {
                meta.version = v;
                break;
            }
        }
    }
    for key in ["\"blender\"", "'blender'"] {
        if let Some(p) = region.find(key) {
            if let Some(v) = tuple_after(region, p + key.len()) {
                meta.blender_min = v;
                break;
            }
        }
    }
    meta
}

/// 读取插件的名称/版本/最低 Blender 版本（manifest 优先，其次 bl_info）
#[tauri::command]
pub fn read_addon_meta(path: &str) -> AddonMeta {
    let dir = Path::new(path);
    let manifest = dir.join("blender_manifest.toml");
    if let Ok(text) = fs::read_to_string(&manifest) {
        return parse_manifest(&text);
    }
    let init = dir.join("__init__.py");
    if let Ok(text) = fs::read_to_string(&init) {
        return parse_bl_info(&text);
    }
    AddonMeta::default()
}

fn scan_one(dir: &Path, depth: u32, out: &mut Vec<AddonScan>) {
    let init = dir.join("__init__.py");
    let manifest = dir.join("blender_manifest.toml");
    if manifest.is_file() {
        out.push(AddonScan {
            path: dir.to_string_lossy().to_string(),
            is_extension: true,
        });
        return;
    }
    if init.is_file() {
        out.push(AddonScan {
            path: dir.to_string_lossy().to_string(),
            is_extension: false,
        });
        return;
    }
    // 不是插件目录时向下找一层（可能拖入的是包含多个插件的父目录）
    if depth < 1 {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    scan_one(&p, depth + 1, out);
                }
            }
        }
    }
}

/// 识别拖入路径中的 Blender 插件目录（传统 addon / 新版扩展）
#[tauri::command]
pub fn scan_addon_paths(paths: Vec<String>) -> Vec<AddonScan> {
    let mut out = vec![];
    for p in paths {
        let dir = Path::new(&p);
        if dir.is_dir() {
            scan_one(dir, 0, &mut out);
        }
    }
    out
}

/// 检查一批目标路径的存在与链接状态
#[derive(serde::Serialize)]
pub struct LinkStatus {
    pub exists: bool,
    pub is_link: bool,
    pub target: Option<String>,
}

#[tauri::command]
pub fn check_link_status(paths: Vec<String>) -> Vec<LinkStatus> {
    paths
        .into_iter()
        .map(|p| {
            let path = Path::new(&p);
            match fs::symlink_metadata(path) {
                Ok(meta) => {
                    let is_link = meta.file_type().is_symlink();
                    let target = if is_link {
                        fs::read_link(path).ok().map(|t| {
                            let s = t.to_string_lossy().to_string();
                            s.strip_prefix(r"\\?\").map(|x| x.to_string()).unwrap_or(s)
                        })
                    } else {
                        None
                    };
                    LinkStatus {
                        exists: true,
                        is_link,
                        target,
                    }
                }
                Err(_) => LinkStatus {
                    exists: false,
                    is_link: false,
                    target: None,
                },
            }
        })
        .collect()
}

/// 读取链接指向的真实路径（用于 UI 展示链接来源）
#[tauri::command]
pub fn read_link_target(path: &str) -> Result<String, String> {
    fs::read_link(path)
        .map(|p| {
            let s = p.to_string_lossy().to_string();
            // junction 目标常带 \\?\ 前缀，去掉便于阅读
            s.strip_prefix(r"\\?\").map(|x| x.to_string()).unwrap_or(s)
        })
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_bl_info, parse_manifest};

    #[test]
    fn manifest_meta() {
        let text = r#"
schema_version = "1.0.0"
id = "my_tool"
version = "2.1.0"
name = "My Tool"
tagline = "Does things"
blender_version_min = "4.2.0"
license = ["SPDX:GPL-3.0-or-later"]
"#;
        let m = parse_manifest(text);
        assert_eq!(m.name, "My Tool");
        assert_eq!(m.version, "2.1.0");
        assert_eq!(m.blender_min, "4.2.0");
    }

    #[test]
    fn bl_info_meta() {
        let text = r#"
# some header comment
bl_info = {
    "name": "Simple Deform Helper",
    "author": "someone",
    "version": (0, 2, 33),
    "blender": (3, 0, 0),
    "category": "3D View",
}

def register():
    pass
"#;
        let m = parse_bl_info(text);
        assert_eq!(m.name, "Simple Deform Helper");
        assert_eq!(m.version, "0.2.33");
        assert_eq!(m.blender_min, "3.0.0");
    }

    #[test]
    fn bl_info_single_quotes() {
        let text = "bl_info = {'name': 'X', 'blender': (4, 2, 0)}";
        let m = parse_bl_info(text);
        assert_eq!(m.name, "X");
        assert_eq!(m.blender_min, "4.2.0");
    }
}
