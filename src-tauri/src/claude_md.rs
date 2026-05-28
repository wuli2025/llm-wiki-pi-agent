//! 板块 ⑥ CLAUDE.md 主上下文管理 (重写版)
//!
//! 新方案:
//! - 每个 conv 项目一份: ~/Polaris/projects/<project-id>/CLAUDE.md
//! - 知识库共享一份: ~/Polaris/PolarisKB/CLAUDE.md (随 KB root 走)
//! - 发对话时, 只注入「当前会话所在项目的 CLAUDE.md」+「KB CLAUDE.md」
//! - 不再扫描代码仓库子目录
//!
//! placeholder marker: 顶部含 `polaris:placeholder` 行表示「未填写」, 不注入

use crate::conv;
use crate::kb;
use anyhow::Result;
use directories::UserDirs;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

pub const PLACEHOLDER_MARKER: &str = "polaris:placeholder";

const TEMPLATE: &str = include_str!("templates/project_claude.md");

pub fn init(_app: &AppHandle) -> Result<()> {
    Ok(())
}

// ───────────────────────── 路径定位 ─────────────────────────

/// polaris-app 仓库根 (src-tauri/ 的父级,编译期固定)
/// chat::spawn_on_host 用这个做 claude CLI 的 cwd,
/// 让 claude 自动信任整棵 polaris-app/ 子树
pub fn project_root() -> Option<PathBuf> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .filter(|p| p.exists())
}

fn projects_root() -> Option<PathBuf> {
    UserDirs::new().map(|u| u.home_dir().join("Polaris").join("projects"))
}

fn project_claude_md_path(project_id: &str) -> Option<PathBuf> {
    projects_root().map(|r| r.join(project_id).join("CLAUDE.md"))
}

fn kb_claude_md_path() -> Option<PathBuf> {
    let kb_root = PathBuf::from(kb::kb_root());
    if kb_root.as_os_str().is_empty() {
        None
    } else {
        Some(kb_root.join("CLAUDE.md"))
    }
}

fn classify(path: &std::path::Path) -> (bool, bool, u64) {
    if !path.exists() {
        return (false, false, 0);
    }
    let content = fs::read_to_string(path).unwrap_or_default();
    let active = !content.contains(PLACEHOLDER_MARKER) && !content.trim().is_empty();
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    (true, active, size)
}

// ───────────────────────── List ─────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectClaudeMd {
    pub project_id: String,
    pub project_name: String,
    pub abs_path: String,
    pub exists: bool,
    pub active: bool,
    pub size: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbClaudeMd {
    pub abs_path: String,
    pub exists: bool,
    pub active: bool,
    pub size: u64,
}

#[tauri::command]
pub fn claude_md_list_projects() -> Vec<ProjectClaudeMd> {
    conv::list_active_projects()
        .into_iter()
        .filter_map(|p| {
            let path = project_claude_md_path(&p.id)?;
            let (exists, active, size) = classify(&path);
            Some(ProjectClaudeMd {
                project_id: p.id,
                project_name: p.name,
                abs_path: path.to_string_lossy().replace('\\', "/"),
                exists,
                active,
                size,
            })
        })
        .collect()
}

#[tauri::command]
pub fn claude_md_kb_info() -> KbClaudeMd {
    let path = match kb_claude_md_path() {
        Some(p) => p,
        None => {
            return KbClaudeMd {
                abs_path: String::new(),
                exists: false,
                active: false,
                size: 0,
            }
        }
    };
    let (exists, active, size) = classify(&path);
    KbClaudeMd {
        abs_path: path.to_string_lossy().replace('\\', "/"),
        exists,
        active,
        size,
    }
}

// ───────────────────────── Read / Write ─────────────────────────

fn resolve_path(area: &str, project_id: Option<&str>) -> Result<PathBuf, String> {
    match area {
        "kb" => kb_claude_md_path().ok_or_else(|| "KB 根目录未就绪".into()),
        "project" => {
            let pid = project_id
                .ok_or_else(|| "area=project 时必须给 projectId".to_string())?;
            if !conv::list_active_projects().iter().any(|p| p.id == pid) {
                return Err(format!("未知项目 id: {}", pid));
            }
            project_claude_md_path(pid).ok_or_else(|| "无法确定项目路径".into())
        }
        _ => Err(format!("未知 area: {}", area)),
    }
}

#[tauri::command]
pub fn claude_md_read(area: String, project_id: Option<String>) -> Result<String, String> {
    let path = resolve_path(&area, project_id.as_deref())?;
    if !path.exists() {
        // 文件还没创建过,返回模板供用户编辑
        return Ok(TEMPLATE.to_string());
    }
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn claude_md_write(
    area: String,
    project_id: Option<String>,
    content: String,
) -> Result<(), String> {
    let path = resolve_path(&area, project_id.as_deref())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())
}

// ───────────────────────── 给 chat::send 用 ─────────────────────────

/// 主上下文渲染 (一次给 chat::send 全部内容):
/// - 知识库块: KB CLAUDE.md (若激活) + 基于 user_prompt 的 kb_search 自动召回 top-3 全文
/// - 项目块:  当前项目 CLAUDE.md (若激活)
///
/// 设计: 把 KB 和它的 CLAUDE.md 当成「一体」, 不再让 LLM 自己去调 kb_search,
/// 而是后端在发对话前就把召回结果嵌进去。
pub fn render_for_project(project_id: Option<&str>, user_prompt: &str) -> String {
    let mut sections: Vec<String> = Vec::new();

    // ① 知识库块
    if let Some(p) = kb_claude_md_path() {
        if let Ok(content) = fs::read_to_string(&p) {
            if !content.contains(PLACEHOLDER_MARKER) && !content.trim().is_empty() {
                let mut block = format!(
                    "### [知识库] `{}`\n\n{}\n\n",
                    p.display(),
                    content.trim()
                );
                // 同一个块里嵌入 kb_search 自动召回
                let q = user_prompt.trim();
                if !q.is_empty() {
                    let hits = kb::kb_search(q.to_string(), Some(3));
                    if !hits.is_empty() {
                        block.push_str("#### 知识库自动召回 (top 3, 已在后端预查, 无需再调任何工具)\n\n");
                        let kb_root = PathBuf::from(kb::kb_root());
                        for (i, h) in hits.iter().enumerate() {
                            let full = kb_root.join(&h.path);
                            let body = fs::read_to_string(&full).unwrap_or_default();
                            let trimmed: String = body.chars().take(3000).collect();
                            block.push_str(&format!(
                                "**[{}] {}** _(score={:.1}, source=`{}`)_\n\n{}\n\n",
                                i + 1,
                                h.title,
                                h.score,
                                h.path,
                                trimmed
                            ));
                        }
                    }
                }
                block.push_str("---\n\n");
                sections.push(block);
            }
        }
    }

    // ② 当前项目 CLAUDE.md 块
    if let Some(pid) = project_id {
        if let Some(p) = project_claude_md_path(pid) {
            if let Ok(content) = fs::read_to_string(&p) {
                if !content.contains(PLACEHOLDER_MARKER) && !content.trim().is_empty() {
                    sections.push(format!(
                        "### [当前项目] `{}`\n\n{}\n\n---\n\n",
                        p.display(),
                        content.trim()
                    ));
                }
            }
        }
    }

    if sections.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n\n## 主上下文 (CLAUDE.md + 知识库 一体注入)\n\n");
    out.push_str(
        "以下内容由 Polaris 在发送前自动准备好, 请优先据此回答, \
         无需再调 kb_search/任何工具来访问知识库:\n\n",
    );
    for s in &sections {
        out.push_str(s);
    }
    out
}
