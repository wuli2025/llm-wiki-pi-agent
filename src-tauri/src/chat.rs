//! 板块 ① 对话核心 — Polaris Lite (pi 内核版)
//!
//! 轻量版内核: 把原 Claude Code (`claude` CLI) 换成更轻量的 pi coding-agent
//! (`pi` CLI, npm 包 `@earendil-works/pi-coding-agent`)。
//! - chat_send: 组装 prompt(Skill/KB/目标模式 注入) -> spawn `pi -p --mode json`
//!   (prompt 走 stdin, 规避 Windows 命令行长度限制) -> emit chat:stream
//! - pi 以「每行一个 JSON 事件」流式输出 (AgentSessionEvent): message_update(text_delta)
//!   逐字增量、tool_execution_start 工具调用、message_end 携带 usage、agent_end 收尾。
//! - 同时读 stdout + stderr (单独线程), stderr 转 error 事件
//! - child.wait 完成后, 检查 exit code, 非 0 时 emit error
//! - 模型/供应商由板块⑥ provider 决定 (写 ~/.pi/agent/models.json), chat 这里只取
//!   `--model provider/id` 显式传给 pi; usage 从事件流的 AssistantMessage.usage 落账。
//! - 整合 conv 模块, 自动写 user/assistant 消息

use crate::claude_md;
use crate::conv;
use crate::kb;
use crate::skills;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use directories::UserDirs;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 给从 GUI 进程拉起的子进程加 `CREATE_NO_WINDOW`：宿主是窗口子系统、本身没有控制台，
/// 直接 spawn 控制台子系统的 docker.exe 会被分配一个新控制台 → 每次发消息都弹一个黑色
/// 终端窗口。加这个标志让它隐藏式运行，用户看不到终端。(宿主机 pi 经 pi_command 已加该标志)
#[cfg_attr(not(windows), allow(unused_variables))]
fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

pub fn init(_app: &AppHandle) -> Result<(), anyhow::Error> {
    Ok(())
}

/// 「只读」档位放行的 pi 工具 (逗号分隔, 传给 pi 的 `--tools`)。
/// pi 内置工具: read / bash / edit / write / grep / find / ls。
/// 其中 grep/find/ls 默认关闭, 需显式列入白名单才启用。
/// 只读档位: 仅读类工具 + 联网用的 bash 一并放行? 不 —— 只读就纯只读, 不给 bash/写/改。
const READONLY_TOOLS: &str = "read,grep,find,ls";

/// 非「只读」档位放行的 pi 工具: 读 + 写改 + bash 执行, 让成品(脚本/网页/报告)能真正产出。
/// 说明: pi 的 headless(`-p`)模式没有逐次「同意」交互, 启用的工具会自动执行,
/// 故「手动 / 自动」在轻量版里都等价于「放行完整工具集」, 区别仅由「只读」档位提供兜底。
const FULL_TOOLS: &str = "read,write,edit,bash,grep,find,ls";

/// 按权限档位 (cli_value: default | acceptEdits | plan) 组装 pi 的 `--tools` 白名单。
/// - plan (拒绝授权 / 只读): 仅读类工具, 不放行写改 / bash 执行;
/// - default / acceptEdits (手动 / 自动): 完整工具集, 成品能真正落地。
fn allowed_tools_for(perm: &str) -> String {
    if perm == "plan" {
        READONLY_TOOLS.to_string()
    } else {
        FULL_TOOLS.to_string()
    }
}

// ───────────────────────── Types ─────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Manual,
    AutoCurrent,
    AutoAll,
    Deny,
}

