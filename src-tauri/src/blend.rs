use base64::Engine;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

/// .blend 解析：支持 12 字节旧头(BHead4/SmallBHead8) 与 5.0+ 17 字节新头(LargeBHead8)，
/// 以及 无压缩 / gzip(≤2.9x) / zstd(3.0+) 三种存储。

#[derive(Serialize, Clone)]
pub struct CategoryStat {
    pub code: String,
    pub label: String,
    pub count: u64,
    pub bytes: u64,
}

#[derive(Serialize, Clone)]
pub struct BlockStat {
    pub name: String,
    pub code: String,
    pub label: String,
    pub bytes: u64,
    pub data_blocks: u64,
}

#[derive(Serialize, Clone)]
pub struct RendInfo {
    pub scene: String,
    pub start: i32,
    pub end: i32,
}

#[derive(Serialize, Clone)]
pub struct Thumb {
    pub width: i32,
    pub height: i32,
    /// 原始 RGBA 像素（自下而上存储），前端用 canvas 绘制并翻转
    pub rgba_base64: String,
}

#[derive(Serialize)]
pub struct BlendAnalysis {
    pub file: String,
    pub file_size: u64,
    pub compression: String,
    pub blender_version: String,
    pub pointer_size: u8,
    pub header_kind: String,
    pub uncompressed_size: u64,
    pub categories: Vec<CategoryStat>,
    pub top_blocks: Vec<BlockStat>,
    pub scenes: Vec<RendInfo>,
    pub thumbnail: Option<Thumb>,
    pub warnings: Vec<String>,
}

fn code_label(code: &str) -> &'static str {
    match code {
        "ME" => "网格",
        "OB" => "物体",
        "MA" => "材质",
        "IM" => "图像",
        "TE" => "纹理",
        "SC" => "场景",
        "WO" => "世界环境",
        "BR" => "笔刷",
        "AC" => "动作(动画)",
        "NT" => "节点树",
        "LI" => "链接库",
        "GR" => "集合",
        "CA" => "相机",
        "LA" => "灯光",
        "SO" => "声音",
        "VF" => "字体",
        "PT" => "点云",
        "VO" => "体积",
        "GD" => "蜡笔(旧版)",
        "GP" => "蜡笔",
        "CU" => "曲线",
        "CV" => "毛发曲线",
        "LT" => "晶格",
        "AR" => "骨架",
        "KE" => "形态键",
        "TX" => "文本",
        "MC" => "影片剪辑",
        "MS" => "遮罩",
        "MB" => "融球",
        "PA" => "粒子设置",
        "PL" => "调色板",
        "PC" => "绘制曲线",
        "LP" => "光照探头",
        "LS" => "线条样式",
        "SK" => "扬声器",
        "WM" => "窗口管理",
        "WS" => "工作区",
        "SR" => "屏幕布局",
        "CF" => "缓存文件",
        "IP" => "旧版动画(Ipo)",
        "SN" => "界面数据",
        _ => "其他数据",
    }
}

enum BHeadKind {
    BHead4,
    Small8,
    Large8,
}

struct Header {
    ptr_size: u8,
    version: u32,
    kind: BHeadKind,
}

struct BHead {
    code: [u8; 4],
    len: u64,
}

fn read_u16_ascii(b: &[u8]) -> Option<u32> {
    let mut v: u32 = 0;
    for &c in b {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v * 10 + (c - b'0') as u32;
    }
    Some(v)
}

