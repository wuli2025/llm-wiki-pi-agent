//! 板块 ② 维基知识库 — MVP 实现
//!
//! 设计依据: PRD-v6 §8 + v5.1 §3-§7
//! - 三层目录铁律: raw/ output/ wiki/ (新建空 KB 时创建)
//! - 关键词加权评分搜索 (PRD §8.8): 标题 +10, 课程标签 +8, 正文 +1
//! - 双链 [[wiki-link]] 解析 -> 图谱节点+边
//! - YAML frontmatter 提取 category (PRD §8.5)
//!
//! MVP 缩水:
//! - 不做 Embedding (Karpathy 论点: 结构化 wiki + 长上下文 > 向量)
//! - 不做 SimHash 去重 (留 §8.6, 后续接入)
//! - 索引常驻内存, 进程重启时重扫 (后续走 SQLite)

use crate::convert;
use anyhow::Result;
use directories::{ProjectDirs, UserDirs};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;

// ───────────────────────── State ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct KbDoc {
    pub rel_path: String,
    pub title: String,
    pub category: String,
    pub wikilinks: Vec<String>,
    pub body: String,
}

static INDEX: Lazy<RwLock<Vec<KbDoc>>> = Lazy::new(|| RwLock::new(Vec::new()));
static KB_ROOT: Lazy<RwLock<PathBuf>> = Lazy::new(|| RwLock::new(PathBuf::new()));

// ───────────────────────── Init ──────────────────────────

pub fn init(app: &AppHandle) -> Result<()> {
    let settings = load_settings();
    let root = settings
        .kb_root
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_kb_root().unwrap_or_else(|_| PathBuf::from(".")));
    ensure_skeleton(&root)?;
    // 首启一次性播种「默认资料库」(随安装包打进来的毛主席资料库)
    seed_default_kb(app, &root);
    *KB_ROOT.write() = root.clone();
    let docs = scan_all(&root);
    *INDEX.write() = docs;
    Ok(())
}

fn default_kb_root() -> Result<PathBuf> {
    let user = UserDirs::new().ok_or_else(|| anyhow::anyhow!("no user dir"))?;
    let home = user.home_dir();
    Ok(home.join("Polaris").join("PolarisKB"))
}

// ───────────────────────── 默认资料库播种 ─────────────────────────

/// 首启一次性播种「默认资料库」: 把随安装包打进来的毛主席资料库拷到 KB 的 `raw/` 下。
/// 用一次性 marker(`<root>/.polaris_seeded`)记录, 之后即便用户在「管理」里清空、
/// 或在「浏览」里逐条删除, 重启也 **不会** 再次重播 —— 尊重用户对资料库的删除。
fn seed_default_kb(app: &AppHandle, root: &Path) {
    let marker = root.join(".polaris_seeded");
    if marker.exists() {
        return;
    }
    if let Some(src) = seed_source(app) {
        let _ = copy_dir_recursive(&src, &root.join("raw"));
    }
    // 不管有没有播到内容都打 marker, 避免每次启动都尝试拷贝
    let _ = fs::write(&marker, b"seeded\n");
}

/// 定位打进安装包的资料库种子目录(其内含 `毛主席/`)。
/// 发布版走 Tauri `resource_dir`; 开发期回退到 `src-tauri/resources/seed-kb`。
fn seed_source(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(rd) = app.path().resource_dir() {
        for cand in [rd.join("resources").join("seed-kb"), rd.join("seed-kb")] {
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("seed-kb");
    if dev.exists() {
        Some(dev)
    } else {
        None
    }
}

/// 递归拷贝目录内容到目标; 已存在的文件跳过(不覆盖用户改动)。
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in WalkDir::new(src).into_iter().flatten() {
        let p = entry.path();
        let rel = match p.strip_prefix(src) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            if !target.exists() {
                fs::copy(p, &target)?;
            }
        }
    }
    Ok(())
}

// ───────────────────────── Settings ──────────────────────

#[derive(Default, Serialize, Deserialize)]
struct AppSettings {
    kb_root: Option<String>,
}

fn settings_path() -> Result<PathBuf> {
    let pd = ProjectDirs::from("com", "polaris", "polaris-app")
        .ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    let dir = pd.config_dir().to_path_buf();
    fs::create_dir_all(&dir)?;
    Ok(dir.join("settings.json"))
}

