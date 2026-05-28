//! 板块 ⑦ 环境医生 (Environment Doctor) — 新用户开箱的「环境监测 + 配置安装」
//!
//! 轻量版内核为 pi (`@earendil-works/pi-coding-agent`), 故这里把原本对 Claude Code
//! 的监测/安装换成 pi:
//! - **监测**: pi (`pi` / `pi.cmd`) 与 PowerShell 7 (`pwsh`) 是否就绪;
//!   附带 Node.js / npm 探测 —— pi 是 npm 包, 需 Node ≥ 22.19。
//! - **安装**: pi 没装时一键 `npm install -g @earendil-works/pi-coding-agent`
//!   (产出 `pi.cmd` 垫片于 npm 全局 bin)。npm 方式需要 Node.js —— 缺失时用 winget 装 Node;
//!   PowerShell 7 缺失时: winget 优先, 失败则下载官方 MSI (国内 GitHub 代理加速) 静默安装。
//! - **改环境变量 (关键)**: npm 全局 bin (`%APPDATA%\npm` 或用户自定义前缀) 多数已在 PATH,
//!   但若不在, 则装了也找不到。这里**双写**: ① 持久化进「用户 PATH」(注册表,
//!   `[Environment]::SetEnvironmentVariable`, 会广播 WM_SETTINGCHANGE); ② 立刻塞进
//!   当前进程 PATH (`std::env::set_var`), 让本次会话不重启即可 spawn pi。
//! - **更新**: 装好后可一键检测/更新 pi 到最新版 (`npm view` / `npm i -g ...@latest`)。
//!
//! 跨平台: 本模块以 Windows 为主场。非 Windows 下探测仍可用 (走 which/直接执行),
//! 安装与 PATH 写入是 Windows 专属逻辑, 其余平台返回友好提示, 不阻断编译。

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 给从 GUI 进程拉起的子进程加 `CREATE_NO_WINDOW`, 免得每次探测都闪一个黑色控制台窗口。
#[cfg_attr(not(windows), allow(unused_variables))]
fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

// ───────────────────────── 视图模型 ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    /// 稳定标识: pi | pwsh | node | npm
    pub key: String,
    /// 展示名
    pub name: String,
    /// 是否在机器上找到 (PATH 命中或已知安装位置存在)
    pub found: bool,
    /// 版本号 (探测到才有)
    pub version: Option<String>,
    /// 解析到的可执行文件路径 (正斜杠)
    pub path: Option<String>,
    /// 是否能通过 PATH 直接发现 (即终端里敲命令能用)
    pub on_path: bool,
    /// 是否是「必须」(pi 必须; 其余推荐)
    pub required: bool,
    /// 一句话状态说明 / 安装建议
    pub hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvReport {
    /// "windows" | "macos" | "linux" ...
    pub os: String,
    pub pi: ToolStatus,
    pub pwsh: ToolStatus,
    pub node: ToolStatus,
    pub npm: ToolStatus,
    /// pi 垫片应在 / 已在的目录 (用于「修复 PATH」)
    pub pi_dir: Option<String>,
    /// 该目录是否已在「用户 PATH」里 (Windows)。false ⇒ 需要修复
    pub pi_dir_on_user_path: bool,
    /// 整体是否就绪 (pi 可用即视为可以进入)
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathFixResult {
    pub ok: bool,
    /// 实际加入 PATH 的目录
    pub dir: Option<String>,
    /// "added" | "present" | "process_only" | "skipped"
    pub status: String,
    pub message: String,
}

// ───────────────────────── 流式事件 ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvStreamEvent {
    pub req_id: String,
    /// "log" | "error" | "done"
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    /// done 时: 是否成功
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    /// done 时: 收尾说明 (含 PATH 配置结果)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

static CHILDREN: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, Child>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
static REQ_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_req_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let c = REQ_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("env-{:x}-{:x}", ts, c)
}

// ───────────────────────── 探测原语 ─────────────────────────

fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|u| u.home_dir().to_path_buf())
}

fn to_fwd(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// 用 `where.exe`(Windows) / `which`(unix) 找出某命令的全部命中路径 (存在的才留)。
fn which_all(bin: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("where.exe");
        c.arg(bin);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("which");
        c.args(["-a", bin]);
        c
    };
    cmd.stdin(Stdio::null());
    no_window(&mut cmd);
    let out = match cmd.output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}