fn parse_header(r: &mut dyn Read) -> Result<Header, String> {
    let mut magic = [0u8; 7];
    r.read_exact(&mut magic)
        .map_err(|_| "文件太短，不是有效的 .blend 文件".to_string())?;
    if &magic != b"BLENDER" {
        return Err("不是 .blend 文件（缺少 BLENDER 标识）".into());
    }
    let mut b = [0u8; 5];
    r.read_exact(&mut b).map_err(|_| "文件头不完整".to_string())?;
    match b[0] {
        b'_' | b'-' => {
            // 旧 12 字节头
            let ptr_size = if b[0] == b'_' { 4 } else { 8 };
            match b[1] {
                b'v' => {}
                b'V' => return Err("大端字节序的旧版文件（PowerPC 时代），不支持解析".into()),
                _ => return Err("无法识别的文件头字节序标记".into()),
            }
            let version =
                read_u16_ascii(&b[2..5]).ok_or_else(|| "文件头版本号无效".to_string())?;
            Ok(Header {
                ptr_size,
                version,
                kind: if ptr_size == 4 {
                    BHeadKind::BHead4
                } else {
                    BHeadKind::Small8
                },
            })
        }
        b'0'..=b'9' => {
            // 5.0+ 新头："BLENDER" + 长度2位 + '-' + 格式版本2位 + 'v' + 版本4位 = 17 字节
            let header_len =
                read_u16_ascii(&b[0..2]).ok_or_else(|| "新版文件头长度无效".to_string())?;
            if header_len != 17 {
                return Err(format!(
                    "未知的文件头长度 {header_len}，可能来自比本工具更新的 Blender"
                ));
            }
            if b[2] != b'-' {
                return Err("新版文件头格式异常".into());
            }
            let mut c = [0u8; 5];
            r.read_exact(&mut c).map_err(|_| "新版文件头不完整".to_string())?;
            if c[0] != b'v' {
                return Err("新版文件头缺少版本标记".into());
            }
            let version =
                read_u16_ascii(&c[1..5]).ok_or_else(|| "新版文件头版本号无效".to_string())?;
            Ok(Header {
                ptr_size: 8,
                version,
                kind: BHeadKind::Large8,
            })
        }
        _ => Err("无法识别的 .blend 文件头".into()),
    }
}

fn version_string(v: u32) -> String {
    format!("{}.{}", v / 100, v % 100)
}

fn read_i32(r: &mut dyn Read) -> std::io::Result<i32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}
fn read_u64(r: &mut dyn Read) -> std::io::Result<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}
fn read_i64(r: &mut dyn Read) -> std::io::Result<i64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

/// 读一个块头；文件干净结束返回 None
fn read_bhead(r: &mut dyn Read, kind: &BHeadKind) -> Result<Option<BHead>, String> {
    let mut code = [0u8; 4];
    match r.read_exact(&mut code) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(format!("读取块头失败: {e}")),
    }
    let len: i64 = match kind {
        BHeadKind::BHead4 => {
            let len = read_i32(r).map_err(|e| e.to_string())?;
            let _old = read_i32(r).map_err(|e| e.to_string())?;
            let _sdna = read_i32(r).map_err(|e| e.to_string())?;
            let _nr = read_i32(r).map_err(|e| e.to_string())?;
            len as i64
        }
        BHeadKind::Small8 => {
            let len = read_i32(r).map_err(|e| e.to_string())?;
            let _old = read_u64(r).map_err(|e| e.to_string())?;
            let _sdna = read_i32(r).map_err(|e| e.to_string())?;
            let _nr = read_i32(r).map_err(|e| e.to_string())?;
            len as i64
        }
        BHeadKind::Large8 => {
            // 注意：LargeBHead8 字段顺序与旧结构不同
            let _sdna = read_i32(r).map_err(|e| e.to_string())?;
            let _old = read_u64(r).map_err(|e| e.to_string())?;
            let len = read_i64(r).map_err(|e| e.to_string())?;
            let _nr = read_i64(r).map_err(|e| e.to_string())?;
            len
        }
    };
    if len < 0 {
        return Err("块长度为负，文件可能已损坏".into());
    }
    Ok(Some(BHead {
        code,
        len: len as u64,
    }))
}

fn skip(r: &mut dyn Read, n: u64) -> Result<(), String> {
    let copied = std::io::copy(&mut r.take(n), &mut std::io::sink())
        .map_err(|e| format!("跳过数据失败: {e}"))?;
    if copied != n {
        return Err("文件提前结束".into());
    }
    Ok(())
}

fn read_at_most(r: &mut dyn Read, n: u64) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(n.min(1 << 20) as usize);
    r.take(n)
        .read_to_end(&mut buf)
        .map_err(|e| format!("读取块数据失败: {e}"))?;
    Ok(buf)
}

/// 是否为 ID 数据块（两个大写字母 + 两个空字节）
fn id_code(code: &[u8; 4]) -> Option<String> {
    if code[2] == 0
        && code[3] == 0
        && code[0].is_ascii_uppercase()
        && (code[1].is_ascii_uppercase() || code[1].is_ascii_digit())
    {
        Some(String::from_utf8_lossy(&code[0..2]).to_string())
    } else {
        None
    }
}

