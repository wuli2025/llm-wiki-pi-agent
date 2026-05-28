//! # polaris-core —— 板块契约层 (Contract Layer)
//!
//! 这是整套板块化架构的关键 crate：**唯一允许被所有板块依赖、且自身不
//! 依赖任何板块**的 crate。在此定义跨板块流转的 DTO、能力 trait、事件契约。
//!
//! ## 依赖方向铁律 (由编译器强制)
//!
//! 所有箭头单向指向 core：`chat → core ← kb`、`chat → core ← sandbox` ……
//! **板块之间永远没有直接箭头**。一旦某个板块 crate 写了
//! `use polaris_xxx::…`（host 除外），Cargo 直接拒绝编译。
//!
//! 对应 PRD-v6 §16「板块边界铁律」第 1 条。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ───────────────────────── 共享 DTO ─────────────────────────

/// 沙箱状态。
///
/// 由板块⑤ `polaris-sandbox` 产出，板块① `chat` 在发送前预检容器时读取。
/// 放在 core 是为了让两个板块都能引用同一份类型，而不必互相 import。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatus {
    pub docker_installed: bool,
    pub docker_running: bool,
    pub image_built: bool,
    pub image_name: String,
    pub container_running: bool,
    pub container_name: String,
    pub notes: Vec<String>,
}

// ───────────────────────── 能力 trait ───────────────────────

/// 知识库根目录定位器。
///
/// 用于打破 `sandbox → kb` 的硬依赖：沙箱在 `sandbox_start` 挂载 KB 时
/// 需要知道 KB 根路径，但**不应** import 板块② 的内部实现。
/// 改由 host 在启动时 `manage` 一个实现，沙箱通过 Tauri 托管状态取用
/// —— 这就是「依赖反转」。
///
/// 对应 PRD-v6 §16 第 1 条（跨板块只能调公开契约）。
pub trait KbLocator: Send + Sync + 'static {
    /// 返回当前 KB 根目录的绝对路径。
    fn kb_root(&self) -> PathBuf;
}