/// 取某命令的版本号。Windows 走 `cmd /c <bin> <args>` 以便正确解析 .exe/.cmd (PATHEXT);
/// 其余平台直接执行。返回首个非空行 (去掉前后空白)。
fn probe_version(bin: &str, args: &[&str]) -> Option<String> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        let mut full = vec!["/c".to_string(), bin.to_string()];
        full.extend(args.iter().map(|s| s.to_string()));
        c.args(full);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new(bin);
        c.args(args);
        c
    };
    cmd.stdin(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    let pick = |bytes: &[u8]| -> Option<String> {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .map(|s| s.to_string())
    };
    if out.status.success() {
        // 优先 stdout, 个别工具把版本写到 stderr
        pick(&out.stdout).or_else(|| pick(&out.stderr))
    } else {
        None
    }
}

/// npm 全局安装前缀。走 `npm prefix -g` —— **用户可能改过前缀**(实测有人放在 `D:\Users\x\npm`,
/// 而非默认 `%APPDATA%\npm`), 硬编码默认值会漏掉。失败 / 目录不存在 → None。
fn npm_global_prefix() -> Option<PathBuf> {
    #[cfg(windows)]
    let mut cmd = {
        // 经 cmd /c 以便解析 npm.cmd (CreateProcessW 不认 .cmd)
        let mut c = Command::new("cmd");
        c.args(["/c", "npm", "prefix", "-g"]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("npm");
        c.args(["prefix", "-g"]);
        c
    };
    cmd.stdin(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())?
        .to_string();
    let p = PathBuf::from(line);
    p.exists().then_some(p)
}

/// 已知的 pi 可执行/垫片候选位置。npm 装出的是 `pi.cmd` 垫片 (chat.rs 经 `cmd /c pi` 调起),
/// 这里把 npm 全局前缀 (含用户自定义前缀) 与 bun / 本地 bin 都纳入候选, 仅作探测 / PATH 兜底。
fn pi_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    // npm 全局 (用户真实前缀): pi.cmd 垫片优先
    if let Some(prefix) = npm_global_prefix() {
        v.push(prefix.join("pi.cmd"));
        v.push(prefix.join("pi.exe"));
        v.push(prefix.join("pi"));
    }
    if let Some(h) = home_dir() {
        // 默认 npm 前缀兜底 (拿不到 `npm prefix -g` 时, 例如 npm 不在 PATH)
        let appdata_npm = h.join("AppData").join("Roaming").join("npm");
        v.push(appdata_npm.join("pi.cmd"));
        v.push(appdata_npm.join("pi.exe"));
        v.push(appdata_npm.join("pi"));
        // bun 全局
        v.push(h.join(".bun").join("bin").join("pi.exe"));
        v.push(h.join(".bun").join("bin").join("pi"));
        // 通用本地 bin
        v.push(h.join(".local").join("bin").join("pi"));
        v.push(h.join(".local").join("bin").join("pi.exe"));
    }
    v
}

fn pwsh_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from(r"C:\Program Files\PowerShell\7\pwsh.exe"),
        PathBuf::from(r"C:\Program Files\PowerShell\7-preview\pwsh.exe"),
    ]
}

/// 通用工具探测: which 命中 + 已知候选, 取首个可用; on_path = 是否被 PATH 发现。
fn detect(
    key: &str,
    name: &str,
    bin: &str,
    version_args: &[&str],
    candidates: &[PathBuf],
    required: bool,
    install_hint: &str,
) -> ToolStatus {
    let on_path_hits = which_all(bin);
    let on_path = !on_path_hits.is_empty();

    // 解析出一个具体路径: PATH 命中优先 (Windows 偏好 .exe), 否则用存在的候选
    let resolved: Option<PathBuf> = {
        // 偏好 .exe 命中 (若有原生 exe)
        let exe_hit = on_path_hits
            .iter()
            .find(|p| {
                p.extension()
                    .map(|e| e.eq_ignore_ascii_case("exe"))
                    .unwrap_or(false)
            })
            .cloned();
        exe_hit
            .or_else(|| on_path_hits.first().cloned())
            .or_else(|| candidates.iter().find(|p| p.exists()).cloned())
    };

    let found = resolved.is_some();
    let version = if found {
        probe_version(bin, version_args)
    } else {
        None
    };

    let hint = if found {
        match &version {
            Some(v) => v.clone(),
            None => "已安装".to_string(),
        }
    } else {
        install_hint.to_string()
    };

    ToolStatus {
        key: key.to_string(),
        name: name.to_string(),
        found,
        version,
        path: resolved.as_deref().map(to_fwd),
        on_path,
        required,
        hint,
    }
}

