//! 板块 ① 对话核心 - 项目 + 对话 + 消息持久化
//!
//! MVP: 单文件 JSON (`~/Polaris/data/state.json`), 全局 RwLock 保护
//! 后续接 ② Wiki 的 storage::* (SQLite), API 不动

use anyhow::Result;
use directories::UserDirs;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String, // user | assistant | tool
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    #[serde(default)]
    projects: Vec<Project>,
    #[serde(default)]
    conversations: Vec<Conversation>,
    #[serde(default)]
    messages: Vec<Message>,
    /// 一次性 marker: 是否已赠送「毛主席」默认项目 + 写入人格 CLAUDE.md。
    /// 置位后即便用户删了该项目也不再重建 —— 尊重用户。
    #[serde(default)]
    seeded_mao: bool,
}

/// 默认赠送的「毛主席」项目名(前端据此识别该项目, 显示彩蛋空状态)
pub const MAO_PROJECT_NAME: &str = "毛主席";
const MAO_PERSONA_TEMPLATE: &str = include_str!("templates/mao_persona_claude.md");

// ───────────────────────── State ─────────────────────────

static STATE: Lazy<RwLock<State>> = Lazy::new(|| RwLock::new(State::default()));
static STATE_PATH: Lazy<RwLock<PathBuf>> = Lazy::new(|| RwLock::new(PathBuf::new()));

// ───────────────────────── Init / persist ────────────────

pub fn init(_app: &AppHandle) -> Result<()> {
    let user = UserDirs::new().ok_or_else(|| anyhow::anyhow!("no user dir"))?;
    let dir = user.home_dir().join("Polaris").join("data");
    fs::create_dir_all(&dir)?;
    let path = dir.join("state.json");
    *STATE_PATH.write() = path.clone();

    let mut state: State = if path.exists() {
        let txt = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&txt).unwrap_or_default()
    } else {
        State::default()
    };

    // 首次启动: 自建一个"默认项目"
    if state.projects.is_empty() {
        let pid = new_id("p");
        let now = now_ms();
        state.projects.push(Project {
            id: pid.clone(),
            name: "默认项目".into(),
            created_at: now,
            archived: false,
        });
    }

    // 首启一次性: 赠送「毛主席」项目 + 写入毛主席人格 CLAUDE.md。
    // 插到最前, 作为默认进入的项目, 让对话框彩蛋空状态可见。
    if !state.seeded_mao {
        match state.projects.iter().find(|p| p.name == MAO_PROJECT_NAME) {
            Some(p) => write_mao_persona(&p.id),
            None => {
                let pid = new_id("p");
                state.projects.insert(
                    0,
                    Project {
                        id: pid.clone(),
                        name: MAO_PROJECT_NAME.into(),
                        created_at: now_ms(),
                        archived: false,
                    },
                );
                write_mao_persona(&pid);
            }
        }
        state.seeded_mao = true;
    }

    *STATE.write() = state;
    persist();
    Ok(())
}

/// 把毛主席人格 CLAUDE.md 写到该项目目录 `~/Polaris/projects/<id>/CLAUDE.md`。
/// 已存在则不覆盖(尊重用户改动)。路径须与 `claude_md` 模块一致。
fn write_mao_persona(project_id: &str) {
    let Some(user) = UserDirs::new() else { return };
    let dir = user
        .home_dir()
        .join("Polaris")
        .join("projects")
        .join(project_id);
    let path = dir.join("CLAUDE.md");
    if path.exists() {
        return;
    }
    if fs::create_dir_all(&dir).is_ok() {
        let _ = fs::write(&path, MAO_PERSONA_TEMPLATE);
    }
}