fn load_settings() -> AppSettings {
    settings_path()
        .ok()
        .and_then(|p| fs::read_to_string(&p).ok())
        .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
        .unwrap_or_default()
}

fn save_settings(s: &AppSettings) -> Result<()> {
    let p = settings_path()?;
    fs::write(p, serde_json::to_string_pretty(s)?)?;
    Ok(())
}

/// 三层目录铁律 (PRD §8.3)
fn ensure_skeleton(root: &Path) -> Result<()> {
    for sub in ["raw", "output", "wiki"] {
        fs::create_dir_all(root.join(sub))?;
    }
    let claude_md = root.join("CLAUDE.md");
    if !claude_md.exists() {
        fs::write(&claude_md, include_str!("templates/kb_claude.md"))?;
    }
    let index_md = root.join("wiki").join("index.md");
    if !index_md.exists() {
        fs::write(&index_md, include_str!("templates/wiki_index.md"))?;
    }
    Ok(())
}

// ───────────────────────── Scan + Parse ──────────────────

fn scan_all(root: &Path) -> Vec<KbDoc> {
    let mut docs = Vec::new();
    if !root.exists() {
        return docs;
    }
    for entry in WalkDir::new(root).into_iter().flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "md" && ext != "markdown" {
            continue;
        }
        if let Ok(rel) = p.strip_prefix(root) {
            // 对话产物目录 conversations/ 不纳入知识库索引/图谱 (保护板块②不被对话产物污染);
            // 这些文件改由 chat::artifact_search 单独检索。
            if rel
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                == Some("conversations")
            {
                continue;
            }
            if let Some(d) = parse_doc(p, rel) {
                docs.push(d);
            }
        }
    }
    docs
}