impl PermissionMode {
    fn cli_value(&self) -> &'static str {
        match self {
            PermissionMode::Manual => "default",
            PermissionMode::AutoCurrent => "acceptEdits",
            // AutoAll 不再 bypass permissions，与 AutoCurrent 一致
            PermissionMode::AutoAll => "acceptEdits",
            PermissionMode::Deny => "plan",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendArgs {
    pub prompt: String,
    pub permission_mode: PermissionMode,
    #[serde(default)]
    pub use_sandbox: bool,
    #[serde(default)]
    pub skill_ids: Option<Vec<String>>,
    #[serde(default)]
    pub conversation_id: Option<String>,
    /// 目标模式：完成条件。设置后注入「持续推进直到达成」指令。
    #[serde(default)]
    pub goal: Option<String>,
    /// 「请教毛主席」：注入毛选式客观分析指令，调用毛主席资料库，生成标注来源的 HTML。
    #[serde(default)]
    pub consult_mao: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamEvent {
    pub req_id: String,
    pub kind: String, // delta | tool | error | done
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

// ───────────────────────── State ─────────────────────────

static CHILDREN: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, Child>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
static REQ_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_req_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let c = REQ_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("req-{:x}-{:x}", ts, c)
}

// ───────────────────────── Commands ──────────────────────

#[tauri::command]
pub async fn chat_send(app: AppHandle, args: ChatSendArgs) -> Result<String, String> {
    let req_id = next_req_id();

    // 把 user 消息写入对话历史 (若提供 conversation_id)
    if let Some(cid) = &args.conversation_id {
        let _ = conv::append_message(cid, "user", &args.prompt);
    }

    // 产物目录 (每个会话一份): claude 把成品文件写到这里 → 侧边栏可预览
    let art_dir = artifacts_dir(args.conversation_id.as_deref());
    let _ = std::fs::create_dir_all(&art_dir);
    let art_before = dir_snapshot(&art_dir);

    // 一体注入: Skill prompt → KB CLAUDE.md + kb_search 召回 → 用户问题
    let current_project_id = args
        .conversation_id
        .as_deref()
        .and_then(conv::project_id_of_conversation);
    let cm_ctx = claude_md::render_for_project(current_project_id.as_deref(), &args.prompt);

    let mut final_prompt = String::new();

    // 1. Skill system prompts —— 显式点选 + 按任务意图自动激活（去重）
    let mut injected: Vec<String> = Vec::new();
    // 1a. 用户在对话框显式激活的 skill
    if let Some(ids) = &args.skill_ids {
        for id in ids {
            if injected.iter().any(|x| x == id) {
                continue;
            }
            if let Some((meta, system_prompt)) = skills::find(id) {
                final_prompt.push_str(&system_prompt);
                final_prompt.push('\n');
                injected.push(meta.id);
            }
        }
    }
    // 1b. 按任务意图自动激活（即使对话框没点选）：
    //     创建技能 → skill-creator；网页/浏览器自动化 → cloak-browser
    for (meta, system_prompt) in skills::auto_skills_for_intent(&args.prompt) {
        if injected.iter().any(|x| *x == meta.id) {
            continue;
        }
        final_prompt.push_str(&system_prompt);
        final_prompt.push('\n');
        injected.push(meta.id);
    }
    if !final_prompt.is_empty() {
        final_prompt.push_str("\n---\n\n");
    }

    // 2. 输出文件约定 (Polaris) — 让成品文件落到产物目录, 侧边栏即可预览
    final_prompt.push_str(&output_convention(&art_dir));
    final_prompt.push_str("\n\n---\n\n");

    // 2.5 目标模式: 用户设了完成条件时, 注入「持续推进直到达成」指令
    if let Some(goal) = args
        .goal
        .as_deref()
        .map(str::trim)
        .filter(|g| !g.is_empty())
    {
        final_prompt.push_str(&goal_directive(goal));
        final_prompt.push_str("\n\n---\n\n");
    }

    // 2.6 请教毛主席: 注入毛选式客观分析指令(调资料库 + 生成标来源 HTML)
    if args.consult_mao {
        final_prompt.push_str(&mao_consult_directive(&art_dir));
        final_prompt.push_str("\n\n---\n\n");
    }

    // 2.7 生图能力检测: 用户想生成图片, 但供应商坞里全是文本/代码大模型, 没有一个能真生图。
    //     注入「当前供应商 + 能否真生图」的事实, 让 image-gen 技能据此决定:
    //     不支持 → 用中文说清楚, 并改用「很有图片质感的 HTML」兜底。
    //     模型有时不遵守「开头摊牌」指令(会先说「已生成」), 所以由后端在回复最前面
    //     **确定性地**插入这句中文说明(见下方 image_notice), 保证用户一上来就看到。
    let image_notice: Option<String> = if skills::detect_image_intent(&args.prompt) {
        let (provider_name, supported) = crate::provider::image_gen_capability();
        final_prompt.push_str(&image_capability_directive(&provider_name, supported, &art_dir));
        final_prompt.push_str("\n\n---\n\n");
        if supported {
            None
        } else {
            Some(format!(
                "> ⚠️ **说明**：你当前使用的「{}」是文本大模型，**不支持生成真实图片**。下面用一张「HTML 模拟的画面」来替代；如需真实 AI 生图，请在「API 供应商」里配置支持文生图的图像接口。\n\n",
                provider_name
            ))
        }
    } else {
        None
    };

    // 3. CLAUDE.md 上下文
    if !cm_ctx.is_empty() {
        final_prompt.push_str(&cm_ctx);
        final_prompt.push_str("\n\n## 用户问题\n\n");
    }

    // 4. 用户原始问题
    final_prompt.push_str(&args.prompt);

    let perm = args.permission_mode.cli_value();
    let conv_id_opt = args.conversation_id.clone();

    // 默认走宿主机执行；开启沙箱时在容器内调起 pi。prompt 均经 stdin 喂入。
    let mut child = if args.use_sandbox {
        spawn_in_sandbox(&final_prompt, perm)?
    } else {
        spawn_on_host(&final_prompt, perm, &art_dir)?
    };

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "pi 子进程没有 stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "pi 子进程没有 stderr".to_string())?;

    CHILDREN.lock().insert(req_id.clone(), child);

    // stderr 读线程: 任何 stderr 行都 emit 为 error 事件; 累积起来给 wait 用
    let app_err = app.clone();
    let req_err = req_id.clone();
    let conv_id_err = conv_id_opt.clone();
    let stderr_buf = Arc::new(Mutex::new(String::new()));
    let stderr_buf_clone = stderr_buf.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            if line.trim().is_empty() {
                continue;
            }
            stderr_buf_clone.lock().push_str(&line);
            stderr_buf_clone.lock().push('\n');
            emit_event(
                &app_err,
                ChatStreamEvent {
                    req_id: req_err.clone(),
                    kind: "error".into(),
                    text: Some(format!("[stderr] {}", line)),
                    tool: None,
                    conversation_id: conv_id_err.clone(),
                },
            );
        }
    });

