//! Skill 系统 — MVP v0.4
//!
//! 统一目录 catalog（编译期内置 + 可安装市场）+ 用户 skill（磁盘持久化，~/Polaris/skills/）
//!
//! - 预装 skill（preinstalled=true）：开箱即用，始终 installed
//! - 市场 skill（preinstalled=false）：列在「市场精选」，点「安装」即复制到用户目录
//! - 用户自建 skill：create_skill 写盘，source = user
//! - 安装 / 创建都会立即出现在技能中心；前端负责安装后自动激活（无需额外授权步骤）

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ═══════════════════════════════════════════════════════════════
// 统一目录 Catalog（编译期，只读）
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CatalogSkill {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str, // official | third-party
    /// true = 预装（始终可用，无需安装），false = 市场技能（需点安装）
    pub preinstalled: bool,
    pub system_prompt: &'static str,
}

fn catalog() -> Vec<CatalogSkill> {
    vec![
        // ── 预装（开箱即用） ──
        CatalogSkill {
            id: "deep-research",
            name: "深度搜索",
            description: "使用 LLM 大规模联网搜索相关内容，自动检索、汇总、交叉验证多来源信息",
            source: "third-party",
            preinstalled: true,
            system_prompt: include_str!("templates/skills/deep-research.md"),
        },
        CatalogSkill {
            id: "skill-creator",
            name: "Skill 创建向导",
            description: "引导用户创建自定义 Skill，自动生成模板和配置文件",
            source: "official",
            preinstalled: true,
            system_prompt: include_str!("templates/skills/skill-creator.md"),
        },
        // ── 市场（点安装即用） ──
        CatalogSkill {
            id: "pdf",
            name: "PDF 文档处理",
            description: "提取 / 生成 / 编辑 PDF：抽取文本表格、合并拆分、Markdown 转 PDF、表单与 OCR",
            source: "official",
            preinstalled: false,
            system_prompt: include_str!("templates/skills/pdf.md"),
        },
        CatalogSkill {
            id: "xlsx",
            name: "Excel 表格",
            description: "读取分析与生成 Excel：透视统计、公式、图表、多 sheet 报表",
            source: "official",
            preinstalled: false,
            system_prompt: include_str!("templates/skills/xlsx.md"),
        },
        CatalogSkill {
            id: "pptx",
            name: "PPT 演示文稿",
            description: "把 PDF / 文档 / 数据转成有高级感的 PPT：母版配色、版式层级、图表，python-pptx 生成",
            source: "official",
            preinstalled: false,
            system_prompt: include_str!("templates/skills/pptx.md"),
        },
        CatalogSkill {
            id: "edge-tts",
            name: "语音合成 Edge-TTS",
            description: "把文本转成自然语音音频，多语言多音色，免费无需 key",
            source: "third-party",
            preinstalled: false,
            system_prompt: include_str!("templates/skills/edge-tts.md"),
        },
        CatalogSkill {
            id: "hyperframes",
            name: "视频动画 Hyperframes",
            description: "用逐帧 / 分镜方式生成短视频与动画，ffmpeg 合成，可配 Edge-TTS 旁白",
            source: "third-party",
            preinstalled: false,
            system_prompt: include_str!("templates/skills/hyperframes.md"),
        },
        CatalogSkill {
            id: "web-search",
            name: "联网搜索",
            description: "实时联网检索，基于 Tavily / Brave 等真实来源回答并交叉验证",
            source: "third-party",
            preinstalled: false,
            system_prompt: include_str!("templates/skills/web-search.md"),
        },
        CatalogSkill {
            id: "image-gen",
            name: "AI 生图 gpt-image-2",
            description: "用 OpenAI gpt-image-2 模型按描述生成图片，自动扩写提示词，支持多候选与改图",
            source: "third-party",
            preinstalled: false,
            system_prompt: include_str!("templates/skills/image-gen.md"),
        },
        // ── 默认浏览器插件（预装、默认开启，可随时移除） ──
        CatalogSkill {
            id: "cloak-browser",
            name: "CloakBrowser 浏览器",
            description: "Agent 默认浏览器：源码级隐身 Chromium，drop-in 替换 Playwright，过 Cloudflare / 反爬。可随时关闭移除",
            source: "third-party",
            preinstalled: true,
            system_prompt: include_str!("templates/skills/cloak-browser.md"),
        },
    ]
}

fn find_catalog(id: &str) -> Option<CatalogSkill> {
    catalog().into_iter().find(|c| c.id == id)
}

// ═══════════════════════════════════════════════════════════════
// 用户 Skills（磁盘持久化）
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct UserSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    /// 来源：用户自建为 "user"；从市场安装时保留原始 source（official / third-party）
    pub source: String,
    pub author: String,
    pub created_at: i64,
    #[serde(skip)]
    pub system_prompt: String,
}