static RE_FRONTMATTER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)^---\r?\n(.*?)\r?\n---\r?\n").unwrap());
static RE_TITLE_H1: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^#\s+(.+)$").unwrap());
static RE_WIKILINK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[\[([^\]\|#]+)(?:[#\|][^\]]*)?\]\]").unwrap());
/// 标准 Markdown 链接 [文字](目标) — 用于从 README/目录页派生边
static RE_MDLINK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[[^\]]*\]\(([^)]+)\)").unwrap());
static RE_YAML_KV: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^(\w+)\s*:\s*(.+)$").unwrap());

fn parse_doc(abs_path: &Path, rel: &Path) -> Option<KbDoc> {
    let body = fs::read_to_string(abs_path).ok()?;

    // 提取 frontmatter
    let (fm, body_only) = match RE_FRONTMATTER.captures(&body) {
        Some(c) => (
            c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
            body[c.get(0).unwrap().end()..].to_string(),
        ),
        None => (String::new(), body.clone()),
    };

    // category
    let mut category = String::new();
    let mut fm_title: Option<String> = None;
    for cap in RE_YAML_KV.captures_iter(&fm) {
        let k = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_lowercase();
        let v = cap.get(2).map(|m| m.as_str().trim().trim_matches('"')).unwrap_or("");
        match k.as_str() {
            "category" => category = v.to_string(),
            "title" => fm_title = Some(v.to_string()),
            _ => {}
        }
    }

    // title: frontmatter > # H1 > 文件名
    let title = fm_title
        .or_else(|| {
            RE_TITLE_H1
                .captures(&body_only)
                .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        })
        .unwrap_or_else(|| {
            abs_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string()
        });

    // [[wikilinks]]
    let wikilinks: Vec<String> = RE_WIKILINK
        .captures_iter(&body_only)
        .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        .collect();

    Some(KbDoc {
        rel_path: rel.to_string_lossy().replace('\\', "/"),
        title,
        category,
        wikilinks,
        body: body_only,
    })
}

// ───────────────────────── Tauri commands ────────────────

#[tauri::command]
pub fn kb_root() -> String {
    KB_ROOT.read().to_string_lossy().to_string()
}

#[tauri::command]
pub fn kb_default_root() -> String {
    default_kb_root()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[tauri::command]
pub fn kb_set_root(new_path: String) -> Result<usize, String> {
    let trimmed = new_path.trim().to_string();
    if trimmed.is_empty() {
        return Err("路径不能为空".into());
    }
    let new_root = PathBuf::from(&trimmed);
    ensure_skeleton(&new_root).map_err(|e| format!("无法创建目录骨架: {e}"))?;
    let mut s = load_settings();
    s.kb_root = Some(trimmed);
    save_settings(&s).map_err(|e| format!("写入设置失败: {e}"))?;
    *KB_ROOT.write() = new_root.clone();
    let docs = scan_all(&new_root);
    let n = docs.len();
    *INDEX.write() = docs;
    Ok(n)
}

#[tauri::command]
pub fn kb_scan() -> Result<usize, String> {
    let root = KB_ROOT.read().clone();
    let docs = scan_all(&root);
    let n = docs.len();
    *INDEX.write() = docs;
    Ok(n)
}

#[tauri::command]
pub fn kb_list(subdir: Option<String>) -> Vec<String> {
    let idx = INDEX.read();
    idx.iter()
        .filter(|d| {
            subdir
                .as_deref()
                .map(|s| d.rel_path.starts_with(s))
                .unwrap_or(true)
        })
        .map(|d| d.rel_path.clone())
        .collect()
}

/// Karpathy 式「结构化 wiki + 长上下文 + 双链导航」上下文块, 供 chat 发送前注入。
///
/// 不做关键词召回硬塞 (那是 Karpathy 反对的「平铺 + 向量/关键词召回」范式)。而是把
/// **wiki/ 知识层全文** + **整库的双链/目录地图** + **KB 根的绝对路径** 给模型,
/// 让它用 Read/Glob/Grep 沿双链自取 —— 这才是 headless 下真正可行、且忠于 llmwiki 的
/// 「调用知识库」方式 (claude CLI 在 --print 下有 Read/Glob/Grep, 且 KB 就在 cwd 子树里)。
/// KB 为空 / 不存在时返回空串。
pub fn kb_context_block() -> String {
    let root = KB_ROOT.read().clone();
    if root.as_os_str().is_empty() || !root.exists() {
        return String::new();
    }
    let idx = INDEX.read();
    if idx.is_empty() {
        return String::new();
    }
    let norm = |s: &str| s.replace('\\', "/");
    let stem = |rp: &str| -> String {
        let n = norm(rp);
        let base = n.rsplit('/').next().unwrap_or(&n).to_string();
        base.strip_suffix(".md")
            .or_else(|| base.strip_suffix(".markdown"))
            .unwrap_or(&base)
            .to_string()
    };
    let parent = |rp: &str| -> String {
        let n = norm(rp);
        match n.rfind('/') {
            Some(i) => n[..i].to_string(),
            None => ".".to_string(),
        }
    };

    let root_disp = norm(&root.to_string_lossy());
    let mut out = String::new();
    out.push_str(&format!(
        "### 维基库结构 (Karpathy 式: 结构化 wiki + 长上下文 + 双链导航)\n\n\
知识库根目录: `{root_disp}`\n\
**就在你的工作目录下** —— 你可以(并且应当)用 `Read` / `Glob` / `Grep` 直接打开其中任意页面来取证。\n\
三层目录: `raw/`(只读原始资料, 严禁写入) · `output/`(生成的成品) · `wiki/`(人工确认的知识层)。\n\n"
    ));

    // wiki/ 知识层: 全文注入 (很小, 是导航的起点; 顺着里面的双链继续展开)
    let mut wiki_docs: Vec<&KbDoc> = idx
        .iter()
        .filter(|d| norm(&d.rel_path).starts_with("wiki/"))
        .collect();
    wiki_docs.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    if !wiki_docs.is_empty() {
        out.push_str("#### wiki/ 知识层 (已全文注入, 请顺着其中的双链继续展开)\n\n");
        for d in &wiki_docs {
            out.push_str(&format!(
                "##### [[{}]] · `{}`\n\n{}\n\n",
                stem(&d.rel_path),
                norm(&d.rel_path),
                d.body.trim()
            ));
        }
    }

    // 知识库地图: raw/ output/ 等按文件夹分组, 列标题清单 (供沿双链/路径用 Read/Grep 自取)
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<&KbDoc>> = BTreeMap::new();
    for d in idx.iter() {
        let rp = norm(&d.rel_path);
        if rp == "CLAUDE.md" || rp.starts_with("wiki/") {
            continue; // 行为指南单独注入; wiki 已全文给过
        }
        groups.entry(parent(&rp)).or_default().push(d);
    }
    if !groups.is_empty() {
        out.push_str("#### 知识库地图 (沿双链 `[[名称]]` 或路径, 用 Read / Grep 自取原文)\n\n");
        const MAX_PER_FOLDER: usize = 60;
        for (folder, docs) in &groups {
            out.push_str(&format!("- **{}/** ({} 篇)\n", folder, docs.len()));
            for d in docs.iter().take(MAX_PER_FOLDER) {
                let title = if d.title.trim().is_empty() {
                    stem(&d.rel_path)
                } else {
                    d.title.trim().to_string()
                };
                out.push_str(&format!(
                    "  - [[{}]] — {} · `{}`\n",
                    stem(&d.rel_path),
                    title,
                    norm(&d.rel_path)
                ));
            }
            if docs.len() > MAX_PER_FOLDER {
                out.push_str(&format!(
                    "  - …其余 {} 篇, 用 `Glob \"{}/**\"` 或 `Grep` 关键词列出\n",
                    docs.len() - MAX_PER_FOLDER,
                    folder
                ));
            }
        }
        out.push('\n');
    }

    out.push_str(
        "#### 调用方式 (KB-first, 忠于 Karpathy)\n\
- 回答前先沿上面的结构与双链, 用 Read/Glob/Grep 打开相关页面取证, 不要凭空作答。\n\
- 命中知识库内容时用脚注标源: 正文处 `[^1]`, 文末 `[^1]: [[文件名]]`。\n\
- 双链 `[[…]]` 只写名称 (wiki 根相对名或标题), 不写绝对路径。\n\
- 库里确实查不到时, 用 `💡` 标明这是你的推断/仿写, 不要伪造引文, 也不要谎称检索过。\n\n",
    );
    out
}

#[tauri::command]
pub fn kb_read(rel_path: String) -> Result<String, String> {
    let root = KB_ROOT.read().clone();
    let full = root.join(&rel_path);
    if !full.starts_with(&root) {
        return Err("path escapes KB root".into());
    }
    fs::read_to_string(&full).map_err(|e| e.to_string())
}

/// 删除资料库里的一份资料(浏览页每条右侧 × 用)。
/// 仅允许删除 KB root 子树内的文件; 删除后重扫索引, 返回剩余文件数。
#[tauri::command]
pub fn kb_delete(rel_path: String) -> Result<usize, String> {
    let root = KB_ROOT.read().clone();
    let full = root.join(&rel_path);
    // 防越界: 规范化后必须仍在 KB root 下
    let canon_root = root.canonicalize().unwrap_or_else(|_| root.clone());
    let canon_full = full.canonicalize().map_err(|_| "文件不存在".to_string())?;
    if !canon_full.starts_with(&canon_root) {
        return Err("路径越界, 拒绝删除".into());
    }
    if !canon_full.is_file() {
        return Err("只能删除文件".into());
    }
    fs::remove_file(&canon_full).map_err(|e| e.to_string())?;
    let docs = scan_all(&root);
    let n = docs.len();
    *INDEX.write() = docs;
    Ok(n)
}

/// 清空资料库(管理页「清空资料库」用): 删除 `raw/` 下全部资料并重建空 `raw/`,
/// 保留三层骨架与 CLAUDE.md / wiki。返回清空后剩余索引文件数。
/// 注: 不删 `<root>/.polaris_seeded` marker, 所以重启 **不会** 重新播种默认资料库。
#[tauri::command]
pub fn kb_clear() -> Result<usize, String> {
    let root = KB_ROOT.read().clone();
    let raw = root.join("raw");
    if raw.exists() {
        fs::remove_dir_all(&raw).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&raw).map_err(|e| e.to_string())?;
    let docs = scan_all(&root);
    let n = docs.len();
    *INDEX.write() = docs;
    Ok(n)
}

#[derive(Serialize)]
pub struct KbHit {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
}

/// PRD §8.8 关键词加权评分: 标题 +10 / category +8 / 正文 +1
#[tauri::command]
pub fn kb_search(query: String, top_k: Option<usize>) -> Vec<KbHit> {
    let q = query.to_lowercase();
    let terms: Vec<&str> = q.split_whitespace().collect();
    if terms.is_empty() {
        return vec![];
    }
    let topk = top_k.unwrap_or(8);
    let idx = INDEX.read();
    let mut scored: Vec<(f64, &KbDoc, String)> = idx
        .iter()
        .filter_map(|d| {
            let title_lc = d.title.to_lowercase();
            let cat_lc = d.category.to_lowercase();
            let body_lc = d.body.to_lowercase();
            let mut score = 0.0;
            for t in &terms {
                if title_lc.contains(t) {
                    score += 10.0;
                }
                if !cat_lc.is_empty() && cat_lc.contains(t) {
                    score += 8.0;
                }
                let body_count = body_lc.matches(t).count() as f64;
                score += body_count;
            }
            if score < 1.0 {
                return None;
            }
            // snippet around first term hit
            let snippet = first_snippet(&d.body, &terms, 160);
            Some((score, d, snippet))
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(topk)
        .map(|(score, d, snippet)| KbHit {
            path: d.rel_path.clone(),
            title: d.title.clone(),
            snippet,
            score,
        })
        .collect()
}

fn first_snippet(body: &str, terms: &[&str], max_len: usize) -> String {
    let lower = body.to_lowercase();
    let mut best = 0usize;
    for t in terms {
        if let Some(p) = lower.find(t) {
            best = p;
            break;
        }
    }
    let start = best.saturating_sub(40);
    let end = (start + max_len).min(body.len());
    let raw = &body[clamp_char_boundary(body, start)..clamp_char_boundary(body, end)];
    raw.replace('\n', " ").trim().to_string()
}

fn clamp_char_boundary(s: &str, mut idx: usize) -> usize {
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx.min(s.len())
}

/// Ingest 单文件:任意格式 → 转 markdown 写入 raw/(不可转的原样复制),刷新索引。
#[tauri::command]
pub fn kb_ingest(source_path: String) -> Result<String, String> {
    let root = KB_ROOT.read().clone();
    let rel = ingest_one(&root, &PathBuf::from(&source_path))?;
    let docs = scan_all(&root);
    *INDEX.write() = docs;
    Ok(rel)
}

/// 知识库拖拽上传:批量(可含目录,自动展开)。每个文件转 markdown 入 raw/,
/// 全部处理完只重扫一次索引。返回逐文件结果(失败不影响其余)。
#[tauri::command]
pub fn kb_upload_files(paths: Vec<String>) -> Vec<KbUploadResult> {
    const MAX_FILES: usize = 500;
    let root = KB_ROOT.read().clone();
    let files = expand_to_files(&paths, MAX_FILES);

    let mut results = Vec::with_capacity(files.len());
    for f in &files {
        let name = f
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| f.to_string_lossy().to_string());
        match ingest_one(&root, f) {
            Ok(rel) => results.push(KbUploadResult {
                name,
                rel_path: rel,
                ok: true,
                message: String::new(),
            }),
            Err(e) => results.push(KbUploadResult {
                name,
                rel_path: String::new(),
                ok: false,
                message: e,
            }),
        }
    }

    // 整批结束后重扫一次
    let docs = scan_all(&root);
    *INDEX.write() = docs;

    results
}

#[derive(Serialize)]
pub struct KbUploadResult {
    pub name: String,
    pub rel_path: String,
    pub ok: bool,
    pub message: String,
}

/// 把一个源文件落到 KB 的 raw/:
/// - 可抽文本 → 写 `raw/<stem>.md`
/// - 不可抽(图片/二进制) → 原样复制 `raw/<filename>`
/// 返回写入的相对路径(正斜杠)。
fn ingest_one(root: &Path, src: &Path) -> Result<String, String> {
    if !src.is_file() {
        return Err(format!("不是文件: {}", src.to_string_lossy()));
    }
    let raw_dir = root.join("raw");
    fs::create_dir_all(&raw_dir).map_err(|e| e.to_string())?;

    match convert::convert_to_markdown(src)? {
        Some(md) => {
            let stem = src
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "untitled".into());
            let dst = unique_path(&raw_dir, &stem, "md");
            // 顶部补一个标题,便于 KB 索引与预览
            let titled = format!("# {stem}\n\n{md}");
            fs::write(&dst, titled).map_err(|e| e.to_string())?;
            Ok(rel_of(root, &dst))
        }
        None => {
            let fname = src
                .file_name()
                .ok_or_else(|| "无文件名".to_string())?
                .to_string_lossy()
                .to_string();
            let (stem, ext) = split_name(&fname);
            let dst = unique_path(&raw_dir, &stem, &ext);
            fs::copy(src, &dst).map_err(|e| e.to_string())?;
            Ok(rel_of(root, &dst))
        }
    }
}

/// 展开输入路径:目录递归取文件,文件直接收,去重并限量。
fn expand_to_files(paths: &[String], cap: usize) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for p in paths {
        if out.len() >= cap {
            break;
        }
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            for e in WalkDir::new(&pb).into_iter().flatten() {
                if e.path().is_file() {
                    out.push(e.path().to_path_buf());
                    if out.len() >= cap {
                        break;
                    }
                }
            }
        } else if pb.is_file() {
            out.push(pb);
        }
    }
    out
}