// ───────────────────────── 用户 PATH (Windows) ─────────────────────────

/// 读「用户级 PATH」(注册表 HKCU\Environment), 经 PowerShell .NET API 拿。
#[cfg(windows)]
fn read_user_path() -> Option<String> {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "[Environment]::GetEnvironmentVariable('Path','User')",
    ]);
    cmd.stdin(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(not(windows))]
fn read_user_path() -> Option<String> {
    None
}

/// dir 是否(忽略大小写/尾斜杠)出现在分号分隔的 PATH 串里。
fn path_contains_dir(path_str: &str, dir: &str) -> bool {
    let norm = |s: &str| s.trim().trim_end_matches(['\\', '/']).to_lowercase();
    let target = norm(dir);
    if target.is_empty() {
        return false;
    }
    path_str.split(';').any(|p| norm(p) == target)
}

/// 把 dir 追加进「用户 PATH」(持久化, 注册表) + 当前进程 PATH (立即生效)。
/// Windows 专属; 其余平台仅尝试改进程 PATH。
fn ensure_dir_on_path(dir: &str) -> PathFixResult {
    let dir = dir.trim();
    if dir.is_empty() || !PathBuf::from(dir).exists() {
        return PathFixResult {
            ok: false,
            dir: Some(dir.to_string()),
            status: "skipped".into(),
            message: "目标目录不存在, 无法加入 PATH (请先安装)。".into(),
        };
    }

    // ① 当前进程 PATH (prepend → 本次会话立即能 spawn pi, 无需重启 app)
    let proc_path = std::env::var("PATH").unwrap_or_default();
    if !path_contains_dir(&proc_path, dir) {
        let sep = if cfg!(windows) { ';' } else { ':' };
        let new = if proc_path.is_empty() {
            dir.to_string()
        } else {
            format!("{dir}{sep}{proc_path}")
        };
        std::env::set_var("PATH", new);
    }

    // ② 用户级持久化 PATH (Windows)。用显式 return 收尾, 避免 cfg 块尾表达式歧义。
    #[cfg(windows)]
    {
        if let Some(user_path) = read_user_path() {
            if path_contains_dir(&user_path, dir) {
                return PathFixResult {
                    ok: true,
                    dir: Some(dir.to_string()),
                    status: "present".into(),
                    message: format!("{dir} 已在用户 PATH 中 (进程 PATH 也已同步)。"),
                };
            }
        }
        return match append_user_path(dir) {
            Ok(_) => PathFixResult {
                ok: true,
                dir: Some(dir.to_string()),
                status: "added".into(),
                message: format!(
                    "已把 {dir} 加入用户 PATH 并同步到当前进程。新开的终端 / 重启后均生效。"
                ),
            },
            Err(e) => PathFixResult {
                ok: false,
                dir: Some(dir.to_string()),
                status: "process_only".into(),
                message: format!(
                    "已加入当前进程 PATH, 但持久化到用户 PATH 失败: {e}。可手动把 {dir} 加到 PATH。"
                ),
            },
        };
    }
    #[cfg(not(windows))]
    {
        return PathFixResult {
            ok: true,
            dir: Some(dir.to_string()),
            status: "process_only".into(),
            message: format!("已加入当前进程 PATH。请把 {dir} 写进你的 shell 配置以持久化。"),
        };
    }
}

