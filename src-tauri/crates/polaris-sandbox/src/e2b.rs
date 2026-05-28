//! CubeSandbox (E2B 兼容) 后端 —— 板块⑤ 的「替换 Docker」落地点。
//!
//! ## 背景
//! CubeSandbox 是腾讯云基于 **RustVMM + KVM** 的微虚机沙箱，依赖 **Linux + KVM**，
//! **Windows 宿主无法原生运行**；但它**原生兼容 E2B SDK**（「只需替换一个 URL 环境变量」）。
//! 因此 Polaris 的接入方式 = 把 CubeSandbox 当作一个 **E2B 端点**（远程部署 / WSL2 / 云）来连。
//!
//! ## 设计
//! 沿用本 crate「shell out、零运行时依赖」的哲学：HTTP 调 **curl**（Win10+ 内置），
//! 不引入 reqwest/ureq，不改 release panic 配置，不碰现有 docker 命令（纯 additive）。
//! - 配置持久化到 `~/Polaris/sandbox/cube-sandbox.json`
//! - `cube_status` 用 curl 探测端点可达性（连通即「可用」）
//! - 选择 backend=e2b 后，chat 层可据此把执行路由到 CubeSandbox（后续 P3 续接）

use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// 沙箱后端选择 + CubeSandbox(E2B) 连接配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CubeConfig {
    /// "docker"(默认, 现状) | "e2b"(CubeSandbox)
    pub backend: String,
    /// CubeSandbox / E2B 端点 (形如 https://host:port 或 E2B_DOMAIN)
    pub endpoint: String,
    /// 访问密钥 (E2B_API_KEY)，可空
    pub api_key: String,
}

impl Default for CubeConfig {
    fn default() -> Self {
        CubeConfig {
            backend: "docker".into(),
            endpoint: String::new(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CubeStatus {
    pub backend: String,
    pub endpoint: String,
    pub configured: bool,
    /// 端点是否可达 (curl 探测)
    pub reachable: bool,
    pub note: String,
}

fn config_path() -> Option<PathBuf> {
    let dir = UserDirs::new()?.home_dir().join("Polaris").join("sandbox");
    let _ = std::fs::create_dir_all(&dir);
    Some(dir.join("cube-sandbox.json"))
}

pub fn load_config() -> CubeConfig {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<CubeConfig>(&s).ok())
        .unwrap_or_default()
}

fn save(cfg: &CubeConfig) -> Result<(), String> {
    let p = config_path().ok_or("无法定位配置目录")?;
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(p, s).map_err(|e| e.to_string())
}

// ───────────────────────── Tauri 命令 ─────────────────────────

#[tauri::command]
pub fn cube_config_get() -> CubeConfig {
    load_config()
}

#[tauri::command]
pub fn cube_config_set(config: CubeConfig) -> Result<CubeConfig, String> {
    save(&config)?;
    Ok(config)
}

/// 探测 CubeSandbox(E2B) 端点是否可达（curl，超时 6s）。
#[tauri::command]
pub fn cube_status() -> CubeStatus {
    let cfg = load_config();
    let configured = !cfg.endpoint.trim().is_empty();
    if !configured {
        return CubeStatus {
            backend: cfg.backend,
            endpoint: cfg.endpoint,
            configured: false,
            reachable: false,
            note: "未配置 CubeSandbox 端点。CubeSandbox 依赖 Linux+KVM，需部署在 Linux 主机/WSL2/云上，再把端点 URL 填到这里。".into(),
        };
    }

    // 以 curl 探测可达性 (HEAD 失败则尝试 GET)；只看连得通否，不强求 2xx。
    let mut args: Vec<String> = vec![
        "-s".into(),
        "-o".into(),
        nul_path(),
        "-m".into(),
        "6".into(),
        "-w".into(),
        "%{http_code}".into(),
    ];
    if !cfg.api_key.trim().is_empty() {
        args.push("-H".into());
        args.push(format!("X-API-Key: {}", cfg.api_key.trim()));
    }
    args.push(cfg.endpoint.trim().to_string());

    let out = Command::new("curl").args(&args).output();
    let (reachable, note) = match out {
        Ok(o) => {
            let code = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if o.status.success() && code != "000" && !code.is_empty() {
                (true, format!("端点可达 (HTTP {}).", code))
            } else {
                (false, "端点无响应 (curl 未拿到状态码)。检查 URL / 网络 / CubeSandbox 是否在运行。".into())
            }
        }
        Err(e) => (false, format!("curl 调用失败: {}。Windows 10+ 自带 curl。", e)),
    };

    CubeStatus {
        backend: cfg.backend,
        endpoint: cfg.endpoint,
        configured: true,
        reachable,
        note,
    }
}

#[cfg(windows)]
fn nul_path() -> String {
    "NUL".into()
}
#[cfg(not(windows))]
fn nul_path() -> String {
    "/dev/null".into()
}