/// 在 dir 下生成不冲突的路径 `<stem>.<ext>`,冲突则追加 ` (2)` ` (3)` …
fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let safe = sanitize_stem(stem);
    let first = dir.join(format!("{safe}.{ext}"));
    if !first.exists() {
        return first;
    }
    for n in 2..10_000 {
        let cand = dir.join(format!("{safe} ({n}).{ext}"));
        if !cand.exists() {
            return cand;
        }
    }
    first
}

/// 去掉文件名里对 Windows 非法的字符
fn sanitize_stem(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    let t = cleaned.trim().trim_matches('.').trim();
    if t.is_empty() {
        "untitled".into()
    } else {
        t.to_string()
    }
}

fn split_name(fname: &str) -> (String, String) {
    match fname.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), e.to_string()),
        _ => (fname.to_string(), "bin".to_string()),
    }
}

fn rel_of(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .unwrap_or(full)
        .to_string_lossy()
        .replace('\\', "/")
}

// ───────────────────────── Graph ─────────────────────────

#[derive(Serialize)]
pub struct KbNode {
    pub id: String,
    pub title: String,
    pub category: String,
    /// 节点类型: "doc" 文档 | "folder" 目录中枢 | "root" 知识库根
    pub kind: String,
}

#[derive(Serialize)]
pub struct KbEdge {
    pub source: String,
    pub target: String,
}

