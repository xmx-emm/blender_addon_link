use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use crate::procutil::hidden_command;

/// 正在运行的渲染任务：job_id -> 子进程 pid（支持多任务并行）
static REGISTRY: LazyLock<Mutex<HashMap<String, u32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// 被请求取消的 job_id 集合
static CANCELLED: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn is_cancelled(job_id: &str) -> bool {
    CANCELLED
        .lock()
        .map(|s| s.contains(job_id))
        .unwrap_or(false)
}

fn clear_cancel(job_id: &str) {
    if let Ok(mut s) = CANCELLED.lock() {
        s.remove(job_id);
    }
}

#[derive(Deserialize)]
pub struct RenderJobSpec {
    pub id: String,
    pub exe: String,
    pub blend: String,
    /// animation | range | frame
    pub mode: String,
    pub frame_start: Option<i32>,
    pub frame_end: Option<i32>,
    pub frame: Option<i32>,
    pub scene: Option<String>,
    pub engine: Option<String>,
    pub output: Option<String>,
    pub extra_args: Option<Vec<String>>,
}

#[derive(Serialize, Clone)]
struct LogEvent {
    job_id: String,
    line: String,
}

#[derive(Serialize, Clone)]
struct ProgressEvent {
    job_id: String,
    frame: Option<i32>,
    mem_mb: Option<f64>,
    sample: Option<u32>,
    sample_total: Option<u32>,
    saved: Option<String>,
    saved_count: u32,
    elapsed_seconds: f64,
}

#[derive(Serialize)]
pub struct RenderOutcome {
    pub code: i32,
    pub success: bool,
    pub cancelled: bool,
    pub saved_count: u32,
    pub seconds: f64,
    pub tail: Vec<String>,
}

/// 提取进度字段。兼容两种格式：
/// 经典: "Fra:123 Mem:45.2M (Peak ..) | ... | Sample 64/128"
/// 5.x:  "00:03.1  render           | Fra: 1 | Mem: 151M | Sample 80/4096"
fn parse_progress_line(line: &str) -> (Option<i32>, Option<f64>, Option<u32>, Option<u32>) {
    let mut frame = None;
    let mut mem = None;
    let mut sample = None;
    let mut sample_total = None;
    if let Some(pos) = line.find("Fra:") {
        let rest = line[pos + 4..].trim_start();
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        frame = num.parse::<i32>().ok();
    }
    if frame.is_some() {
        if let Some(pos) = line.find("Mem:") {
            let rest = line[pos + 4..].trim_start();
            let m: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            mem = m.parse::<f64>().ok();
        }
    }
    // Cycles: "... Sample 64/128"
    if let Some(pos) = line.rfind("Sample ") {
        let seg = &line[pos + 7..];
        if let Some((a, b)) = seg.split_once('/') {
            let a: String = a.chars().take_while(|c| c.is_ascii_digit()).collect();
            let b: String = b.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let (Ok(x), Ok(y)) = (a.parse(), b.parse()) {
                sample = Some(x);
                sample_total = Some(y);
            }
        }
    }
    // EEVEE: "Rendering 12 / 64 samples"
    if sample.is_none() {
        if let Some(pos) = line.find("Rendering ") {
            let seg = &line[pos + 10..];
            if let Some(end) = seg.find(" samples") {
                if let Some((a, b)) = seg[..end].split_once(" / ") {
                    if let (Ok(x), Ok(y)) = (a.trim().parse(), b.trim().parse()) {
                        sample = Some(x);
                        sample_total = Some(y);
                    }
                }
            }
        }
    }
    (frame, mem, sample, sample_total)
}