/// 在 ID 块数据前部按指针对齐寻找 name 字段（以类型双字母开头、可打印、null 结尾）
/// Blender 4.5+ 名字上限扩到 258 字节，搜索窗口取 400
fn extract_id_name(data: &[u8], code2: &str, ptr: usize) -> Option<String> {
    let cb = code2.as_bytes();
    let limit = data.len().min(400);
    let mut off = ptr * 2;
    while off + 3 <= limit {
        if data[off] == cb[0] && data[off + 1] == cb[1] {
            let name_start = off + 2;
            let max_end = data.len().min(off + 258);
            let mut end = name_start;
            let mut valid = true;
            while end < max_end {
                let c = data[end];
                if c == 0 {
                    break;
                }
                // 允许可打印 ASCII 与 UTF-8 多字节（中文名常见）
                if c < 0x20 || c == 0x7f {
                    valid = false;
                    break;
                }
                end += 1;
            }
            if valid && end < max_end && end > name_start {
                return Some(String::from_utf8_lossy(&data[name_start..end]).to_string());
            }
        }
        off += ptr;
    }
    None
}

struct Agg {
    count: u64,
    bytes: u64,
}

pub fn open_blend_stream(path: &Path) -> Result<(Box<dyn Read>, String, u64), String> {
    let meta =
        std::fs::metadata(path).map_err(|e| format!("读取文件信息失败: {e}"))?;
    let file = std::fs::File::open(path).map_err(|e| format!("打开文件失败: {e}"))?;
    let mut br = std::io::BufReader::with_capacity(256 * 1024, file);
    let mut magic = [0u8; 4];
    br.read_exact(&mut magic).map_err(|_| "文件太短".to_string())?;
    // 回到文件头
    use std::io::Seek;
    br.seek(std::io::SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let (reader, compression): (Box<dyn Read>, String) = if magic.starts_with(b"BLEN") {
        (Box::new(br), "无压缩".into())
    } else if magic[0] == 0x1f && magic[1] == 0x8b {
        (
            Box::new(flate2::read::MultiGzDecoder::new(br)),
            "gzip".into(),
        )
    } else if magic == [0x28, 0xb5, 0x2f, 0xfd] || magic == [0x5e, 0x2a, 0x4d, 0x18] {
        // zstd 数据帧或 seekable 跳过帧开头
        let dec = zstd::stream::read::Decoder::new(br)
            .map_err(|e| format!("初始化 zstd 解压失败: {e}"))?;
        (Box::new(dec), "zstd".into())
    } else {
        return Err("无法识别的文件格式（既不是 .blend 也不是其压缩形式）".into());
    };
    Ok((reader, compression, meta.len()))
}

fn parse_rend(data: &[u8]) -> Option<RendInfo> {
    if data.len() < 8 {
        return None;
    }
    let start = i32::from_le_bytes(data[0..4].try_into().ok()?);
    let end = i32::from_le_bytes(data[4..8].try_into().ok()?);
    let name_bytes = &data[8..];
    let nul = name_bytes.iter().position(|&c| c == 0).unwrap_or(name_bytes.len());
    let scene = String::from_utf8_lossy(&name_bytes[..nul]).to_string();
    Some(RendInfo { scene, start, end })
}

fn parse_thumb(data: &[u8]) -> Option<Thumb> {
    if data.len() < 8 {
        return None;
    }
    let w = i32::from_le_bytes(data[0..4].try_into().ok()?);
    let h = i32::from_le_bytes(data[4..8].try_into().ok()?);
    if !(1..=2048).contains(&w) || !(1..=2048).contains(&h) {
        return None;
    }
    let need = (w as usize) * (h as usize) * 4;
    if data.len() < 8 + need {
        return None;
    }
    Some(Thumb {
        width: w,
        height: h,
        rgba_base64: base64::engine::general_purpose::STANDARD.encode(&data[8..8 + need]),
    })
}

fn special_label(code: &str) -> Option<&'static str> {
    match code {
        "DNA1" => Some("结构描述(DNA)"),
        "GLOB" => Some("全局信息"),
        "USER" => Some("用户设置"),
        "REND" => Some("渲染信息"),
        "TEST" => Some("缩略图"),
        _ => None,
    }
}

