//! # 板块 ⑤ 安全沙箱层 (polaris-sandbox)
//!
//! 轻量 Docker 镜像 + docker CLI 包装。设计依据 PRD-v6 §11。
//!
//! ## 板块边界 (架构重构 Phase 1)
//! 本板块已抽离为独立 crate。它**不再** `use crate::kb` —— 挂载 KB 时所需的
//! 根路径改由 host 通过 [`polaris_core::KbLocator`] 注入 (依赖反转)，故本 crate
//! 只依赖 `polaris-core` 契约，可独立 `cargo test -p polaris-sandbox`。
//!
//! ## 为什么命令放在 `commands` 子模块
//! `#[tauri::command]` 会为每个命令生成一个 `__cmd__*` 隐藏宏并 `pub(crate) use`
//! 它；当命令直接定义在 **crate 根** (lib.rs 顶层) 时，该再导入会与宏定义本身
//! 撞名 (E0255)。放进子模块即可规避，与命令在普通 `mod` 里的行为一致。
//! host 端用 `polaris_sandbox::commands::<cmd>` 注册命令 (宏需按定义模块路径解析)。

pub mod commands;
/// CubeSandbox (E2B 兼容) 后端 —— 「替换 Docker」的可选后端 (additive)。
pub mod e2b;

// 把「可直接调用的函数」与常量再导出到 crate 根：
// 供板块① `chat` 以 `polaris_sandbox::sandbox_status()` / `::CONTAINER_NAME` 调用，
// 以及 host 在 setup 阶段调 `polaris_sandbox::init()`。
pub use commands::{init, sandbox_status, CONTAINER_NAME, IMAGE_NAME};
pub use e2b::{load_config as cube_config, CubeConfig};