#[derive(Serialize)]
pub struct KbGraph {
    pub nodes: Vec<KbNode>,
    pub edges: Vec<KbEdge>,
}

/// 知识库根中枢节点 id (合成节点, 不对应真实文件)
const ROOT_ID: &str = "__kb_root__";

/// 目录中枢节点 id 前缀。Windows/真实文件名不含冒号, 故不会与 rel_path 冲突。
fn folder_id(rel: &str) -> String {
    format!("dir:{rel}")
}

/// 把 Markdown 链接目标 (可能含 ./ ../) 解析回知识库内的 rel_path。
/// base_dir 为发出链接的文档所在目录 (rel)。返回规范化的正斜杠 rel_path。
fn resolve_rel(base_dir: Option<&Path>, link: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(b) = base_dir {
        for s in b.to_string_lossy().replace('\\', "/").split('/') {
            if !s.is_empty() {
                parts.push(s.to_string());
            }
        }
    }
    for seg in link.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other.to_string()),
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// 知识图谱: 文档节点 + 目录层级派生的中枢结构 + 双链/Markdown 链接关系边。
///
/// 散点根因 (PRD §8 设计回顾): 原实现只认 `[[wikilink]]`, 未链接的文档=孤点。
/// 现按真实目录层级 (raw/X/卷/篇) 自动生成"目录中枢节点"和树状边, 使任意
/// 知识库无需手工双链即可呈现连通图谱; 双链与 Markdown 链接作为额外关系叠加。
#[tauri::command]
pub fn kb_graph() -> KbGraph {
    use std::collections::HashSet;
    let idx = INDEX.read();

    // 标题/文件名 -> rel_path (用于 [[wikilink]] 解析)
    let mut title_to_path: HashMap<String, String> = HashMap::new();
    let mut path_set: HashSet<String> = HashSet::new();
    for d in idx.iter() {
        title_to_path.insert(d.title.to_lowercase(), d.rel_path.clone());
        let stem = Path::new(&d.rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        title_to_path.entry(stem).or_insert_with(|| d.rel_path.clone());
        path_set.insert(d.rel_path.clone());
    }

    let mut nodes: Vec<KbNode> = Vec::new();
    let mut edge_set: HashSet<(String, String)> = HashSet::new();
    let mut folder_set: HashSet<String> = HashSet::new();

    // ① 文档节点
    for d in idx.iter() {
        nodes.push(KbNode {
            id: d.rel_path.clone(),
            title: d.title.clone(),
            category: d.category.clone(),
            kind: "doc".into(),
        });
    }

    // ② 目录层级 -> 中枢节点 + 树状边
    for d in idx.iter() {
        let segs: Vec<&str> = d.rel_path.split('/').filter(|s| !s.is_empty()).collect();
        if segs.len() < 2 {
            // 根目录下的散文件: 直接挂到知识库根
            edge_set.insert((d.rel_path.clone(), ROOT_ID.to_string()));
            continue;
        }
        // 累积每一层文件夹路径 (不含文件名)
        let mut acc = String::new();
        let mut folders: Vec<String> = Vec::new();
        for s in &segs[..segs.len() - 1] {
            if acc.is_empty() {
                acc = (*s).to_string();
            } else {
                acc = format!("{acc}/{s}");
            }
            folders.push(acc.clone());
        }
        // 文档 -> 最深一层目录
        edge_set.insert((d.rel_path.clone(), folder_id(folders.last().unwrap())));
        // 目录 -> 上级目录 逐层
        for w in folders.windows(2) {
            edge_set.insert((folder_id(&w[1]), folder_id(&w[0])));
        }
        // 顶层目录 -> 知识库根
        edge_set.insert((folder_id(&folders[0]), ROOT_ID.to_string()));
        for f in folders {
            folder_set.insert(f);
        }
    }

    // ③ 目录中枢节点
    for f in &folder_set {
        let title = f.rsplit('/').next().unwrap_or(f).to_string();
        nodes.push(KbNode {
            id: folder_id(f),
            title,
            category: String::new(),
            kind: "folder".into(),
        });
    }
    // ④ 知识库根节点 (有内容时)
    if !nodes.is_empty() {
        nodes.push(KbNode {
            id: ROOT_ID.to_string(),
            title: "知识库".into(),
            category: String::new(),
            kind: "root".into(),
        });
    }

    // ⑤ [[wikilink]] 关系边
    for d in idx.iter() {
        for link in &d.wikilinks {
            let key = link.to_lowercase();
            if let Some(target) = title_to_path.get(&key) {
                if target != &d.rel_path {
                    edge_set.insert((d.rel_path.clone(), target.clone()));
                }
            }
        }
    }

    // ⑥ Markdown 链接 [文](relpath.md) 关系边
    for d in idx.iter() {
        let base_dir = Path::new(&d.rel_path).parent();
        for cap in RE_MDLINK.captures_iter(&d.body) {
            let raw = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if raw.is_empty()
                || raw.starts_with("http")
                || raw.starts_with('#')
                || raw.starts_with("mailto:")
            {
                continue;
            }
            let target_raw = raw.split(['#', '?']).next().unwrap_or(raw);
            if !(target_raw.ends_with(".md") || target_raw.ends_with(".markdown")) {
                continue;
            }
            if let Some(t) = resolve_rel(base_dir, target_raw) {
                if t != d.rel_path && path_set.contains(&t) {
                    edge_set.insert((d.rel_path.clone(), t));
                }
            }
        }
    }

    let edges = edge_set
        .into_iter()
        .map(|(source, target)| KbEdge { source, target })
        .collect();

    KbGraph { nodes, edges }
}

/// 用于 chat_send: 把 search hits 渲染成 system prompt KB 块
pub fn render_kb_context(query: &str, top_k: usize) -> String {
    let hits = kb_search(query.to_string(), Some(top_k));
    if hits.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\n## 维基库召回 (KB-first)\n\n");
    out.push_str("以下文件由 Polaris 在你的本地知识库中按关键词加权评分召回,优先以此回答:\n\n");
    let root = KB_ROOT.read().clone();
    for (i, h) in hits.iter().enumerate() {
        let full = root.join(&h.path);
        let body = fs::read_to_string(&full).unwrap_or_default();
        let trimmed: String = body.chars().take(4000).collect();
        out.push_str(&format!(
            "### [{}] {}\n来源: `{}`\n\n{}\n\n---\n\n",
            i + 1,
            h.title,
            h.path,
            trimmed
        ));
    }
    out
}