    // stdout 读线程: stream-json -> 事件; 累积 assistant 文本 + 产物路径
    let app_out = app.clone();
    let req_out = req_id.clone();
    let conv_id_thread = conv_id_opt.clone();
    let stderr_buf_for_done = stderr_buf.clone();
    let art_dir_thread = art_dir.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut assistant_text = String::new();
        // 生图不支持时: 后端确定性地把中文说明作为**第一段**发出去并计入正文,
        // 不依赖模型遵守「开头摊牌」指令 → 用户一定先看到「当前模型不支持生图」。
        if let Some(notice) = image_notice {
            assistant_text.push_str(&notice);
            emit_event(
                &app_out,
                ChatStreamEvent {
                    req_id: req_out.clone(),
                    kind: "delta".into(),
                    text: Some(notice),
                    tool: None,
                    conversation_id: conv_id_thread.clone(),
                },
            );
        }
        // 本轮生成的成品文件 (绝对路径, 正斜杠), 既来自 Write/Edit 工具调用,
        // 也来自产物目录的前后快照 diff (覆盖 Bash/脚本生成的文件)
        let mut artifacts: Vec<String> = Vec::new();
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(&line) {
                Ok(v) => handle_stream_event(
                    &app_out,
                    &req_out,
                    conv_id_thread.as_deref(),
                    &v,
                    &mut assistant_text,
                    &mut artifacts,
                ),
                Err(_) => {
                    // 非 JSON 行: 当作 delta 直接显示 (调试友好)
                    assistant_text.push_str(&line);
                    assistant_text.push('\n');
                    emit_event(
                        &app_out,
                        ChatStreamEvent {
                            req_id: req_out.clone(),
                            kind: "delta".into(),
                            text: Some(line),
                            tool: None,
                            conversation_id: conv_id_thread.clone(),
                        },
                    );
                }
            }
        }

        // 等子进程退出, 检查 exit code (不能持锁 wait, 否则 chat_cancel 死锁)
        let child_opt = CHILDREN.lock().remove(&req_out);
        let exit_msg: Option<String> = if let Some(mut child) = child_opt {
            match child.wait() {
                Ok(status) => {
                    if !status.success() {
                        let stderr_txt = stderr_buf_for_done.lock().clone();
                        Some(format!(
                            "pi 进程异常退出 (exit code={:?})\n--- stderr ---\n{}",
                            status.code(),
                            if stderr_txt.is_empty() {
                                "(stderr 为空)".to_string()
                            } else {
                                stderr_txt
                            }
                        ))
                    } else {
                        None
                    }
                }
                Err(e) => Some(format!("等待 pi 进程失败: {}", e)),
            }
        } else {
            None
        };

        if let Some(msg) = exit_msg {
            emit_event(
                &app_out,
                ChatStreamEvent {
                    req_id: req_out.clone(),
                    kind: "error".into(),
                    text: Some(msg),
                    tool: None,
                    conversation_id: conv_id_thread.clone(),
                },
            );
        }

        // 产物目录前后快照 diff: 捕获 Bash / 脚本 / Skill 生成的新增或改动文件
        let art_after = dir_snapshot(&art_dir_thread);
        for (path, mtime) in art_after.iter() {
            let changed = match art_before.get(path) {
                None => true,
                Some(old) => mtime > old,
            };
            if !changed {
                continue;
            }
            let s = path.to_string_lossy().replace('\\', "/");
            if !artifacts.contains(&s) {
                artifacts.push(s.clone());
                emit_event(
                    &app_out,
                    ChatStreamEvent {
                        req_id: req_out.clone(),
                        kind: "artifact".into(),
                        text: Some(s),
                        tool: None,
                        conversation_id: conv_id_thread.clone(),
                    },
                );
            }
        }

        // 持久化 assistant 消息 (产物清单以注释 marker 形式存入正文, 重载历史时解析)
        if let Some(cid) = &conv_id_thread {
            let mut content = assistant_text.trim().to_string();
            if !artifacts.is_empty() {
                if let Ok(json) = serde_json::to_string(&artifacts) {
                    content.push_str(&format!("\n\n{}{}-->", ARTIFACT_MARKER_PREFIX, json));
                }
            }
            if !content.trim().is_empty() {
                let _ = conv::append_message(cid, "assistant", &content);
            }
        }

        emit_event(
            &app_out,
            ChatStreamEvent {
                req_id: req_out.clone(),
                kind: "done".into(),
                text: None,
                tool: None,
                conversation_id: conv_id_thread.clone(),
            },
        );
    });

    Ok(req_id)
}

#[tauri::command]
pub fn chat_cancel(req_id: String) -> Result<(), String> {
    if let Some(mut child) = CHILDREN.lock().remove(&req_id) {
        let _ = child.kill();
    }
    Ok(())
}

// ───────────────────────── Internals ─────────────────────