fn build_args(spec: &RenderJobSpec) -> Result<Vec<String>, String> {
    let mut args: Vec<String> = vec!["-b".into(), spec.blend.clone(), "-y".into()];
    if let Some(engine) = spec.engine.as_deref().filter(|s| !s.is_empty()) {
        args.push("-E".into());
        args.push(engine.into());
    }
    if let Some(scene) = spec.scene.as_deref().filter(|s| !s.is_empty()) {
        args.push("-S".into());
        args.push(scene.into());
    }
    if let Some(out) = spec.output.as_deref().filter(|s| !s.is_empty()) {
        args.push("-o".into());
        args.push(out.into());
    }
    match spec.mode.as_str() {
        "animation" => args.push("-a".into()),
        "range" => {
            let (s, e) = (
                spec.frame_start.ok_or("缺少起始帧")?,
                spec.frame_end.ok_or("缺少结束帧")?,
            );
            if e < s {
                return Err("结束帧不能小于起始帧".into());
            }
            args.push("-s".into());
            args.push(s.to_string());
            args.push("-e".into());
            args.push(e.to_string());
            args.push("-a".into());
        }
        "frame" => {
            args.push("-f".into());
            args.push(spec.frame.ok_or("缺少帧号")?.to_string());
        }
        m => return Err(format!("未知渲染模式: {m}")),
    }
    if let Some(extra) = &spec.extra_args {
        for a in extra {
            if !a.trim().is_empty() {
                args.push(a.trim().to_string());
            }
        }
    }
    Ok(args)
}

fn kill_tree(pid: u32) {
    let mut cmd = hidden_command("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
    let _ = cmd.output();
}

/// 渲染期间阻止系统休眠（不阻止关屏）；Drop 时恢复，覆盖所有退出路径
struct KeepAwake;

impl KeepAwake {
    fn new() -> Self {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Power::{
                SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
            };
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
        }
        KeepAwake
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS};
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

fn do_render(app: &AppHandle, spec: RenderJobSpec) -> Result<RenderOutcome, String> {
    if !std::path::Path::new(&spec.exe).is_file() {
        return Err(format!("blender.exe 不存在: {}", spec.exe));
    }
    if !std::path::Path::new(&spec.blend).is_file() {
        return Err(format!(".blend 文件不存在: {}", spec.blend));
    }
    let args = build_args(&spec)?;
    let job_id = spec.id.clone();
    let start = Instant::now();
    let _awake = KeepAwake::new();

    if is_cancelled(&job_id) {
        return Ok(RenderOutcome {
            code: -1,
            success: false,
            cancelled: true,
            saved_count: 0,
            seconds: start.elapsed().as_secs_f64(),
            tail: vec![],
        });
    }

    let mut cmd = hidden_command(&spec.exe);
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = cmd.spawn().map_err(|e| format!("启动 Blender 失败: {e}"))?;
    if let Ok(mut reg) = REGISTRY.lock() {
        reg.insert(job_id.clone(), child.id());
    }

    // A cancel request may arrive after the reservation but before Blender has
    // started emitting output. Handle it immediately so a silent process does
    // not run until completion while the render loop waits on its pipes.
    if is_cancelled(&job_id) {
        kill_tree(child.id());
        let _ = child.kill();
        let code = child
            .wait()
            .ok()
            .and_then(|status| status.code())
            .unwrap_or(-1);
        if let Ok(mut reg) = REGISTRY.lock() {
            reg.remove(&job_id);
        }
        return Ok(RenderOutcome {
            code,
            success: false,
            cancelled: true,
            saved_count: 0,
            seconds: start.elapsed().as_secs_f64(),
            tail: vec![],
        });
    }

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel::<String>();
    let tx2 = tx.clone();
    let h1 = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let h2 = std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tx2.send(line).is_err() {
                break;
            }
        }
    });

    let mut saved_count: u32 = 0;
    let mut tail: Vec<String> = vec![];
    let mut last_emit = Instant::now();
    // 行接收循环：管道全部关闭后 rx 断开
    for line in rx.iter() {
        if is_cancelled(&job_id) {
            break;
        }
        let (frame, mem, sample, sample_total) = parse_progress_line(&line);
        let mut saved: Option<String> = None;
        if let Some(pos) = line.find("Saved: ") {
            saved_count += 1;
            saved = Some(line[pos + 7..].trim().trim_matches('\'').to_string());
        }
        tail.push(line.clone());
        if tail.len() > 400 {
            tail.drain(..200);
        }
        let _ = app.emit("render-log", LogEvent { job_id: job_id.clone(), line: line.clone() });
        // 进度事件节流（保存事件必发）
        let significant = saved.is_some() || frame.is_some();
        if significant || last_emit.elapsed().as_millis() > 300 {
            last_emit = Instant::now();
            let _ = app.emit(
                "render-progress",
                ProgressEvent {
                    job_id: job_id.clone(),
                    frame,
                    mem_mb: mem,
                    sample,
                    sample_total,
                    saved,
                    saved_count,
                    elapsed_seconds: start.elapsed().as_secs_f64(),
                },
            );
        }
    }

    let cancelled = is_cancelled(&job_id);
    if cancelled {
        kill_tree(child.id());
        let _ = child.kill();
    }
    let status = child.wait().map_err(|e| format!("等待进程失败: {e}"))?;
    let _ = h1.join();
    let _ = h2.join();
    if let Ok(mut reg) = REGISTRY.lock() {
        reg.remove(&job_id);
    }

    let code = status.code().unwrap_or(-1);
    Ok(RenderOutcome {
        code,
        success: !cancelled && code == 0,
        cancelled,
        saved_count,
        seconds: start.elapsed().as_secs_f64(),
        tail: tail.into_iter().rev().take(60).rev().collect(),
    })
}