fn persist() {
    let st = STATE.read();
    let path = STATE_PATH.read().clone();
    if let Ok(txt) = serde_json::to_string_pretty(&*st) {
        let _ = fs::write(&path, txt);
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let ts = now_ms() as u64;
    let c = CTR.fetch_add(1, Ordering::Relaxed);
    format!("{}-{:x}-{:x}", prefix, ts, c)
}

// ───────────────────────── Internal API (chat::send 用) ──

/// 反查 conversation 对应的 project_id (chat::send 注入 CLAUDE.md 时用)
pub fn project_id_of_conversation(conversation_id: &str) -> Option<String> {
    STATE
        .read()
        .conversations
        .iter()
        .find(|c| c.id == conversation_id)
        .map(|c| c.project_id.clone())
}

/// 列出所有未归档的项目 (claude_md 模块用,避免直接锁 STATE)
pub fn list_active_projects() -> Vec<Project> {
    STATE
        .read()
        .projects
        .iter()
        .filter(|p| !p.archived)
        .cloned()
        .collect()
}

pub fn append_message(conversation_id: &str, role: &str, content: &str) -> Result<String> {
    let id = new_id("m");
    let now = now_ms();
    {
        let mut st = STATE.write();
        // 找到 conversation, 顺便更新 updated_at + 推断 title (首条 user 消息)
        let mut should_set_title: Option<String> = None;
        for c in st.conversations.iter_mut() {
            if c.id == conversation_id {
                c.updated_at = now;
                if c.title == "新对话" && role == "user" {
                    let snippet: String = content.chars().take(24).collect();
                    should_set_title = Some(snippet);
                }
                break;
            }
        }
        if let Some(t) = should_set_title {
            for c in st.conversations.iter_mut() {
                if c.id == conversation_id {
                    c.title = t;
                    break;
                }
            }
        }
        st.messages.push(Message {
            id: id.clone(),
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: now,
        });
    }
    persist();
    Ok(id)
}

// ───────────────────────── Tauri commands ────────────────

#[tauri::command]
pub fn conv_list_projects() -> Vec<Project> {
    STATE
        .read()
        .projects
        .iter()
        .filter(|p| !p.archived)
        .cloned()
        .collect()
}

#[tauri::command]
pub fn conv_create_project(name: String) -> Result<Project, String> {
    let p = Project {
        id: new_id("p"),
        name: if name.trim().is_empty() {
            "新项目".into()
        } else {
            name.trim().to_string()
        },
        created_at: now_ms(),
        archived: false,
    };
    STATE.write().projects.push(p.clone());
    persist();
    Ok(p)
}

#[tauri::command]
pub fn conv_archive_project(project_id: String) -> Result<(), String> {
    let mut st = STATE.write();
    for p in st.projects.iter_mut() {
        if p.id == project_id {
            p.archived = true;
        }
    }
    drop(st);
    persist();
    Ok(())
}

#[tauri::command]
pub fn conv_list_conversations(project_id: String) -> Vec<Conversation> {
    let mut list: Vec<Conversation> = STATE
        .read()
        .conversations
        .iter()
        .filter(|c| c.project_id == project_id)
        .cloned()
        .collect();
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    list
}

#[tauri::command]
pub fn conv_create_conversation(project_id: String) -> Result<Conversation, String> {
    let st = STATE.read();
    if !st.projects.iter().any(|p| p.id == project_id) {
        return Err(format!("project {} 不存在", project_id));
    }
    drop(st);
    let now = now_ms();
    let c = Conversation {
        id: new_id("c"),
        project_id,
        title: "新对话".into(),
        created_at: now,
        updated_at: now,
    };
    STATE.write().conversations.push(c.clone());
    persist();
    Ok(c)
}

#[tauri::command]
pub fn conv_delete_conversation(conversation_id: String) -> Result<(), String> {
    let mut st = STATE.write();
    st.conversations.retain(|c| c.id != conversation_id);
    st.messages.retain(|m| m.conversation_id != conversation_id);
    drop(st);
    persist();
    Ok(())
}

#[tauri::command]
pub fn conv_get_messages(conversation_id: String) -> Vec<Message> {
    let mut list: Vec<Message> = STATE
        .read()
        .messages
        .iter()
        .filter(|m| m.conversation_id == conversation_id)
        .cloned()
        .collect();
    list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    list
}

#[tauri::command]
pub fn conv_rename_conversation(conversation_id: String, title: String) -> Result<(), String> {
    let mut st = STATE.write();
    for c in st.conversations.iter_mut() {
        if c.id == conversation_id {
            c.title = title.clone();
            c.updated_at = now_ms();
        }
    }
    drop(st);
    persist();
    Ok(())
}