fn do_analyze(path_str: &str) -> Result<BlendAnalysis, String> {
    let path = Path::new(path_str);
    let (mut reader, compression, file_size) = open_blend_stream(path)?;
    let header = parse_header(&mut reader.as_mut())?;

    let head_size: u64 = match header.kind {
        BHeadKind::BHead4 => 20,
        BHeadKind::Small8 => 24,
        BHeadKind::Large8 => 32,
    };
    let mut uncompressed: u64 = if matches!(header.kind, BHeadKind::Large8) { 17 } else { 12 };
    let mut warnings: Vec<String> = vec![];
    let mut cats: BTreeMap<String, Agg> = BTreeMap::new();
    let mut blocks: Vec<BlockStat> = vec![];
    let mut scenes: Vec<RendInfo> = vec![];
    let mut thumbnail: Option<Thumb> = None;
    // 当前 DATA 归属的 ID 块（索引到 blocks / 类别码）
    let mut owner: Option<(usize, String)> = None;

    loop {
        let bh = match read_bhead(&mut reader.as_mut(), &header.kind) {
            Ok(Some(b)) => b,
            Ok(None) => {
                warnings.push("文件没有以 ENDB 块正常结束（可能被截断）".into());
                break;
            }
            Err(e) => {
                warnings.push(format!("解析中断: {e}（以下为已解析部分的统计）"));
                break;
            }
        };
        uncompressed += head_size + bh.len;
        let code_str: String = String::from_utf8_lossy(
            &bh.code[..bh.code.iter().position(|&c| c == 0).unwrap_or(4)],
        )
        .to_string();
        if &bh.code == b"ENDB" {
            break;
        }
        if &bh.code == b"DATA" {
            if let Some((idx, cat)) = &owner {
                blocks[*idx].bytes += bh.len;
                blocks[*idx].data_blocks += 1;
                let e = cats.entry(cat.clone()).or_insert(Agg { count: 0, bytes: 0 });
                e.bytes += bh.len;
            } else {
                let e = cats
                    .entry("DATA".into())
                    .or_insert(Agg { count: 0, bytes: 0 });
                e.count += 1;
                e.bytes += bh.len;
            }
            skip(&mut reader.as_mut(), bh.len)?;
            continue;
        }
        if &bh.code == b"REND" {
            let data = read_at_most(&mut reader.as_mut(), bh.len.min(4096))?;
            if bh.len > 4096 {
                skip(&mut reader.as_mut(), bh.len - 4096)?;
            }
            if let Some(r) = parse_rend(&data) {
                scenes.push(r);
            }
            let e = cats.entry("REND".into()).or_insert(Agg { count: 0, bytes: 0 });
            e.count += 1;
            e.bytes += bh.len;
            owner = None;
            continue;
        }
        if &bh.code == b"TEST" {
            let cap = bh.len.min(32 * 1024 * 1024);
            let data = read_at_most(&mut reader.as_mut(), cap)?;
            if bh.len > cap {
                skip(&mut reader.as_mut(), bh.len - cap)?;
            }
            if thumbnail.is_none() {
                thumbnail = parse_thumb(&data);
            }
            let e = cats.entry("TEST".into()).or_insert(Agg { count: 0, bytes: 0 });
            e.count += 1;
            e.bytes += bh.len;
            owner = None;
            continue;
        }

        if let Some(code2) = id_code(&bh.code) {
            // ID 块：读前 512 字节做名字启发式，其余跳过
            let head_bytes = bh.len.min(512);
            let data = read_at_most(&mut reader.as_mut(), head_bytes)?;
            if bh.len > head_bytes {
                skip(&mut reader.as_mut(), bh.len - head_bytes)?;
            }
            let name = extract_id_name(&data, &code2, header.ptr_size as usize)
                .unwrap_or_else(|| format!("({})", code_label(&code2)));
            let e = cats.entry(code2.clone()).or_insert(Agg { count: 0, bytes: 0 });
            e.count += 1;
            e.bytes += bh.len;
            blocks.push(BlockStat {
                name,
                code: code2.clone(),
                label: code_label(&code2).to_string(),
                bytes: bh.len,
                data_blocks: 0,
            });
            owner = Some((blocks.len() - 1, code2));
        } else {
            // 其他特殊块（GLOB/USER/DNA1/未知）
            let e = cats
                .entry(code_str.clone())
                .or_insert(Agg { count: 0, bytes: 0 });
            e.count += 1;
            e.bytes += bh.len;
            skip(&mut reader.as_mut(), bh.len)?;
            owner = None;
        }
    }

    let mut categories: Vec<CategoryStat> = cats
        .into_iter()
        .map(|(code, a)| {
            let label = special_label(&code)
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if code == "DATA" {
                        "附属数据".to_string()
                    } else {
                        code_label(&code).to_string()
                    }
                });
            CategoryStat {
                code,
                label,
                count: a.count,
                bytes: a.bytes,
            }
        })
        .collect();
    categories.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    blocks.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    blocks.truncate(100);

    Ok(BlendAnalysis {
        file: path_str.to_string(),
        file_size,
        compression,
        blender_version: version_string(header.version),
        pointer_size: header.ptr_size,
        header_kind: if matches!(header.kind, BHeadKind::Large8) {
            "新格式(5.0+)".into()
        } else {
            "经典格式".into()
        },
        uncompressed_size: uncompressed,
        categories,
        top_blocks: blocks,
        scenes,
        thumbnail,
        warnings,
    })
}