/// 执行单个渲染任务（前端并发池按任务调用，支持多任务并行）
#[tauri::command]
pub async fn render_run(app: AppHandle, spec: RenderJobSpec) -> Result<RenderOutcome, String> {
    let job_id = spec.id.clone();
    {
        // 先占位注册（pid=0），防止同一任务被重复启动；spawn 成功后由 do_render 写入真实 pid
        let mut reg = REGISTRY.lock().map_err(|_| "任务注册表锁异常".to_string())?;
        if reg.contains_key(&job_id) {
            return Err("该任务已在渲染中".into());
        }
        reg.insert(job_id.clone(), 0);
    }
    let result = tauri::async_runtime::spawn_blocking(move || do_render(&app, spec))
        .await
        .map_err(|e| format!("渲染线程异常: {e}"));
    // 兜底清理：正常路径 do_render 已移除，异常路径（参数错误/启动失败等）这里移除占位
    if let Ok(mut reg) = REGISTRY.lock() {
        reg.remove(&job_id);
    }
    clear_cancel(&job_id);
    result?
}

/// 定向取消某个渲染任务（杀它的整棵进程树）
#[tauri::command]
pub fn render_cancel(job_id: String) {
    if let Ok(mut s) = CANCELLED.lock() {
        s.insert(job_id.clone());
    }
    let pid = REGISTRY.lock().ok().and_then(|reg| reg.get(&job_id).copied());
    if let Some(pid) = pid {
        if pid != 0 {
            kill_tree(pid);
        }
    }
}

/// 取消全部正在运行的渲染任务
#[tauri::command]
pub fn render_cancel_all() {
    let entries: Vec<(String, u32)> = REGISTRY
        .lock()
        .map(|reg| reg.iter().map(|(k, v)| (k.clone(), *v)).collect())
        .unwrap_or_default();
    if let Ok(mut s) = CANCELLED.lock() {
        for (id, _) in &entries {
            s.insert(id.clone());
        }
    }
    for (_, pid) in entries {
        if pid != 0 {
            kill_tree(pid);
        }
    }
}

/// 计划关机（延迟秒数内可撤销）
#[tauri::command]
pub fn schedule_shutdown(seconds: u32) -> Result<(), String> {
    let mut cmd = hidden_command("shutdown");
    cmd.args(["/s", "/t", &seconds.to_string()]);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

#[tauri::command]
pub fn abort_shutdown() -> Result<(), String> {
    let mut cmd = hidden_command("shutdown");
    cmd.arg("/a");
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_progress_line;

    // 样例均取自本机 Blender 4.2 / 5.2 实测输出
    #[test]
    fn parse_52_cycles() {
        let line = "00:16.359  render           | Fra: 1 | Remaining: 11:22.26 | Mem: 151M | Sample 80/4096";
        assert_eq!(
            parse_progress_line(line),
            (Some(1), Some(151.0), Some(80), Some(4096))
        );
    }

    #[test]
    fn parse_52_eevee() {
        let line = "00:03.703  render           | Fra: 1 | Rendering 25 / 64 samples";
        assert_eq!(parse_progress_line(line), (Some(1), None, Some(25), Some(64)));
    }

    #[test]
    fn parse_classic() {
        let line = "Fra:123 Mem:45.20M (Peak 67.80M) | Time:00:12.34 | Sample 64/128";
        assert_eq!(
            parse_progress_line(line),
            (Some(123), Some(45.20), Some(64), Some(128))
        );
    }

    #[test]
    fn parse_plain() {
        assert_eq!(parse_progress_line("Blender quit"), (None, None, None, None));
    }
}
