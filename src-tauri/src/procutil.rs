use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Windows 下隐藏子进程控制台窗口
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn hidden_command(program: &str) -> Command {
    let cmd = Command::new(program);
    #[cfg(windows)]
    let cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = cmd;
        c.creation_flags(CREATE_NO_WINDOW);
        c
    };
    cmd
}

pub struct RunOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub seconds: f64,
}

/// 同步运行命令，带超时与可选取消标记；后台线程消费管道避免写满死锁
pub fn run_with_timeout(
    mut cmd: Command,
    timeout_secs: u64,
    cancel: Option<&AtomicBool>,
) -> Result<RunOutput, String> {
    use std::io::Read;
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let start = Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("启动进程失败: {e}"))?;
    let mut out_pipe = child.stdout.take().unwrap();
    let mut err_pipe = child.stderr.take().unwrap();
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out_pipe.read_to_string(&mut s);
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err_pipe.read_to_string(&mut s);
        s
    });
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let seconds = start.elapsed().as_secs_f64();
                return Ok(RunOutput {
                    code: status.code().unwrap_or(-1),
                    stdout: t_out.join().unwrap_or_default(),
                    stderr: t_err.join().unwrap_or_default(),
                    seconds,
                });
            }
            Ok(None) => {
                if let Some(c) = cancel {
                    if c.load(Ordering::Relaxed) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err("已取消".into());
                    }
                }
                if start.elapsed() > Duration::from_secs(timeout_secs) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("进程超时（超过 {timeout_secs} 秒），已终止"));
                }
                std::thread::sleep(Duration::from_millis(30));
            }
            Err(e) => return Err(format!("等待进程失败: {e}")),
        }
    }
}