/// 通过 PowerShell .NET API 把 dir 追加进用户 PATH (会广播 WM_SETTINGCHANGE)。
#[cfg(windows)]
fn append_user_path(dir: &str) -> Result<(), String> {
    // 单引号转义: PowerShell 里单引号字符串内的 ' 写成 ''
    let safe = dir.replace('\'', "''");
    let script = format!(
        "$d = '{safe}'; \
$u = [Environment]::GetEnvironmentVariable('Path','User'); \
if ($null -eq $u) {{ $u = '' }}; \
$parts = $u.Split(';') | Where-Object {{ $_ -ne '' }}; \
if ($parts -notcontains $d) {{ \
  $base = $u.TrimEnd(';'); \
  if ($base -eq '') {{ $new = $d }} else {{ $new = $base + ';' + $d }}; \
  [Environment]::SetEnvironmentVariable('Path', $new, 'User'); \
  Write-Output 'ADDED' \
}} else {{ Write-Output 'PRESENT' }}"
    );
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    cmd.stdin(Stdio::null());
    no_window(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("调用 PowerShell 写 PATH 失败: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// pi 垫片应该落脚的目录 (用于「修复 PATH」): 已解析路径的父目录优先,
/// 否则 npm 全局前缀 (pi.cmd 垫片所在), 再否则默认 `%APPDATA%\npm`。
fn pi_dir_for_fix(pi: &ToolStatus) -> Option<PathBuf> {
    if let Some(p) = &pi.path {
        return PathBuf::from(p.replace('/', std::path::MAIN_SEPARATOR_STR))
            .parent()
            .map(|p| p.to_path_buf());
    }
    npm_global_prefix().or_else(|| home_dir().map(|h| h.join("AppData").join("Roaming").join("npm")))
}

// ───────────────────────── Commands ─────────────────────────

#[tauri::command]
pub fn env_check() -> EnvReport {
    let os = std::env::consts::OS.to_string();

    let pi = detect(
        "pi",
        "pi (内核)",
        "pi",
        &["--version"],
        &pi_candidates(),
        true,
        "未安装 —— 可一键安装 (npm)",
    );
    let pwsh = detect(
        "pwsh",
        "PowerShell 7",
        "pwsh",
        &["--version"],
        &pwsh_candidates(),
        false,
        "未安装 —— 建议安装 (winget)",
    );
    let node = detect(
        "node",
        "Node.js",
        "node",
        &["--version"],
        &[],
        false,
        "未安装 (pi 需 Node ≥ 22.19)",
    );
    let npm = detect(
        "npm",
        "npm",
        "npm",
        &["--version"],
        &[],
        false,
        "未安装 (安装 pi 需要它)",
    );

    // PATH 体检: pi 垫片目录是否在用户 PATH 里
    let pi_dir = pi_dir_for_fix(&pi);
    let pi_dir_on_user_path = match (&pi_dir, read_user_path()) {
        (Some(d), Some(up)) => path_contains_dir(&up, &d.to_string_lossy()),
        // 没装 / 拿不到用户 PATH → 当作「无需提示修复」(待安装后再判)
        _ => true,
    };

    let ready = pi.found;

    EnvReport {
        os,
        pi,
        pwsh,
        node,
        npm,
        pi_dir: pi_dir.as_deref().map(to_fwd),
        pi_dir_on_user_path,
        ready,
    }
}

/// 修复 PATH: 把 pi 垫片所在目录写进用户 PATH + 当前进程 PATH。
#[tauri::command]
pub fn env_fix_path() -> Result<PathFixResult, String> {
    let report = env_check();
    match report.pi_dir {
        Some(d) => Ok(ensure_dir_on_path(&d)),
        None => Ok(PathFixResult {
            ok: false,
            dir: None,
            status: "skipped".into(),
            message: "尚未找到 pi 安装目录, 请先安装。".into(),
        }),
    }
}

/// 安装 pi 内核 (`npm install -g @earendil-works/pi-coding-agent`)。
/// method 兼容旧前端 ("native"/"npm" 均走 npm)。流式把安装日志通过 `env:stream`
/// 事件推给前端; 成功后自动修 PATH。
#[tauri::command]
pub fn env_install_pi(app: AppHandle, method: Option<String>) -> Result<String, String> {
    if !cfg!(windows) {
        return Err("自动安装目前仅支持 Windows; 其他平台请用 `npm i -g @earendil-works/pi-coding-agent` 手动安装。".into());
    }
    let _ = method; // pi 只有 npm 一条安装路径
    let inner = "npm install -g @earendil-works/pi-coding-agent".to_string();
    let req_id = next_req_id();
    let cmd = build_powershell(&inner);
    stream_install(app, req_id.clone(), cmd, true, "pi");
    Ok(req_id)
}

/// 安装 Node.js LTS (winget) —— pi (npm 包) 的前置依赖。
/// winget 安装会自带配 PATH, 故无需我们再改 (`fix_path_after=false`)。
#[tauri::command]
pub fn env_install_node(app: AppHandle) -> Result<String, String> {
    if !cfg!(windows) {
        return Err("Node.js 自动安装仅支持 Windows; 其他平台请用系统包管理器手动安装。".into());
    }
    let inner = "winget install --id OpenJS.NodeJS.LTS -e --source winget \
--accept-package-agreements --accept-source-agreements"
        .to_string();
    let req_id = next_req_id();
    let cmd = build_powershell(&inner);
    stream_install(app, req_id.clone(), cmd, false, "Node.js");
    Ok(req_id)
}

/// 安装 PowerShell 7。成功无需改 PATH (MSI / winget 安装都会自带配 PATH)。
///
/// 之前只用 `winget`, 但很多机器上要么没有 winget、要么 winget 源在国内拉不动
/// → 用户报「PowerShell 7 下载不了」。这里改成**两层策略**:
/// ① 有 winget 先用 winget (官方、能拿最新版);
/// ② winget 缺失 / 失败 → **直接下载官方 MSI 再 msiexec 静默安装**, 且下载走
///    国内可达的 GitHub 文件代理 (gh-proxy / ghfast) 兜底, 实在不行再走 GitHub 直连。
///    这就是「下载路径」修复 —— 明确把 MSI 落到 `%TEMP%` 再装, 不再黑盒依赖 winget。
#[tauri::command]
pub fn env_install_pwsh(app: AppHandle) -> Result<String, String> {
    if !cfg!(windows) {
        return Err("PowerShell 7 自动安装仅支持 Windows。".into());
    }
    let req_id = next_req_id();
    let cmd = build_powershell(PWSH_INSTALL_SCRIPT);
    stream_install(app, req_id.clone(), cmd, false, "PowerShell 7");
    Ok(req_id)
}

/// PowerShell 7 安装脚本: winget 优先, 失败则下载官方 MSI (国内代理加速) 静默安装。
/// 版本仅用于 MSI 兜底直链 (winget 路径自动取最新); 选 7.4.x LTS, 稳定且长期可用。
const PWSH_INSTALL_SCRIPT: &str = r#"
$ErrorActionPreference = 'Continue'
# ① 优先 winget (能拿最新版, 自带配 PATH)
$wg = Get-Command winget -ErrorAction SilentlyContinue
if ($wg) {
  Write-Output '检测到 winget, 优先用它安装 PowerShell 7...'
  & winget install --id Microsoft.PowerShell -e --source winget --accept-package-agreements --accept-source-agreements
  if ($LASTEXITCODE -eq 0) { Write-Output 'PowerShell 7 (winget) 安装完成。'; exit 0 }
  Write-Output ('winget 安装未成功 (退出码 ' + $LASTEXITCODE + '), 改用直接下载 MSI...')
} else {
  Write-Output '未检测到 winget, 改用直接下载官方 MSI...'
}
# ② 下载官方 MSI -> %TEMP% -> msiexec 静默安装。下载路径走国内可达的 GitHub 代理兜底。
$ver = '7.4.6'
$arch = switch ($env:PROCESSOR_ARCHITECTURE) { 'ARM64' { 'arm64' } 'AMD64' { 'x64' } default { 'x86' } }
$msi = "PowerShell-$ver-win-$arch.msi"
$dst = Join-Path $env:TEMP $msi
$rel = "https://github.com/PowerShell/PowerShell/releases/download/v$ver/$msi"
$urls = @(
  "https://gh-proxy.com/$rel",
  "https://ghfast.top/$rel",
  "https://ghproxy.net/$rel",
  $rel
)
$ok = $false
foreach ($u in $urls) {
  try {
    Write-Output "下载: $u"
    Invoke-WebRequest -Uri $u -OutFile $dst -UseBasicParsing -TimeoutSec 600
    if ((Test-Path $dst) -and ((Get-Item $dst).Length -gt 1MB)) { $ok = $true; break }
  } catch {
    Write-Output ("  下载失败: " + $_.Exception.Message)
  }
}
if (-not $ok) {
  Write-Output 'PowerShell 7 安装包下载失败 (可检查网络 / 代理后重试)。'
  exit 1
}
# 安装到 Program Files 需要管理员权限 -> 用 RunAs 触发 UAC (拒绝则友好报错, 不静默失败)
Write-Output "安装中 (msiexec, 会弹一次 UAC 授权): $dst"
try {
  $p = Start-Process msiexec.exe -ArgumentList ('/i "' + $dst + '" /quiet /norestart ADD_PATH=1') -Wait -PassThru -Verb RunAs
} catch {
  Write-Output ('安装启动失败 (可能未授予管理员权限): ' + $_.Exception.Message)
  exit 1
}
Remove-Item $dst -ErrorAction SilentlyContinue
if ($p.ExitCode -ne 0) { Write-Output ('msiexec 退出码 ' + $p.ExitCode); exit 1 }
Write-Output 'PowerShell 7 安装完成。'
"#;

// ───────────────────────── pi 更新 ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiUpdateInfo {
    /// 是否已安装 (装了才谈更新)
    pub installed: bool,
    /// 当前版本 (纯 x.y.z, 解析不出则原样)
    pub current: Option<String>,
    /// npm 上的最新版本
    pub latest: Option<String>,
    /// 是否有可用更新 (latest > current)
    pub update_available: bool,
    /// 是否成功查到了 latest (网络/registry 可用)
    pub checked: bool,
    /// 一句话说明
    pub message: String,
}

/// 把 "0.3.1 (pi)" 这类串里第一个形如 a.b.c 的版本号解析成元组。
fn parse_triplet(tok: &str) -> Option<(u64, u64, u64)> {
    let mut it = tok.split('.');
    let a = it.next()?.parse::<u64>().ok()?;
    let b = it.next()?.parse::<u64>().ok()?;
    let c = it.next()?.parse::<u64>().ok()?;
    Some((a, b, c))
}

fn extract_semver(s: &str) -> Option<(u64, u64, u64)> {
    for tok in s.split(|c: char| !(c.is_ascii_digit() || c == '.')) {
        if tok.is_empty() {
            continue;
        }
        if let Some(t) = parse_triplet(tok) {
            return Some(t);
        }
    }
    None
}

const PI_PACKAGE: &str = "@earendil-works/pi-coding-agent";

/// npm registry 上 pi 的最新版本号 (`npm view ... version`)。
fn npm_view_latest() -> Option<String> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/c", "npm", "view", PI_PACKAGE, "version"]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("npm");
        c.args(["view", PI_PACKAGE, "version"]);
        c
    };
    cmd.stdin(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .map(|s| s.to_string())
}

/// 没有 npm 时的兜底: 直接打 npm registry HTTP 接口取 dist-tags.latest。
#[cfg(windows)]
fn registry_latest_via_http() -> Option<String> {
    let script = "(Invoke-RestMethod -UseBasicParsing \
'https://registry.npmjs.org/@earendil-works/pi-coding-agent').'dist-tags'.latest";
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    cmd.stdin(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!v.is_empty()).then_some(v)
}

#[cfg(not(windows))]
fn registry_latest_via_http() -> Option<String> {
    None
}

/// 检测 pi 是否有新版本: 当前版本 (`pi --version`) vs registry latest。
#[tauri::command]
pub fn env_pi_update_check() -> PiUpdateInfo {
    let current_raw = probe_version("pi", &["--version"]);
    let installed = current_raw.is_some() || pi_candidates().iter().any(|p| p.exists());
    if !installed {
        return PiUpdateInfo {
            installed: false,
            current: None,
            latest: None,
            update_available: false,
            checked: false,
            message: "未检测到 pi, 请先安装。".into(),
        };
    }

    // 当前版本: 优先展示解析出的纯 semver, 否则原样
    let cur_semver = current_raw.as_deref().and_then(extract_semver);
    let current = cur_semver
        .map(|(a, b, c)| format!("{a}.{b}.{c}"))
        .or_else(|| current_raw.clone());

    let latest = npm_view_latest().or_else(registry_latest_via_http);
    match latest {
        Some(l) => {
            let lv = extract_semver(&l);
            let update_available = match (cur_semver, lv) {
                (Some(c), Some(n)) => n > c,
                _ => false,
            };
            let message = if update_available {
                format!("发现新版本 {l} (当前 {})。", current.clone().unwrap_or_default())
            } else {
                format!("已是最新版本 ({})。", current.clone().unwrap_or_default())
            };
            PiUpdateInfo {
                installed: true,
                current,
                latest: Some(l),
                update_available,
                checked: true,
                message,
            }
        }
        None => PiUpdateInfo {
            installed: true,
            current,
            latest: None,
            update_available: false,
            checked: false,
            message: "无法获取最新版本号 (可检查网络 / npm 后重试)。".into(),
        },
    }
}

/// 更新 pi 到最新版 (`npm i -g @earendil-works/pi-coding-agent@latest`)。
/// 复用流式安装管线; 成功后自动修 PATH (与首次安装一致)。
#[tauri::command]
pub fn env_update_pi(app: AppHandle) -> Result<String, String> {
    if !cfg!(windows) {
        return Err("自动更新目前仅支持 Windows; 其他平台请用 npm 手动更新。".into());
    }
    let inner = "npm install -g @earendil-works/pi-coding-agent@latest";
    let req_id = next_req_id();
    let cmd = build_powershell(inner);
    stream_install(app, req_id.clone(), cmd, true, "pi 更新");
    Ok(req_id)
}

#[tauri::command]
pub fn env_cancel(req_id: String) -> Result<(), String> {
    if let Some(mut child) = CHILDREN.lock().remove(&req_id) {
        let _ = child.kill();
    }
    Ok(())
}

// ───────────────────────── 内部: 流式安装 ─────────────────────────

/// 构造一个跑给定内联命令的 PowerShell 进程 (Bypass 执行策略, 以便 iex 远程脚本)。
fn build_powershell(inner: &str) -> Command {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        inner,
    ]);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_window(&mut cmd);
    cmd
}