/// 用户 skills 根目录: ~/Polaris/skills/
fn skills_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|u| u.home_dir().join("Polaris").join("skills"))
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 扫描用户 skills 目录，返回所有用户 skill
fn scan_user_skills() -> Vec<UserSkill> {
    let Some(root) = skills_dir() else {
        return vec![];
    };
    let Ok(entries) = fs::read_dir(&root) else {
        return vec![];
    };

    let mut skills = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_file = path.join("skill.md");
        if !skill_file.exists() {
            continue;
        }
        if let Ok(skill) = parse_skill_file(&skill_file) {
            skills.push(skill);
        }
    }
    skills.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    skills
}

/// 解析 skill.md 文件: YAML frontmatter + body
fn parse_skill_file(path: &Path) -> Result<UserSkill, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();

    // 找 frontmatter 边界 ---
    if lines.len() < 3 || lines[0].trim() != "---" {
        return Err("missing frontmatter".into());
    }
    let mut end_idx = 0;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = i;
            break;
        }
    }
    if end_idx == 0 {
        return Err("unclosed frontmatter".into());
    }

    // 解析 frontmatter key: value
    let mut id = String::new();
    let mut name = String::new();
    let mut description = String::new();
    let mut source = "user".to_string();
    let mut author = "user".to_string();
    let mut created_at = 0i64;

    for line in &lines[1..end_idx] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim().trim_matches('"').trim_matches('\'');
            match k {
                "id" => id = v.to_string(),
                "name" => name = v.to_string(),
                "description" => description = v.to_string(),
                "source" => source = v.to_string(),
                "author" => author = v.to_string(),
                "created_at" => created_at = v.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    let system_prompt = lines[end_idx + 1..].join("\n").trim().to_string();

    if id.is_empty() {
        // fallback: 用目录名做 id
        id = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
    }
    if name.is_empty() {
        name = id.clone();
    }

    Ok(UserSkill {
        id,
        name,
        description,
        source,
        author,
        created_at,
        system_prompt,
    })
}