fn handle_stream_event(
    app: &AppHandle,
    req_id: &str,
    conv_id: Option<&str>,
    v: &Value,
    accum: &mut String,
    artifacts: &mut Vec<String>,
) {
    let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
    match t {
        // 流式增量: pi 的 message_update 携带 assistantMessageEvent (text_delta / error)
        "message_update" => {
            let Some(ev) = v.get("assistantMessageEvent") else {
                return;
            };
            let et = ev.get("type").and_then(|x| x.as_str()).unwrap_or("");
            match et {
                "text_delta" => {
                    if let Some(delta) = ev.get("delta").and_then(|x| x.as_str()) {
                        if !delta.is_empty() {
                            accum.push_str(delta);
                            emit_event(
                                app,
                                ChatStreamEvent {
                                    req_id: req_id.into(),
                                    kind: "delta".into(),
                                    text: Some(delta.to_string()),
                                    tool: None,
                                    conversation_id: conv_id.map(|s| s.to_string()),
                                },
                            );
                        }
                    }
                }
                "error" => {
                    let msg = ev
                        .get("error")
                        .and_then(|e| e.get("errorMessage"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("(unknown error)")
                        .to_string();
                    emit_event(
                        app,
                        ChatStreamEvent {
                            req_id: req_id.into(),
                            kind: "error".into(),
                            text: Some(msg),
                            tool: None,
                            conversation_id: conv_id.map(|s| s.to_string()),
                        },
                    );
                }
                _ => {}
            }
        }
        // 工具调用开始: pi 的 tool_execution_start { toolName, args }
        "tool_execution_start" => {
            let name = v
                .get("toolName")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown");
            emit_event(
                app,
                ChatStreamEvent {
                    req_id: req_id.into(),
                    kind: "tool".into(),
                    text: None,
                    tool: Some(name.to_string()),
                    conversation_id: conv_id.map(|s| s.to_string()),
                },
            );
            // 写/改文件类工具 → 记一个成品文件 (实时反馈)。pi 的 write/edit 参数字段是 `path`。
            if name.eq_ignore_ascii_case("write") || name.eq_ignore_ascii_case("edit") {
                let fp = v
                    .get("args")
                    .and_then(|a| a.get("path").or_else(|| a.get("file_path")))
                    .and_then(|x| x.as_str());
                if let Some(fp) = fp {
                    let norm = fp.replace('\\', "/");
                    if !artifacts.contains(&norm) {
                        artifacts.push(norm.clone());
                        emit_event(
                            app,
                            ChatStreamEvent {
                                req_id: req_id.into(),
                                kind: "artifact".into(),
                                text: Some(norm),
                                tool: None,
                                conversation_id: conv_id.map(|s| s.to_string()),
                            },
                        );
                    }
                }
            }
        }
        // 一条 assistant 消息结束: 落 token 用量 + (极少数无 delta 的情况)兜底显示正文
        "message_end" => {
            let Some(msg) = v.get("message") else { return };
            if msg.get("role").and_then(|x| x.as_str()) != Some("assistant") {
                return;
            }
            // 兜底: 若整条消息没经过 text_delta (个别 provider 不分块), 从 content 取 text
            if accum.is_empty() {
                if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
                    let mut txt = String::new();
                    for block in content {
                        if block.get("type").and_then(|x| x.as_str()) == Some("text") {
                            if let Some(s) = block.get("text").and_then(|x| x.as_str()) {
                                txt.push_str(s);
                            }
                        }
                    }
                    if !txt.is_empty() {
                        accum.push_str(&txt);
                        emit_event(
                            app,
                            ChatStreamEvent {
                                req_id: req_id.into(),
                                kind: "delta".into(),
                                text: Some(txt),
                                tool: None,
                                conversation_id: conv_id.map(|s| s.to_string()),
                            },
                        );
                    }
                }
            }
            // 错误收尾 (stopReason == "error")
            if msg.get("stopReason").and_then(|x| x.as_str()) == Some("error") {
                let emsg = msg
                    .get("errorMessage")
                    .and_then(|x| x.as_str())
                    .unwrap_or("(unknown error)")
                    .to_string();
                emit_event(
                    app,
                    ChatStreamEvent {
                        req_id: req_id.into(),
                        kind: "error".into(),
                        text: Some(emsg),
                        tool: None,
                        conversation_id: conv_id.map(|s| s.to_string()),
                    },
                );
            }
            // 落 token 用量 (供「用量看板」)
            if let Some(u) = msg.get("usage") {
                let provider = msg.get("provider").and_then(|x| x.as_str()).unwrap_or("");
                let model = msg.get("model").and_then(|x| x.as_str()).unwrap_or("");
                let g = |k: &str| u.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
                let cost = u
                    .get("cost")
                    .and_then(|c| c.get("total"))
                    .and_then(|x| x.as_f64())
                    .unwrap_or(0.0);
                crate::provider::record_usage(
                    provider,
                    model,
                    g("input"),
                    g("output"),
                    g("cacheRead"),
                    g("cacheWrite"),
                    cost,
                );
            }
        }
        _ => {}
    }
}

fn emit_event(app: &AppHandle, ev: ChatStreamEvent) {
    let _ = app.emit("chat:stream", ev);
}

/// pi 的公共 headless 参数 (不含 prompt —— prompt 走 stdin)。
/// - `-p --mode json`: 单次非交互, 每行一个 JSON 事件流式输出;
/// - `--no-session`: 会话历史由板块 conv 自管, pi 不另存;
/// - `--tools`: 按权限档位放行的工具白名单;
/// - `--model provider/id`: 由板块⑥ provider 决定当前供应商/模型 (写在 ~/.pi/agent/models.json)。
fn pi_args(perm: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-p".into(),
        "--mode".into(),
        "json".into(),
        "--no-session".into(),
        "--tools".into(),
        allowed_tools_for(perm),
    ];
    if let Some(model_ref) = crate::provider::current_model_ref() {
        args.push("--model".into());
        args.push(model_ref);
    }
    args
}

/// 把 prompt 写进子进程 stdin 并关闭 (EOF), pi 据此读取 initialMessage。
/// 单独线程写, 防止超长 prompt 把管道写满时与我们读 stdout 互相阻塞。
fn feed_stdin(child: &mut Child, prompt: &str) {
    if let Some(mut stdin) = child.stdin.take() {
        let data = prompt.to_string();
        std::thread::spawn(move || {
            let _ = stdin.write_all(data.as_bytes());
            // drop(stdin) → 关闭管道, 触发 EOF
        });
    }
}

fn spawn_in_sandbox(prompt: &str, perm: &str) -> Result<Child, String> {
    // 沙箱内以 /workspace 为 cwd 调起 pi (docker exec -i 让 stdin 贯通)。
    let mut cmd = Command::new("docker");
    cmd.args([
        "exec",
        "-i",
        "-w",
        "/workspace",
        polaris_sandbox::CONTAINER_NAME,
        "pi",
    ]);
    cmd.args(pi_args(perm));
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_window(&mut cmd); // 隐藏式: 不弹控制台窗口
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("在沙箱内调起 pi 失败: {}", e))?;
    feed_stdin(&mut child, prompt);
    Ok(child)
}

/// 构造调起宿主机 pi 的 Command。
/// Windows 关键: npm 全局装出来的是 `pi.cmd` 批处理垫片, 而 Rust 的 `Command::new`
/// 走 CreateProcess 只会补 `.exe` 不认 `.cmd` —— 直接 `Command::new("pi")` 会找不到。
/// 故 Windows 下经 `cmd /c pi ...` 调起 (并加 CREATE_NO_WINDOW 防黑框闪现)。
fn pi_command() -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = Command::new("cmd");
        c.arg("/c").arg("pi");
        c.creation_flags(CREATE_NO_WINDOW);
        c
    }
    #[cfg(not(windows))]
    {
        Command::new("pi")
    }
}

