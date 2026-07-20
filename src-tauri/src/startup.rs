use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

use crate::procutil::{hidden_command, run_with_timeout};

static CANCEL: AtomicBool = AtomicBool::new(false);

const LIST_MARKER: &str = "@@BL_ADDONS@@";
const TIMING_MARKER: &str = "@@BL_TIMING@@";

#[derive(Serialize, Deserialize, Clone)]
pub struct AddonTiming {
    pub module: String,
    pub display_name: String,
    pub seconds: f64,
    pub ok: bool,
    pub error: String,
}

#[derive(Serialize)]
pub struct StartupResult {
    pub warmup_seconds: f64,
    pub normal_seconds: Vec<f64>,
    pub factory_seconds: Vec<f64>,
    pub addons: Vec<AddonTiming>,
    pub notes: Vec<String>,
}

#[derive(Serialize, Clone)]
struct Progress {
    step: u32,
    total: u32,
    message: String,
}

fn emit_progress(app: &AppHandle, step: u32, total: u32, message: &str) {
    let _ = app.emit(
        "startup-progress",
        Progress {
            step,
            total,
            message: message.to_string(),
        },
    );
}

fn check_cancel() -> Result<(), String> {
    if CANCEL.load(Ordering::Relaxed) {
        Err("已取消".into())
    } else {
        Ok(())
    }
}

fn version_at_least(version: &str, major: u32, minor: u32) -> bool {
    let mut it = version.split('.');
    let a: u32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let b: u32 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    (a, b) >= (major, minor)
}

/// 跑一次退出型 Blender，返回墙钟秒数
fn timed_run(exe: &str, factory: bool, offline_ok: bool) -> Result<f64, String> {
    let mut cmd = hidden_command(exe);
    cmd.arg("-b");
    if factory {
        cmd.arg("--factory-startup");
    }
    if offline_ok {
        cmd.arg("--offline-mode");
    }
    cmd.args(["--python-expr", "import sys; sys.exit(0)"]);
    let out = run_with_timeout(cmd, 300, Some(&CANCEL))?;
    if out.code != 0 {
        return Err(format!(
            "Blender 退出码 {}：{}",
            out.code,
            out.stderr.lines().last().unwrap_or("")
        ));
    }
    Ok(out.seconds)
}

/// 读取当前启用的插件模块名列表
fn enabled_addons(exe: &str, offline_ok: bool) -> Result<Vec<String>, String> {
    let expr = format!(
        "import bpy, json, sys; print('{LIST_MARKER}' + json.dumps(list(bpy.context.preferences.addons.keys()))); sys.exit(0)"
    );
    let mut cmd = hidden_command(exe);
    cmd.arg("-b");
    if offline_ok {
        cmd.arg("--offline-mode");
    }
    cmd.args(["--python-expr", &expr]);
    let out = run_with_timeout(cmd, 300, Some(&CANCEL))?;
    for line in out.stdout.lines() {
        if let Some(json) = line.trim().strip_prefix(LIST_MARKER) {
            return serde_json::from_str::<Vec<String>>(json)
                .map_err(|e| format!("解析插件列表失败: {e}"));
        }
    }
    Err(format!(
        "未能从 Blender 输出中读取插件列表（退出码 {}）",
        out.code
    ))
}