fn emit(app: &AppHandle, ev: EnvStreamEvent) {
    let _ = app.emit("env:stream", ev);
}

/// 起子进程, 双线程读 stdout/stderr → `env:stream` 日志; 退出后(可选)修 PATH, 再发 done。
fn stream_install(app: AppHandle, req_id: String, mut cmd: Command, fix_path_after: bool, label: &str) {
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            emit(
                &app,
                EnvStreamEvent {
                    req_id,
                    kind: "done".into(),
                    line: None,
                    ok: Some(false),
                    message: Some(format!("启动安装进程失败: {e} (PowerShell 是否可用?)")),
                },
            );
            return;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    CHILDREN.lock().insert(req_id.clone(), child);

    // stderr 线程
    if let Some(stderr) = stderr {
        let app_e = app.clone();
        let req_e = req_id.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let Ok(line) = line else { continue };
                if line.trim().is_empty() {
                    continue;
                }
                emit(
                    &app_e,
                    EnvStreamEvent {
                        req_id: req_e.clone(),
                        kind: "log".into(),
                        line: Some(line),
                        ok: None,
                        message: None,
                    },
                );
            }
        });
    }

    // stdout 线程 (主): 读完 → wait → 修 PATH → done
    let label = label.to_string();
    std::thread::spawn(move || {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { continue };
                if line.trim().is_empty() {
                    continue;
                }
                emit(
                    &app,
                    EnvStreamEvent {
                        req_id: req_id.clone(),
                        kind: "log".into(),
                        line: Some(line),
                        ok: None,
                        message: None,
                    },
                );
            }
        }

        let child_opt = CHILDREN.lock().remove(&req_id);
        let success = if let Some(mut child) = child_opt {
            child.wait().map(|s| s.success()).unwrap_or(false)
        } else {
            // 被 cancel 掉了
            emit(
                &app,
                EnvStreamEvent {
                    req_id: req_id.clone(),
                    kind: "done".into(),
                    line: None,
                    ok: Some(false),
                    message: Some("安装已取消。".into()),
                },
            );
            return;
        };

        let mut message = if success {
            format!("{label} 安装完成。")
        } else {
            format!("{label} 安装未成功 (进程非零退出)，可查看上方日志或改用其他方式重试。")
        };

        // 成功后自动修 PATH (改环境变量) —— 这是「装完即可用」的关键
        if success && fix_path_after {
            let report = env_check();
            if let Some(dir) = report.pi_dir {
                let fix = ensure_dir_on_path(&dir);
                message.push('\n');
                message.push_str(&fix.message);
            }
        }

        emit(
            &app,
            EnvStreamEvent {
                req_id: req_id.clone(),
                kind: "done".into(),
                line: None,
                ok: Some(success),
                message: Some(message),
            },
        );
    });
}