fn spawn_on_host(prompt: &str, perm: &str, _art_dir: &Path) -> Result<Child, String> {
    // cwd = polaris 应用根 (CLAUDE.md / AGENTS.md / PolarisKB 所在),
    // pi 会自动从 cwd 读取 AGENTS.md / CLAUDE.md 上下文文件。
    let cwd = claude_md::project_root().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });

    let mut cmd = pi_command();
    cmd.args(pi_args(perm))
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_window(&mut cmd); // 隐藏式: 每次发消息不再弹出黑色终端窗口
    let mut child = cmd
        .spawn()
        .map_err(|e| {
            format!("调起 pi CLI 失败: {} (是否已安装? 可在「环境医生」一键安装 pi)", e)
        })?;
    feed_stdin(&mut child, prompt);
    Ok(child)
}

// ───────────────────────── Artifacts (产物预览) ─────────────────────────

/// assistant 正文里夹带的产物清单 marker 前缀; 完整形如
/// `<!--POLARIS_ARTIFACTS:["C:/a/b.html"]-->`, 重载历史时由前端解析并隐藏。
pub const ARTIFACT_MARKER_PREFIX: &str = "<!--POLARIS_ARTIFACTS:";

/// 每个会话一个目录。优先落到「工作文件夹」(KB root) 下，让产物与用户的知识库
/// 同处一地、可见可备份：`<kb_root>/conversations/<id>/`。
/// KB root 不可用时回退到 `~/Polaris/data/artifacts/<id>`。
fn conversation_dir(conv_id: Option<&str>) -> PathBuf {
    let id = conv_id.unwrap_or("scratch");
    let kb_root = PathBuf::from(kb::kb_root());
    if !kb_root.as_os_str().is_empty() && kb_root.exists() {
        kb_root.join("conversations").join(id)
    } else {
        UserDirs::new()
            .map(|u| u.home_dir().join("Polaris").join("data").join("artifacts"))
            .unwrap_or_else(|| PathBuf::from("artifacts"))
            .join(id)
    }
}

/// 产物(成品)目录: 会话目录下的 `outputs/`。claude 把成品写到这里 → 侧边栏可预览。
fn artifacts_dir(conv_id: Option<&str>) -> PathBuf {
    conversation_dir(conv_id).join("outputs")
}

/// 递归快照目录里的文件 → mtime, 用于前后 diff 找新增/改动文件
fn dir_snapshot(dir: &Path) -> HashMap<PathBuf, SystemTime> {
    let mut m = HashMap::new();
    if !dir.exists() {
        return m;
    }
    for entry in WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(mt) = meta.modified() {
                    m.insert(entry.path().to_path_buf(), mt);
                }
            }
        }
    }
    m
}

/// 注入给 claude 的「输出文件约定」, 引导成品落到产物目录
fn output_convention(art_dir: &Path) -> String {
    let dir = art_dir.to_string_lossy().replace('\\', "/");
    format!(
        "## 输出文件约定 (Polaris)\n\n\
当你生成任何可供用户**查看或下载的成品文件**(HTML 网页 / 数据可视化 / 报告 / Markdown / 图片 / CSV / PDF 等)时,请遵守:\n\n\
1. 把成品文件保存到这个已授权可写的目录(用绝对路径):\n   `{dir}`\n\
2. 网页类成品请优先生成**单文件、自包含的 HTML**(把 CSS/JS 内联进去),以便在侧边栏直接预览。\n\
3. 在回答末尾用一句话点明你生成了哪些文件(文件名即可)。\n\n\
普通问答无需创建文件。",
        dir = dir
    )
}

/// 目标模式指令 (Goal Mode 复刻): 把用户设定的「完成条件」当作直接指令注入 prompt,
/// 引导 agent 持续推进直到真正达成 —— 条件未满足前不收尾、不反问, 自行规划下一步。
/// 内核无关: pi 本身就是一个会多轮调用工具的 agentic 循环, 注入此指令即可让它像
/// Claude Code 的 goal 模式那样「咬住目标不放」, 把整个达成过程在一次会话内自驱完成。
fn goal_directive(goal: &str) -> String {
    format!(
        "## 目标模式 (Goal Mode)\n\n\
本轮已开启**目标模式**。用户设定的完成条件是:\n\n\
> {goal}\n\n\
把这个条件本身当作你的指令, 持续推进直到它真正达成:\n\
1. 条件未满足时不要收尾, 也不要反问用户「接下来做什么」—— 自行规划并执行下一步。\n\
2. 每完成一步, 对照条件自检是否已达成; 未达成就继续做, 直到满足为止。\n\
3. 条件达成后, 明确说明它已达成, 并简述你是如何确认的。\n\
4. 仅当遇到无法自行解决的硬阻塞(如缺少凭据 / 权限 / 外部依赖)时, 才停下来向用户说明原因。",
        goal = goal
    )
}