/// 逐插件计时脚本：工厂启动下逐个 enable 并输出 JSON
fn timing_script(modules: &[String]) -> String {
    let json_list = serde_json::to_string(modules).unwrap_or_else(|_| "[]".into());
    format!(
        r#"import json, time, sys
import bpy
import addon_utils

# 防止任何插件在启用过程中把工厂设置写回用户偏好
try:
    bpy.context.preferences.use_preferences_save = False
except Exception:
    pass

MODULES = json.loads(r'''{json_list}''')

# 扩展(bl_ext.*)在工厂启动下需要先刷新仓库索引才能启用
try:
    addon_utils.extensions_refresh()
except Exception:
    pass

results = []
for name in MODULES:
    t0 = time.perf_counter()
    ok = True
    err = ""
    mod = None
    try:
        # default_set=True：部分插件注册时会访问自身偏好条目，不加入列表会报 KeyError
        mod = addon_utils.enable(name, default_set=True, persistent=False)
        ok = mod is not None
    except Exception as e:
        ok = False
        err = str(e)[:200]
    dt = time.perf_counter() - t0
    display = name
    try:
        import importlib
        m = importlib.import_module(name) if mod is None else mod
        if m is not None and hasattr(m, 'bl_info'):
            display = m.bl_info.get('name', name) or name
    except Exception:
        pass
    if display.startswith('bl_ext.'):
        display = display.rsplit('.', 1)[-1]
    results.append({{"module": name, "display_name": display, "seconds": dt, "ok": ok, "error": err}})

print("{TIMING_MARKER}" + json.dumps(results))
sys.stdout.flush()
sys.exit(0)
"#
    )
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn do_analyze(
    app: &AppHandle,
    exe: &str,
    version: &str,
    runs: u32,
) -> Result<StartupResult, String> {
    let offline_ok = version_at_least(version, 4, 2);
    let runs = runs.clamp(1, 5);
    let total = 1 + runs * 2 + 2;
    let mut step = 0u32;
    let mut notes: Vec<String> = vec![];

    emit_progress(app, step, total, "预热运行（首轮受杀毒软件与磁盘缓存影响，不计入结果）");
    let warmup = timed_run(exe, false, offline_ok)?;
    step += 1;

    let mut normal: Vec<f64> = vec![];
    for i in 0..runs {
        check_cancel()?;
        emit_progress(app, step, total, &format!("正常启动测量 {}/{}", i + 1, runs));
        normal.push(timed_run(exe, false, offline_ok)?);
        step += 1;
    }
    let mut factory: Vec<f64> = vec![];
    for i in 0..runs {
        check_cancel()?;
        emit_progress(app, step, total, &format!("纯净启动测量（--factory-startup）{}/{}", i + 1, runs));
        factory.push(timed_run(exe, true, offline_ok)?);
        step += 1;
    }

    check_cancel()?;
    emit_progress(app, step, total, "读取已启用插件列表");
    let modules = enabled_addons(exe, offline_ok)?;
    step += 1;

    let mut addons: Vec<AddonTiming> = vec![];
    if modules.is_empty() {
        notes.push("当前没有启用任何插件".into());
    } else {
        check_cancel()?;
        emit_progress(
            app,
            step,
            total,
            &format!("逐个测量 {} 个插件的加载耗时（可能较慢）", modules.len()),
        );
        let script = timing_script(&modules);
        let tmp = std::env::temp_dir().join(format!(
            "blender_link_startup_{}.py",
            std::process::id()
        ));
        std::fs::write(&tmp, script).map_err(|e| format!("写入临时脚本失败: {e}"))?;
        let mut cmd = hidden_command(exe);
        cmd.args(["-b", "--factory-startup"]);
        if offline_ok {
            cmd.arg("--offline-mode");
        }
        cmd.arg("--python").arg(&tmp);
        let out = run_with_timeout(cmd, 900, Some(&CANCEL));
        let _ = std::fs::remove_file(&tmp);
        let out = out?;
        let mut parsed = false;
        for line in out.stdout.lines() {
            if let Some(json) = line.trim().strip_prefix(TIMING_MARKER) {
                addons = serde_json::from_str(json)
                    .map_err(|e| format!("解析插件计时结果失败: {e}"))?;
                parsed = true;
                break;
            }
        }
        if !parsed {
            return Err(format!(
                "插件计时未返回结果（退出码 {}）。stderr 末行：{}",
                out.code,
                out.stderr.lines().last().unwrap_or("")
            ));
        }
        addons.sort_by(|a, b| b.seconds.partial_cmp(&a.seconds).unwrap());
    }

    let n_med = median(normal.clone());
    let f_med = median(factory.clone());
    let addon_sum: f64 = addons.iter().map(|a| a.seconds).sum();
    notes.push(format!(
        "插件合计 {:.2} 秒；正常与纯净启动差值 {:.2} 秒（差值还包含用户设置/启动文件的开销）",
        addon_sum,
        (n_med - f_med).max(0.0)
    ));
    if warmup > n_med * 1.8 {
        notes.push(format!(
            "预热运行耗时 {:.2} 秒，明显高于后续测量——说明冷启动（杀毒扫描/磁盘缓存）对启动影响很大",
            warmup
        ));
    }
    emit_progress(app, total, total, "分析完成");

    Ok(StartupResult {
        warmup_seconds: warmup,
        normal_seconds: normal,
        factory_seconds: factory,
        addons,
        notes,
    })
}

/// 启动时间分析：预热 + 正常/纯净对比 + 逐插件计时
#[tauri::command]
pub async fn startup_analyze(
    app: AppHandle,
    exe: String,
    version: String,
    runs: u32,
) -> Result<StartupResult, String> {
    CANCEL.store(false, Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || do_analyze(&app, &exe, &version, runs))
        .await
        .map_err(|e| format!("分析线程异常: {e}"))?
}

#[tauri::command]
pub fn startup_cancel() {
    CANCEL.store(true, Ordering::Relaxed);
}