/// 把一份 skill.md 写到用户目录（创建 / 安装共用）
fn write_skill_file(
    id: &str,
    name: &str,
    description: &str,
    source: &str,
    author: &str,
    system_prompt: &str,
) -> Result<(), String> {
    let Some(root) = skills_dir() else {
        return Err("无法获取用户目录".into());
    };
    let dir = root.join(id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let content = format!(
        "---\nid: {}\nname: {}\ndescription: {}\nsource: {}\nauthor: {}\ncreated_at: {}\n---\n\n{}\n",
        id,
        name,
        description,
        source,
        author,
        now_secs(),
        system_prompt
    );

    fs::write(dir.join("skill.md"), content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除用户目录里的 skill 副本（= 卸载 / 删除）
fn remove_user_skill(id: &str) -> Result<(), String> {
    let Some(root) = skills_dir() else {
        return Err("无法获取用户目录".into());
    };
    let dir = root.join(id);
    if !dir.exists() {
        return Err("技能不存在".into());
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// 统一接口（catalog + 用户）
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct SkillMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    /// 是否已拥有可用（预装 / 已安装 / 用户自建）
    pub installed: bool,
    /// 是否可删除（物理存在于用户目录，可卸载 / 删除）
    pub removable: bool,
}

/// 查找 skill（优先用户目录副本，再 catalog），返回元信息 + system_prompt
pub fn find(id: &str) -> Option<(SkillMeta, String)> {
    // 先查用户目录（允许覆盖同名 catalog skill）
    for user in scan_user_skills() {
        if user.id == id {
            return Some((
                SkillMeta {
                    id: user.id,
                    name: user.name,
                    description: user.description,
                    source: user.source,
                    installed: true,
                    removable: true,
                },
                user.system_prompt,
            ));
        }
    }
    // 再查 catalog
    find_catalog(id).map(|c| {
        (
            SkillMeta {
                id: c.id.into(),
                name: c.name.into(),
                description: c.description.into(),
                source: c.source.into(),
                installed: c.preinstalled,
                removable: false,
            },
            c.system_prompt.to_string(),
        )
    })
}

/// 检测用户消息是否包含创建 skill 的意图
pub fn detect_skill_creation_intent(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let triggers = [
        "创建skill",
        "新建skill",
        "写skill",
        "做一个skill",
        "skill创建",
        "skill新建",
        "skill制作",
        "创建技能",
        "新建技能",
        "写技能",
    ];
    triggers.iter().any(|t| lower.contains(t))
}

/// 检测是否是"需要浏览器 / 网页自动化"的任务
pub fn detect_browser_intent(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let triggers = [
        // URL / 英文
        "http://", "https://", "www.", "browser", "scrape", "scraping", "crawl",
        "playwright", "selenium", "puppeteer", "captcha", "cloudflare",
        // 中文
        "网页", "网站", "浏览器", "打开链接", "打开网址", "抓取", "爬取", "爬虫",
        "登录网", "网页截图", "网页自动化", "填表单", "网上下单", "自动化操作网页",
    ];
    triggers.iter().any(|t| lower.contains(t))
}

/// 按任务意图自动激活的 skill（不依赖用户在对话框点选）。可返回多个。
/// 创建技能意图 → skill-creator；网页/浏览器自动化意图 → cloak-browser。
pub fn auto_skills_for_intent(prompt: &str) -> Vec<(SkillMeta, String)> {
    let mut out = Vec::new();
    if detect_skill_creation_intent(prompt) {
        if let Some(s) = find("skill-creator") {
            out.push(s);
        }
    }
    if detect_browser_intent(prompt) {
        if let Some(s) = find("cloak-browser") {
            out.push(s);
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════
// Tauri Commands
// ═══════════════════════════════════════════════════════════════

#[tauri::command]
pub fn list_skills() -> Vec<SkillMeta> {
    let user = scan_user_skills();
    let user_ids: HashSet<String> = user.iter().map(|s| s.id.clone()).collect();

    let cat = catalog();
    let cat_ids: HashSet<&str> = cat.iter().map(|c| c.id).collect();

    let mut list = Vec::new();

    // 1. 目录技能（市场 + 预装）
    for c in &cat {
        let in_user_dir = user_ids.contains(c.id);
        list.push(SkillMeta {
            id: c.id.into(),
            name: c.name.into(),
            description: c.description.into(),
            source: c.source.into(),
            installed: c.preinstalled || in_user_dir,
            removable: in_user_dir,
        });
    }

    // 2. 纯用户自建技能（不在目录里的）
    for u in &user {
        if !cat_ids.contains(u.id.as_str()) {
            list.push(SkillMeta {
                id: u.id.clone(),
                name: u.name.clone(),
                description: u.description.clone(),
                source: u.source.clone(),
                installed: true,
                removable: true,
            });
        }
    }

    list
}

#[tauri::command]
pub fn get_skill(id: String) -> Result<SkillMeta, String> {
    find(&id)
        .map(|(meta, _)| meta)
        .ok_or_else(|| format!("Skill '{}' 不存在", id))
}

#[derive(Debug, Deserialize)]
pub struct CreateSkillArgs {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
}

#[tauri::command]
pub fn create_skill(args: CreateSkillArgs) -> Result<(), String> {
    // 校验 id: 只允许小写字母、数字、-、_
    if !args
        .id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err("Skill ID 只能包含小写字母、数字、-、_".into());
    }
    write_skill_file(
        &args.id,
        &args.name,
        &args.description,
        "user",
        "user",
        &args.system_prompt,
    )
}

/// 从市场安装一个目录技能：复制模板到用户目录，保留原始 source。
/// 安装即拥有，立即出现在技能中心（前端负责自动激活）。
#[tauri::command]
pub fn install_skill(id: String) -> Result<(), String> {
    let c = find_catalog(&id).ok_or_else(|| format!("市场中没有技能 '{}'", id))?;
    write_skill_file(
        c.id,
        c.name,
        c.description,
        c.source,
        "registry",
        c.system_prompt,
    )
}

// ═══════════════════════════════════════════════════════════════
// 外部导入 / 下载（不限来源，鼓励从外面拿）
//   本地：.md 文件 / .zip 压缩包 / 技能目录
//   远程：http(s) 的 .md 或 .zip / git 仓库 URL（可装整套技能合集）
// ═══════════════════════════════════════════════════════════════

/// 把任意来源的 skill 导入用户目录，返回导入成功的 skill id 列表（供前端自动激活）。
#[tauri::command]
pub fn import_skill(source: String) -> Result<Vec<String>, String> {
    let src = source.trim();
    if src.is_empty() {
        return Err("来源为空".into());
    }

    let is_remote = src.starts_with("http://")
        || src.starts_with("https://")
        || src.starts_with("git@")
        || src.ends_with(".git");

    if is_remote {
        import_from_remote(src)
    } else {
        import_from_local(Path::new(src))
    }
}

fn import_from_remote(src: &str) -> Result<Vec<String>, String> {
    let tmp = make_temp_dir()?;
    let lower = src.to_lowercase();

    let result = if lower.ends_with(".md") {
        let md = tmp.join("skill.md");
        download(src, &md)?;
        import_one_md(&md, "imported").map(|id| vec![id])
    } else if lower.ends_with(".zip") {
        let zip = tmp.join("download.zip");
        download(src, &zip)?;
        let out = tmp.join("unzipped");
        fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        unzip(&zip, &out)?;
        import_from_dir(&out)
    } else {
        // .git 结尾、git@、或 github/gitlab 等仓库 URL → clone 后扫描全部技能
        let dest = tmp.join("repo");
        let dest_s = dest.to_string_lossy();
        run_cmd("git", &["clone", "--depth", "1", src, dest_s.as_ref()])?;
        import_from_dir(&dest)
    };

    let _ = fs::remove_dir_all(&tmp);
    result
}

fn import_from_local(path: &Path) -> Result<Vec<String>, String> {
    if !path.exists() {
        return Err(format!("路径不存在: {}", path.display()));
    }
    if path.is_dir() {
        return import_from_dir(path);
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "md" => import_one_md(path, "imported").map(|id| vec![id]),
        "zip" => {
            let tmp = make_temp_dir()?;
            let out = tmp.join("unzipped");
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            unzip(path, &out)?;
            let r = import_from_dir(&out);
            let _ = fs::remove_dir_all(&tmp);
            r
        }
        other => Err(format!("不支持的文件类型: .{}", other)),
    }
}

/// 递归扫描目录里所有 SKILL.md / skill.md，逐个导入（支持技能合集）
fn import_from_dir(dir: &Path) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if fname.eq_ignore_ascii_case("skill.md") {
            if let Ok(id) = import_one_md(p, "imported") {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
    if ids.is_empty() {
        return Err("未在来源中找到任何 SKILL.md / skill.md".into());
    }
    Ok(ids)
}

/// 导入单个 md：有 frontmatter 按字段解析，无 frontmatter 则整篇即正文。
/// 规范化后写到 ~/Polaris/skills/<id>/skill.md。
fn import_one_md(md: &Path, default_source: &str) -> Result<String, String> {
    let raw = fs::read_to_string(md).map_err(|e| e.to_string())?;

    let (id_raw, name_raw, description, src) = if let Ok(s) = parse_skill_file(md) {
        (s.id, s.name, s.description, s.source)
    } else {
        // 无 frontmatter：用所在目录名（退而求其次文件名）当 id，正文 = 全文
        let base = md
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .filter(|s| !["unzipped", "repo", "skills", ""].contains(s))
            .map(|s| s.to_string())
            .or_else(|| md.file_stem().and_then(|n| n.to_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "imported-skill".to_string());
        (base.clone(), base, String::new(), "user".to_string())
    };

    // 正文：parse 成功用其 system_prompt，否则用去掉 frontmatter 的全文
    let body = match parse_skill_file(md) {
        Ok(s) => s.system_prompt,
        Err(_) => raw.trim().to_string(),
    };

    let id = {
        let cleaned: String = id_raw
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let cleaned = cleaned.trim_matches('-').to_string();
        if cleaned.is_empty() {
            "imported-skill".to_string()
        } else {
            cleaned
        }
    };
    let name = if name_raw.trim().is_empty() {
        id.clone()
    } else {
        name_raw
    };
    let source = if src == "user" {
        default_source.to_string()
    } else {
        src
    };

    write_skill_file(&id, &name, &description, &source, "imported", &body)?;
    Ok(id)
}

// ── 外部工具封装（用系统自带 git / curl / tar，免新增 Rust 依赖） ──

fn make_temp_dir() -> Result<PathBuf, String> {
    let base = std::env::temp_dir().join(format!("polaris-skill-import-{}", now_secs()));
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    Ok(base)
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("无法执行 {}：{}（请确认系统已安装 {}）", cmd, e, cmd))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("{} 执行失败：{}", cmd, err.trim()));
    }
    Ok(())
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    let dest_s = dest.to_string_lossy();
    run_cmd("curl", &["-L", "--fail", "-s", "-o", dest_s.as_ref(), url])
}

fn unzip(zip: &Path, dest: &Path) -> Result<(), String> {
    // Win11 / macOS / Linux 自带 bsdtar 可解 .zip
    let zip_s = zip.to_string_lossy();
    let dest_s = dest.to_string_lossy();
    run_cmd("tar", &["-xf", zip_s.as_ref(), "-C", dest_s.as_ref()])
}

#[tauri::command]
pub fn delete_skill(id: String) -> Result<(), String> {
    let Some(root) = skills_dir() else {
        return Err("无法获取用户目录".into());
    };
    // 物理存在于用户目录 → 直接移除（用户自建 / 已安装市场技能都走这里）
    if root.join(&id).exists() {
        return remove_user_skill(&id);
    }
    // 不在用户目录：可能是预装技能（不可删）或根本不存在
    if find_catalog(&id).map(|c| c.preinstalled).unwrap_or(false) {
        return Err("预装技能不可删除".into());
    }
    Err("技能不存在".into())
}