/// 生图能力指令: 把「当前供应商 + 能否真生图」作为事实交给模型。
/// supported=false(绝大多数情况)时, 要求一开始就用中文讲清「当前模型不支持生成真实图片」,
/// 再用「很有图片质感的自包含 HTML」兜底; supported=true 才允许走真实图像 API。
fn image_capability_directive(provider_name: &str, supported: bool, art_dir: &Path) -> String {
    let dir = art_dir.to_string_lossy().replace('\\', "/");
    if supported {
        format!(
            "## 生图能力检测 (Image Capability)\n\n\
本轮检测到用户想**生成图片**, 且环境里配置了独立的图像 API 密钥(`OPENAI_API_KEY`)。\n\
- 可以走真实文生图: 按 image-gen 技能的说明调用图像 API 生成位图, 存到产物目录(绝对路径): `{dir}`。\n\
- 若调用过程中报错(额度 / 网络 / 该 key 无图像权限), **立即用中文如实告知用户**, 再用下面的 HTML 兜底, 不要假装已生成。",
            dir = dir
        )
    } else {
        format!(
            "## 生图能力检测 (Image Capability) — 关键\n\n\
本轮检测到用户想**生成图片(写实照片 / AI 绘画类位图)**。但用户当前用的供应商是 **「{provider}」**, \
它(以及供应商坞里其它走 Anthropic 协议的文本 / 代码大模型)**并不具备文生图能力**, 环境里也没有配置独立的图像生成 API 密钥。\n\n\
因此请**严格**这样做:\n\
1. 本应用**已经在你这条回复的最前面自动插入了一句中文说明**(「你当前使用的「{provider}」不支持生成真实图片…」), 用户一定会先看到它。所以**你不要再重复这句开头、也不要说「已生成」**, 直接从下面第 2 步动手。\n\
2. **用「很有图片质感」的自包含 HTML 兜底**: 按 image-gen 技能的要求, 用 CSS 渐变 / SVG / 几何构图 / 排版做出一张**看起来就像那张图**的单文件 HTML(海报 / 插画 / 场景感), 存到产物目录(绝对路径): `{dir}`, 让用户在侧边栏直接看到。\n\
3. 末尾用一句中文点明: 这是用 HTML 模拟的图片效果, 如需**真实 AI 生图**, 可在「API 供应商」里配置支持文生图的图像 API(如 OpenAI 图像接口 `OPENAI_API_KEY`)。\n\
4. 例外: 如果用户其实要的是**图表 / 流程图 / 示意图 / 图标 / SVG**, 这些能用代码(SVG / HTML / matplotlib)直接画出来, **不受上面限制** —— 正常生成即可, 无需声明「不支持」。",
            provider = provider_name,
            dir = dir
        )
    }
}

/// 「请教毛主席」指令: 让 claude 以毛主席(毛选)的口吻和思想方法, 沿毛主席资料库
/// 客观分析用户的问题, 并生成一份标注来源的自包含 HTML。资料库(结构化 wiki)已由
/// `claude_md::render_for_project` 以长上下文 + 双链地图注入, 用 Read/Glob/Grep 沿双链自取。
fn mao_consult_directive(art_dir: &Path) -> String {
    let dir = art_dir.to_string_lossy().replace('\\', "/");
    format!(
        "## 请教毛主席 (Consult Mode)\n\n\
本轮用户开启了「请教毛主席」模式。请你 **化身毛主席(毛泽东同志)**, 就同志提出的问题给出回答, \
**核心交付物是一份 HTML 文件**, 对话里只作简要铺垫。\n\n\
**口吻与文风(毛选风格 · 大白话第一)**\n\
- 写成《毛泽东选集》那种调子: 平实有力、口语化, 多用大白话、多打比方, 庄稼汉、工人都听得懂; \
不要堆术语、不要掉书袋、不要写论文腔。\n\
- 称呼用户为「同志」或「小同志」, 自称用「我」。\n\
- 句子要短, 段落要短, 一段讲一个道理; 道理要落到「该怎么办」上, 不空谈。\n\n\
**分析方法**(自觉运用毛主席的思想方法, 客观地分析)\n\
- 实事求是, 一切从实际出发; 调查研究, 没有调查就没有发言权。\n\
- 矛盾分析法: 抓主要矛盾和矛盾的主要方面, 一分为二地看问题(两点论)。\n\
- 具体问题具体分析, 反对教条主义; 从群众中来, 到群众中去。\n\n\
**面向未来的视角(必须)**\n\
- 不能只就事论事停在当下, 要 **站在未来的眼光回头看今天这件事**: 长远走向是什么? \
五年、十年、再远些会怎么演变? 现在的小苗头, 将来会长成什么样的大势?\n\
- 善于看「萌芽中的新东西」: 哪些事情现在看着不起眼, 将来要成主流; 哪些事情现在风光, \
将来要被时代抛掉。把这个判断讲清楚, 才算给同志一个真有用的答复。\n\n\
**引用要克制(重要)**\n\
- **正文一开头不要堆原文/语录**, 不要拿《矛盾论》《实践论》整段往上糊。先用大白话把问题讲透, \
该点出处就一句话带过(例如「这就是《矛盾论》里讲的抓主要矛盾」)。\n\
- 详尽的篇目引用、原文摘抄, **统一放到 HTML 文件末尾的「来源」一节**, 不要散在正文上头。\n\n\
**立场底线**\n\
- 立场坚定地拥护社会主义、共产主义事业和人民利益; **回答不得违背共产主义、社会主义的基本原则**, \
不传播违背社会主义核心价值观的内容。\n\n\
**取证 (KB-first)**\n\
- 毛主席资料库(结构化 wiki)已随本轮以长上下文 + 双链地图注入, 就在你工作目录的 `PolarisKB/` 下。\n\
- 下笔前先用 Read/Glob/Grep 翻 `raw/毛主席/` 下相关篇目(如《矛盾论》《实践论》《论持久战》《关于正确处理人民内部矛盾的问题》等)取证, \
不要凭空发挥; 引用就标明篇目名。\n\n\
**输出步骤**\n\
1. 对话里 **只写简短铺垫**: 一两段毛主席口吻的大白话, 点出抓哪个主要矛盾、看到什么未来走向。\
不要在对话里铺长篇, 详细的分析交给 HTML。\n\
2. 生成一份 **单文件、自包含的 HTML**(CSS 内联, 字体可读、排版清爽)保存到这个可写目录(用绝对路径):\n   `{dir}`\n\
   HTML 内容结构建议:\n\
     - 标题 (问题概括)\n\
     - 「实事求是」: 把问题摆平, 大白话讲清楚现状\n\
     - 「主要矛盾」: 抓住主要矛盾和矛盾的主要方面, 一分为二地看\n\
     - 「该怎么办」: 给同志几条具体的、能落地的办法\n\
     - 「站在未来看今天」: 长远走向、未来五年十年的演变、现在该种什么苗\n\
     - 「来源」: 列出引用的篇目, 必要的原文摘抄集中放这里\n\
   **正文开头不要罗列原文**, 把原文压到「来源」一节去。\n\
3. 对话末尾用一句话点明生成了哪个 HTML 文件(绝对路径), 方便同志打开。\n\n\
结尾可以用一句鼓励的话, 例如「为人民服务」「为建设共产主义事业而奋斗」。",
        dir = dir
    )
}