/// 完整体积分析（重活，放 blocking 线程）
#[tauri::command]
pub async fn analyze_blend(path: String) -> Result<BlendAnalysis, String> {
    tauri::async_runtime::spawn_blocking(move || do_analyze(&path))
        .await
        .map_err(|e| format!("分析线程异常: {e}"))?
}

#[derive(Serialize)]
pub struct BlendMeta {
    pub blender_version: String,
    pub compression: String,
    pub file_size: u64,
    pub scenes: Vec<RendInfo>,
    pub thumbnail: Option<Thumb>,
}

fn do_meta(path_str: &str) -> Result<BlendMeta, String> {
    let path = Path::new(path_str);
    let (mut reader, compression, file_size) = open_blend_stream(path)?;
    let header = parse_header(&mut reader.as_mut())?;
    let mut scenes = vec![];
    let mut thumbnail = None;
    // REND/TEST 块都位于文件最前部，遇到其他块即可停止
    loop {
        let Some(bh) = read_bhead(&mut reader.as_mut(), &header.kind)? else {
            break;
        };
        match &bh.code {
            b"REND" => {
                let data = read_at_most(&mut reader.as_mut(), bh.len.min(4096))?;
                if bh.len > 4096 {
                    skip(&mut reader.as_mut(), bh.len - 4096)?;
                }
                if let Some(r) = parse_rend(&data) {
                    scenes.push(r);
                }
            }
            b"TEST" => {
                let cap = bh.len.min(32 * 1024 * 1024);
                let data = read_at_most(&mut reader.as_mut(), cap)?;
                if bh.len > cap {
                    skip(&mut reader.as_mut(), bh.len - cap)?;
                }
                thumbnail = parse_thumb(&data);
            }
            _ => break,
        }
    }
    Ok(BlendMeta {
        blender_version: version_string(header.version),
        compression,
        file_size,
        scenes,
        thumbnail,
    })
}

/// 轻量读取 .blend 元信息（帧范围/场景名/缩略图），供渲染队列使用，无需启动 Blender
#[tauri::command]
pub async fn blend_meta(path: String) -> Result<BlendMeta, String> {
    tauri::async_runtime::spawn_blocking(move || do_meta(&path))
        .await
        .map_err(|e| format!("读取线程异常: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用真实文件验证解析：`BLEND_TEST_FILE=... cargo test parse_env -- --nocapture`
    #[test]
    fn parse_env_file() {
        let Ok(p) = std::env::var("BLEND_TEST_FILE") else {
            return;
        };
        let r = do_analyze(&p).expect("解析失败");
        println!(
            "file={} version={} compression={} header={} ptr={} uncompressed={} cats={} blocks={} scenes={:?} thumb={} warnings={:?}",
            r.file,
            r.blender_version,
            r.compression,
            r.header_kind,
            r.pointer_size,
            r.uncompressed_size,
            r.categories.len(),
            r.top_blocks.len(),
            r.scenes.iter().map(|s| format!("{} {}-{}", s.scene, s.start, s.end)).collect::<Vec<_>>(),
            r.thumbnail.is_some(),
            r.warnings,
        );
        for c in r.categories.iter().take(8) {
            println!("  cat {} {} count={} bytes={}", c.code, c.label, c.count, c.bytes);
        }
        for b in r.top_blocks.iter().take(8) {
            println!("  blk {} [{}] bytes={} data={}", b.name, b.label, b.bytes, b.data_blocks);
        }
        assert!(!r.categories.is_empty());
        let meta = do_meta(&p).expect("meta 解析失败");
        println!("meta version={} scenes={}", meta.blender_version, meta.scenes.len());
    }
}