/// 标准 Base64 编码 (无外部依赖) — 给图片产物拼 data URL 用
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn classify_ext(ext: &str) -> &'static str {
    match ext {
        "html" | "htm" => "html",
        "svg" => "svg",
        "md" | "markdown" => "markdown",
        "png" | "apng" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "avif" => "image",
        "txt" | "json" | "csv" | "tsv" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "css"
        | "scss" | "less" | "py" | "rs" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "toml"
        | "yaml" | "yml" | "xml" | "log" | "sh" | "bat" | "ps1" | "sql" | "ini" | "conf"
        | "env" | "vue" | "php" | "rb" | "kt" | "swift" | "" => "text",
        _ => "binary",
    }
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" | "apng" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactPayload {
    pub path: String,
    pub name: String,
    pub ext: String,
    /// html | svg | image | markdown | text | binary
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_url: Option<String>,
    pub size: u64,
}

#[tauri::command]
pub fn artifact_read(path: String) -> Result<ArtifactPayload, String> {
    let p = PathBuf::from(&path);
    let meta = std::fs::metadata(&p).map_err(|_| format!("文件不存在或无法访问: {}", path))?;
    if !meta.is_file() {
        return Err("目标不是文件".into());
    }
    let size = meta.len();
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    let ext = p
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let kind = classify_ext(&ext);

    match kind {
        "image" => {
            const MAX: u64 = 25 * 1024 * 1024;
            if size > MAX {
                return Err("图片过大, 无法预览 (>25MB)".into());
            }
            let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
            let data_url = format!("data:{};base64,{}", mime_for(&ext), base64_encode(&bytes));
            Ok(ArtifactPayload {
                path,
                name,
                ext,
                kind: kind.into(),
                text: None,
                data_url: Some(data_url),
                size,
            })
        }
        "binary" => Ok(ArtifactPayload {
            path,
            name,
            ext,
            kind: kind.into(),
            text: None,
            data_url: None,
            size,
        }),
        _ => {
            // html / svg / markdown / text
            const MAX: u64 = 8 * 1024 * 1024;
            if size > MAX {
                return Err("文件过大, 无法预览 (>8MB)".into());
            }
            let text = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
            Ok(ArtifactPayload {
                path,
                name,
                ext,
                kind: kind.into(),
                text: Some(text),
                data_url: None,
                size,
            })
        }
    }
}

/// 用系统默认程序打开产物文件 (浏览器开 HTML / 看图器开图片等)
#[tauri::command]
pub fn artifact_open_external(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 在系统文件管理器中定位并选中该产物文件 (Windows 资源管理器 / macOS Finder)。
/// Linux 无统一「选中文件」语义, 退化为打开其所在目录。
#[tauri::command]
pub fn artifact_reveal(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // explorer /select 需要反斜杠路径; 用 raw_arg 让路径被正确引号包裹
        let win_path = path.replace('/', "\\");
        Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", win_path))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        Command::new("xdg-open")
            .arg(&parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 「参考资料」文件夹视图的一条文件记录。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactEntry {
    /// 绝对路径 (正斜杠), 供 artifact_read / openExternal 用
    pub path: String,
    pub name: String,
    pub ext: String,
    /// html | svg | image | markdown | text | binary —— 前端选图标 / 预览方式
    pub kind: String,
    pub size: u64,
    /// 修改时间 (Unix 秒), 前端按此倒序 + 显示
    pub modified: u64,
}

/// 列出某会话产物目录下的全部成品文件, 按修改时间倒序 (最新在前)。
/// 供右侧抽屉「参考资料」以文件夹视图按时间排列、点开即预览。
#[tauri::command]
pub fn artifact_list(conversation_id: Option<String>) -> Vec<ArtifactEntry> {
    let dir = artifacts_dir(conversation_id.as_deref());
    let mut entries: Vec<ArtifactEntry> = Vec::new();
    if !dir.exists() {
        return entries;
    }
    for w in WalkDir::new(&dir).into_iter().flatten() {
        if !w.file_type().is_file() {
            continue;
        }
        let p = w.path();
        let meta = match w.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        // 跳过隐藏 / 临时文件
        if name.starts_with('.') {
            continue;
        }
        let ext = p
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        entries.push(ArtifactEntry {
            path: p.to_string_lossy().replace('\\', "/"),
            name,
            ext: ext.clone(),
            kind: classify_ext(&ext).to_string(),
            size: meta.len(),
            modified,
        });
    }
    entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    entries
}

/// 跨「所有对话」产物的搜索命中。供历史对话记忆检索把过往输出文件也算入。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSearchHit {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub conversation_id: String,
    pub snippet: String,
    pub modified: u64,
    pub score: i32,
}

/// 所有「会话根目录」候选: 工作文件夹(KB root)/conversations 与回退目录。
fn conversation_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let kb_root = PathBuf::from(kb::kb_root());
    if !kb_root.as_os_str().is_empty() && kb_root.exists() {
        roots.push(kb_root.join("conversations"));
    }
    if let Some(u) = UserDirs::new() {
        roots.push(u.home_dir().join("Polaris").join("data").join("artifacts"));
    }
    roots
}

/// 在所有对话的 outputs 里检索: 文件名命中 +10, 正文命中 +2/次(上限), 按分数+时间排序。
/// 让「搜索以前的对话记忆」把之前输出的文件也算入。
#[tauri::command]
pub fn artifact_search(query: String) -> Vec<ArtifactSearchHit> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<ArtifactSearchHit> = Vec::new();
    for root in conversation_roots() {
        if !root.exists() {
            continue;
        }
        for w in WalkDir::new(&root).into_iter().flatten() {
            if !w.file_type().is_file() {
                continue;
            }
            let p = w.path();
            // 仅 conversations/<id>/outputs/** 下的文件
            let rel = match p.strip_prefix(&root) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let comps: Vec<String> = rel
                .components()
                .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
                .collect();
            // 期望 [<id>, "outputs", ...]
            if comps.len() < 3 || comps[1] != "outputs" {
                continue;
            }
            let conversation_id = comps[0].clone();
            let name = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') {
                continue;
            }
            let ext = p
                .extension()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let kind = classify_ext(&ext);
            let meta = match w.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let mut score = 0;
            let mut snippet = String::new();
            if name.to_lowercase().contains(&q) {
                score += 10;
            }
            // 文本类才读正文匹配 (限大小, 防卡)
            if matches!(kind, "text" | "markdown" | "html" | "svg") && meta.len() < 512 * 1024 {
                if let Ok(body) = std::fs::read_to_string(p) {
                    let lower = body.to_lowercase();
                    if let Some(pos) = lower.find(&q) {
                        score += 2;
                        let start = body[..pos].char_indices().rev().take(40).last().map(|(i, _)| i).unwrap_or(0);
                        let end = (pos + q.len() + 60).min(body.len());
                        let mut e = end;
                        while e < body.len() && !body.is_char_boundary(e) {
                            e += 1;
                        }
                        snippet = body[start..e].replace('\n', " ").trim().to_string();
                    }
                }
            }
            if score > 0 {
                hits.push(ArtifactSearchHit {
                    path: p.to_string_lossy().replace('\\', "/"),
                    name,
                    kind: kind.to_string(),
                    conversation_id,
                    snippet,
                    modified,
                    score,
                });
            }
        }
    }
    hits.sort_by(|a, b| b.score.cmp(&a.score).then(b.modified.cmp(&a.modified)));
    hits.truncate(50);
    hits
}

// ───────────────────────── 对话附件 (拖拽上传) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedFile {
    pub name: String,
    /// 复制后在会话 uploads 目录里的绝对路径 (正斜杠)
    pub path: String,
    /// text | image | pdf | office | binary —— 前端选图标用
    pub kind: String,
    pub size: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 对话拖拽上传:把文件复制进「会话 uploads 目录」,返回附件清单。
/// 与「知识库上传」是两条不同的路径 —— 这里只把文件挂到当前对话,
/// 前端发送时把这些绝对路径写进 prompt,claude 用 Read 工具按需读取。
#[tauri::command]
pub fn chat_attach_files(
    conversation_id: Option<String>,
    paths: Vec<String>,
) -> Vec<AttachedFile> {
    const MAX: usize = 50;
    let dir = conversation_dir(conversation_id.as_deref()).join("uploads");
    let _ = std::fs::create_dir_all(&dir);

    let mut out = Vec::new();
    for p in paths.iter().take(MAX) {
        let src = PathBuf::from(p);
        if src.is_dir() {
            // 目录:浅层展开其中的文件
            if let Ok(rd) = std::fs::read_dir(&src) {
                for e in rd.flatten() {
                    let ep = e.path();
                    if ep.is_file() && out.len() < MAX {
                        push_attach(&dir, &ep, &mut out);
                    }
                }
            }
            continue;
        }
        if !src.is_file() {
            out.push(AttachedFile {
                name: file_name_of(&src),
                path: String::new(),
                kind: "binary".into(),
                size: 0,
                ok: false,
                error: Some("文件不存在".into()),
            });
            continue;
        }
        push_attach(&dir, &src, &mut out);
    }
    out
}

fn file_name_of(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| p.to_string_lossy().to_string())
}

fn push_attach(dir: &Path, src: &Path, out: &mut Vec<AttachedFile>) {
    let name = file_name_of(src);
    let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    let dst = unique_upload_path(dir, &name);
    match std::fs::copy(src, &dst) {
        Ok(_) => out.push(AttachedFile {
            name,
            path: dst.to_string_lossy().replace('\\', "/"),
            kind: attach_kind(src).into(),
            size,
            ok: true,
            error: None,
        }),
        Err(e) => out.push(AttachedFile {
            name,
            path: String::new(),
            kind: "binary".into(),
            size,
            ok: false,
            error: Some(e.to_string()),
        }),
    }
}

fn unique_upload_path(dir: &Path, fname: &str) -> PathBuf {
    let first = dir.join(fname);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match fname.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (fname.to_string(), String::new()),
    };
    for n in 2..10_000 {
        let cand = dir.join(format!("{stem} ({n}){ext}"));
        if !cand.exists() {
            return cand;
        }
    }
    first
}

fn attach_kind(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "avif" | "svg" => "image",
        "pdf" => "pdf",
        "docx" | "doc" | "pptx" | "ppt" | "xlsx" | "xls" | "ods" | "odt" | "odp" => "office",
        "txt" | "md" | "markdown" | "csv" | "tsv" | "json" | "yaml" | "yml" | "xml" | "html"
        | "htm" | "log" | "rs" | "js" | "ts" | "py" | "go" | "java" | "c" | "cpp" | "css"
        | "vue" | "sh" | "toml" | "ini" => "text",
        _ => "binary",
    }
}
