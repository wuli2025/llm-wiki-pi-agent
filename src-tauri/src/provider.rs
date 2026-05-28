//! 板块 ⑥ API 供应商坞 — pi 内核版供应商切换 + token 用量/成本看板
//!
//! 轻量版把内核从 Claude Code 换成 pi (`@earendil-works/pi-coding-agent`)。
//! pi 不读 `~/.claude/settings.json`, 而是用自己的配置:
//! - `~/.pi/agent/models.json`: 自定义供应商 (baseUrl + apiKey + 模型列表 + 协议 api);
//! - `~/.pi/agent/auth.json`:    内置供应商 (如官方 anthropic) 的 api_key 凭据。
//!
//! 因此「切换供应商」= 把当前供应商写进 pi 的 models.json / auth.json, 并记下 current_id;
//! 真正用哪个模型由 chat.rs 取 `current_model_ref()` 拼成 `--model provider/id` 显式传给 pi。
//!
//! 关键洞察: cc-switch 预设里的端点本质都是「Claude Code 兼容端点」, 即 **Anthropic Messages
//! 协议** —— 这正是它们当年能被 claude 用 `ANTHROPIC_BASE_URL` 直连的原因。所以在 pi 里统一
//! 用 `api: "anthropic-messages"` + 各家 baseUrl + 一个默认模型即可复刻整张供应商表。
//!
//! 用量看板: 不再扫 `~/.claude/projects`, 改为 chat.rs 把每条 assistant 消息的 usage
//! (pi 事件流里已带真实 token + cost) 落账到 `~/Polaris/data/usage.jsonl`, 这里读它聚合
//! 今日/周/月/年 + 14 天趋势。零额外网络、零额外依赖。

use anyhow::Result;
use directories::UserDirs;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const DEFAULT_TOKEN_FIELD: &str = "ANTHROPIC_AUTH_TOKEN";
const API_KEY_FIELD: &str = "ANTHROPIC_API_KEY";
/// pi 里所有 cc-switch 端点统一走的协议 (Anthropic Messages)。
const PI_API: &str = "anthropic-messages";
/// 官方档位在 pi 里对应的内置 provider 名。
const OFFICIAL_PI_PROVIDER: &str = "anthropic";

// ───────────────────────── 预设供应商表 ─────────────────────────
// base_url / token_field / category 取自 cc-switch claudeProviderPresets;
// model 为该供应商的一个合理默认模型 (可在弹窗里改); api 统一 anthropic-messages。
// kind: official(走内置 anthropic) | key(写 models.json) | codex / copilot(轻量版不路由)

struct Preset {
    id: &'static str,
    name: &'static str,
    base_url: &'static str,
    token_field: &'static str,
    category: &'static str,
    kind: &'static str,
    /// pi `--model` 用的默认模型 id (该供应商 API 接受的模型名)。
    model: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset { id: "claude-official", name: "Claude 官方", base_url: "", token_field: DEFAULT_TOKEN_FIELD, category: "official", kind: "official", model: "claude-opus-4-7" },
    Preset { id: "shengsuanyun", name: "胜算云", base_url: "https://router.shengsuanyun.com/api", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "patewayai", name: "PatewayAI", base_url: "https://api.pateway.ai", token_field: API_KEY_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "agentplan", name: "火山方舟 Agentplan", base_url: "https://ark.cn-beijing.volces.com/api/coding", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "doubao-seed-1-6" },
    Preset { id: "byteplus", name: "BytePlus", base_url: "https://ark.ap-southeast.bytepluses.com/api/coding", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "doubao-seed-1-6" },
    Preset { id: "doubaoseed", name: "豆包 Seed", base_url: "https://ark.cn-beijing.volces.com/api/compatible", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "doubao-seed-1-6" },
    Preset { id: "gemini-native", name: "Gemini Native", base_url: "https://generativelanguage.googleapis.com", token_field: API_KEY_FIELD, category: "third_party", kind: "key", model: "gemini-2.5-pro" },
    Preset { id: "deepseek", name: "DeepSeek 深度求索", base_url: "https://api.deepseek.com/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "deepseek-chat" },
    Preset { id: "zhipu-glm", name: "智谱 GLM", base_url: "https://open.bigmodel.cn/api/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "glm-4.6" },
    Preset { id: "zhipu-glm-en", name: "智谱 GLM 国际", base_url: "https://api.z.ai/api/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "glm-4.6" },
    Preset { id: "baidu-qianfan-coding-plan", name: "百度千帆 Coding", base_url: "https://qianfan.baidubce.com/anthropic/coding", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "ernie-4.5-turbo" },
    Preset { id: "bailian", name: "阿里百炼", base_url: "https://dashscope.aliyuncs.com/apps/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "qwen-max" },
    Preset { id: "bailian-for-coding", name: "阿里百炼 Coding", base_url: "https://coding.dashscope.aliyuncs.com/apps/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "qwen3-coder-plus" },
    Preset { id: "kimi", name: "Kimi 月之暗面", base_url: "https://api.moonshot.cn/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "kimi-k2-0905-preview" },
    Preset { id: "kimi-for-coding", name: "Kimi For Coding", base_url: "https://api.kimi.com/coding/", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "kimi-k2-0905-preview" },
    Preset { id: "stepfun", name: "StepFun 阶跃", base_url: "https://api.stepfun.com/step_plan", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "step-2-16k" },
    Preset { id: "stepfun-en", name: "StepFun en", base_url: "https://api.stepfun.ai/step_plan", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "step-2-16k" },
    Preset { id: "modelscope", name: "ModelScope 魔搭", base_url: "https://api-inference.modelscope.cn", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "kat-coder", name: "KAT-Coder", base_url: "https://vanchin.streamlake.ai/api/gateway/v1/endpoints/${ENDPOINT_ID}/claude-code-proxy", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "longcat", name: "LongCat", base_url: "https://api.longcat.chat/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "LongCat-Flash-Chat" },
    Preset { id: "minimax", name: "MiniMax", base_url: "https://api.minimaxi.com/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "MiniMax-M2" },
    Preset { id: "minimax-en", name: "MiniMax en", base_url: "https://api.minimax.io/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "MiniMax-M2" },
    Preset { id: "bailing", name: "百灵 BaiLing", base_url: "https://api.tbox.cn/api/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "aihubmix", name: "AiHubMix", base_url: "https://aihubmix.com", token_field: API_KEY_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "siliconflow", name: "SiliconFlow 硅基流动", base_url: "https://api.siliconflow.cn", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "deepseek-ai/DeepSeek-V3" },
    Preset { id: "siliconflow-en", name: "SiliconFlow en", base_url: "https://api.siliconflow.com", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "deepseek-ai/DeepSeek-V3" },
    Preset { id: "dmxapi", name: "DMXAPI", base_url: "https://www.dmxapi.cn", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "packycode", name: "PackyCode", base_url: "https://www.packyapi.com", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "claudeapi", name: "ClaudeAPI", base_url: "https://gw.claudeapi.com", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "claudecn", name: "ClaudeCN", base_url: "https://claudecn.top", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "runapi", name: "RunAPI", base_url: "https://runapi.co", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "relaxycode", name: "RelaxyCode", base_url: "https://www.relaxycode.com", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "cubence", name: "Cubence", base_url: "https://api.cubence.com", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "aigocode", name: "AIGoCode", base_url: "https://api.aigocode.com", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "rightcode", name: "RightCode", base_url: "https://www.right.codes/claude", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "aicodemirror", name: "AICodeMirror", base_url: "https://api.aicodemirror.com/api/claudecode", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "crazyrouter", name: "CrazyRouter", base_url: "https://cn.crazyrouter.com", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "sssaicode", name: "SSSAiCode", base_url: "https://node-hk.sssaicode.com/api", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "compshare", name: "优云智算", base_url: "https://api.modelverse.cn", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "compshare-coding-plan", name: "优云智算 Coding", base_url: "https://cp.compshare.cn", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "micu", name: "Micu", base_url: "https://www.micuapi.ai", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "ctok-ai", name: "CTok.ai", base_url: "https://api.ctok.ai", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "e-flowcode", name: "E-FlowCode", base_url: "https://e-flowcode.cc", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "openrouter", name: "OpenRouter", base_url: "https://openrouter.ai/api", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "anthropic/claude-sonnet-4.5" },
    Preset { id: "therouter", name: "TheRouter", base_url: "https://api.therouter.ai", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "novita-ai", name: "Novita AI", base_url: "https://api.novita.ai/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "github-copilot", name: "GitHub Copilot", base_url: "https://api.githubcopilot.com", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "copilot", model: "" },
    Preset { id: "codex", name: "Codex (ChatGPT)", base_url: "https://chatgpt.com/backend-api/codex", token_field: DEFAULT_TOKEN_FIELD, category: "third_party", kind: "codex", model: "" },
    Preset { id: "lemondata", name: "LemonData", base_url: "https://api.lemondata.cc", token_field: API_KEY_FIELD, category: "third_party", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "nvidia", name: "Nvidia", base_url: "https://integrate.api.nvidia.com", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "pipellm", name: "PIPELLM", base_url: "https://cc-api.pipellm.ai", token_field: DEFAULT_TOKEN_FIELD, category: "aggregator", kind: "key", model: "claude-sonnet-4-5" },
    Preset { id: "xiaomi-mimo", name: "小米 MiMo", base_url: "https://api.xiaomimimo.com/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "mimo-v2.5-pro" },
    Preset { id: "xiaomi-mimo-token-plan-china", name: "小米 MiMo Token Plan", base_url: "https://token-plan-cn.xiaomimimo.com/anthropic", token_field: DEFAULT_TOKEN_FIELD, category: "cn_official", kind: "key", model: "mimo-v2.5-pro" },
];

fn preset_by_id(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// 分类 → 状态点颜色 (统一色板)。轻量版主色调为墨蓝水墨。
fn color_for(category: &str) -> &'static str {
    match category {
        "official" => "#D97757",
        "cn_official" => "#1f4e79",
        "aggregator" => "#3a6ea5",
        "third_party" => "#5a7a9a",
        "cloud_provider" => "#2c5f8a",
        _ => "#1a3a5c",
    }
}

fn website_from_base(base: &str) -> String {
    let b = base.trim();
    if b.is_empty() {
        return String::new();
    }
    if let Some(rest) = b.strip_prefix("https://").or_else(|| b.strip_prefix("http://")) {
        let host = rest.split('/').next().unwrap_or(rest);
        if host.contains('$') {
            return String::new();
        }
        return format!("https://{host}");
    }
    String::new()
}

// ───────────────────────── 持久化 store ─────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoredProvider {
    id: String,
    name: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    website_url: String,
    #[serde(default)]
    token_field: String,
    /// pi `--model` 用的模型 id (覆盖预设默认值)。
    #[serde(default)]
    model: String,
    #[serde(default)]
    settings_config: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    current_id: String,
    #[serde(default)]
    items: Vec<StoredProvider>,
}

static STORE: Lazy<RwLock<Store>> = Lazy::new(|| RwLock::new(Store::default()));
static STORE_PATH: Lazy<RwLock<PathBuf>> = Lazy::new(|| RwLock::new(PathBuf::new()));

pub fn init(_app: &AppHandle) -> Result<()> {
    let user = UserDirs::new().ok_or_else(|| anyhow::anyhow!("no user dir"))?;
    let dir = user.home_dir().join("Polaris").join("data");
    fs::create_dir_all(&dir)?;
    let path = dir.join("providers.json");
    *STORE_PATH.write() = path.clone();

    let store: Store = if path.exists() {
        let txt = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&txt).unwrap_or_default()
    } else {
        Store::default()
    };

    *STORE.write() = store;
    // 启动时把已配置供应商同步进 pi 的 models.json, 保证 pi 侧配置始终最新。
    let snapshot = STORE.read().clone();
    let _ = rebuild_pi_models_json(&snapshot);
    Ok(())
}

fn persist() {
    let path = STORE_PATH.read().clone();
    if path.as_os_str().is_empty() {
        return;
    }
    if let Ok(txt) = serde_json::to_string_pretty(&*STORE.read()) {
        let _ = fs::write(&path, txt);
    }
}

/// 用 base_url + token 构造最小 settings_config (env 载体, 与前端 AddProviderModal 一致)。
fn default_config(base: &str, token_field: &str, token: &str) -> Value {
    let mut env = Map::new();
    let base = base.trim();
    if !base.is_empty() {
        env.insert("ANTHROPIC_BASE_URL".into(), Value::String(base.into()));
    }
    let token = token.trim();
    if !token.is_empty() {
        let field = if token_field.is_empty() {
            DEFAULT_TOKEN_FIELD
        } else {
            token_field
        };
        env.insert(field.into(), Value::String(token.into()));
    }
    json!({ "env": Value::Object(env) })
}

fn cfg_env_str(cfg: &Value, key: &str) -> String {
    cfg.get("env")
        .and_then(|e| e.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

// ───────────────────────── 视图模型 ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub note: String,
    pub base_url: String,
    pub token_field: String,
    pub category: String,
    pub website_url: String,
    pub color: String,
    pub kind: String,
    pub is_preset: bool,
    pub has_key: bool,
    pub auth_token: String,
    /// pi `--model` 用的模型 id。
    pub model: String,
    /// pi 协议 (anthropic-messages)。
    pub api: String,
    pub settings_config: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderListResult {
    pub providers: Vec<ProviderView>,
    pub current_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub website_url: String,
    #[serde(default)]
    pub token_field: Option<String>,
    /// pi `--model` 用的模型 id。
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub settings_config: Value,
}

fn normalize_url(u: &str) -> String {
    u.trim().trim_end_matches('/').to_string()
}

#[allow(clippy::too_many_arguments)]
fn make_view(
    id: &str,
    name: &str,
    note: &str,
    token_field: &str,
    category: &str,
    kind: &str,
    is_preset: bool,
    preset_base: &str,
    preset_model: &str,
    website: &str,
    model_override: &str,
    cfg: Value,
) -> ProviderView {
    let env_base = cfg_env_str(&cfg, "ANTHROPIC_BASE_URL");
    let base_url = if env_base.is_empty() {
        preset_base.to_string()
    } else {
        env_base
    };
    let token = cfg_env_str(&cfg, token_field);
    let has_key = match kind {
        "official" => true,
        "codex" | "copilot" => false,
        _ => !token.is_empty(),
    };
    let website = if website.is_empty() {
        website_from_base(&base_url)
    } else {
        website.to_string()
    };
    let model = if !model_override.is_empty() {
        model_override.to_string()
    } else {
        preset_model.to_string()
    };
    ProviderView {
        id: id.to_string(),
        name: name.to_string(),
        note: note.to_string(),
        base_url,
        token_field: token_field.to_string(),
        category: category.to_string(),
        website_url: website,
        color: color_for(category).to_string(),
        kind: kind.to_string(),
        is_preset,
        has_key,
        auth_token: token,
        model,
        api: PI_API.to_string(),
        settings_config: cfg,
    }
}

fn build_views(store: &Store) -> Vec<ProviderView> {
    let mut out: Vec<ProviderView> = Vec::with_capacity(PRESETS.len() + store.items.len());

    for p in PRESETS {
        let stored = store.items.iter().find(|i| i.id == p.id);
        let token_field = stored
            .map(|s| s.token_field.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| p.token_field.to_string());
        let cfg = stored
            .map(|s| s.settings_config.clone())
            .unwrap_or_else(|| default_config(p.base_url, &token_field, ""));
        let note = stored.map(|s| s.note.as_str()).unwrap_or("");
        let model_override = stored.map(|s| s.model.as_str()).unwrap_or("");
        out.push(make_view(
            p.id, p.name, note, &token_field, p.category, p.kind, true, p.base_url, p.model, "",
            model_override, cfg,
        ));
    }

    for it in &store.items {
        if preset_by_id(&it.id).is_some() {
            continue; // 预设覆盖已在上方合并
        }
        let token_field = if it.token_field.is_empty() {
            DEFAULT_TOKEN_FIELD.to_string()
        } else {
            it.token_field.clone()
        };
        out.push(make_view(
            &it.id, &it.name, &it.note, &token_field, "custom", "custom", false, "", "",
            &it.website_url, &it.model, it.settings_config.clone(),
        ));
    }

    out
}

/// 当前供应商 = store.current_id (回退 claude-official)。轻量版不再嗅探 live env。
fn detect_current(views: &[ProviderView], store: &Store) -> String {
    let id = if store.current_id.is_empty() {
        "claude-official".to_string()
    } else {
        store.current_id.clone()
    };
    if views.iter().any(|v| v.id == id) {
        id
    } else {
        "claude-official".to_string()
    }
}

// ───────────────────────── pi 配置读写 (~/.pi/agent) ─────────────────────────

/// pi 的 agent 配置目录: 允许 PI_CODING_AGENT_DIR 覆盖, 否则 ~/.pi/agent。
fn pi_agent_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("PI_CODING_AGENT_DIR") {
        if !d.trim().is_empty() {
            return Some(PathBuf::from(d));
        }
    }
    UserDirs::new().map(|u| u.home_dir().join(".pi").join("agent"))
}

fn read_json_file(path: &PathBuf) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| json!({}))
}

/// 把当前 store 里「key 型且已填 token」的供应商全量同步进 pi 的 models.json。
/// 我们只接管自己认识的 id (预设 + 自定义), 其余 provider 原样保留。
fn rebuild_pi_models_json(store: &Store) -> Result<(), String> {
    let dir = pi_agent_dir().ok_or_else(|| "无法定位用户主目录".to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建 ~/.pi/agent 失败: {e}"))?;
    let path = dir.join("models.json");

    let mut root = read_json_file(&path);
    let obj = root.as_object_mut().unwrap();
    let mut providers = obj
        .get("providers")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();

    let views = build_views(store);
    // 我们管理的 id 集合 (key/custom 型) —— 先全部移除, 再按当前状态重写
    let managed: HashSet<String> = views
        .iter()
        .filter(|v| v.kind == "key" || v.kind == "custom")
        .map(|v| v.id.clone())
        .collect();
    providers.retain(|k, _| !managed.contains(k));

    for v in &views {
        if v.kind != "key" && v.kind != "custom" {
            continue; // official 走 auth.json; codex/copilot 不路由
        }
        if v.auth_token.trim().is_empty() || v.base_url.trim().is_empty() {
            continue; // 没 key / 没端点 → 不写, pi 校验会拒
        }
        let model = if v.model.trim().is_empty() {
            "claude-sonnet-4-5"
        } else {
            v.model.trim()
        };
        providers.insert(
            v.id.clone(),
            json!({
                "name": v.name,
                "baseUrl": v.base_url,
                "apiKey": v.auth_token,
                "api": v.api,
                "models": [ { "id": model, "name": v.name, "api": v.api } ]
            }),
        );
    }

    obj.insert("providers".into(), Value::Object(providers));
    let txt = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("序列化 models.json 失败: {e}"))?;
    fs::write(&path, txt).map_err(|e| format!("写 models.json 失败: {e}"))?;
    Ok(())
}

/// 把官方档位的 token 写进 pi auth.json 的 anthropic 凭据 (空则移除, 回落到 env / OAuth)。
fn write_anthropic_auth(token: &str) -> Result<(), String> {
    let dir = pi_agent_dir().ok_or_else(|| "无法定位用户主目录".to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建 ~/.pi/agent 失败: {e}"))?;
    let path = dir.join("auth.json");
    let mut root = read_json_file(&path);
    let obj = root.as_object_mut().unwrap();
    let token = token.trim();
    if token.is_empty() {
        obj.remove(OFFICIAL_PI_PROVIDER);
    } else {
        obj.insert(
            OFFICIAL_PI_PROVIDER.into(),
            json!({ "type": "api_key", "key": token }),
        );
    }
    let txt =
        serde_json::to_string_pretty(&root).map_err(|e| format!("序列化 auth.json 失败: {e}"))?;
    fs::write(&path, txt).map_err(|e| format!("写 auth.json 失败: {e}"))?;
    Ok(())
}

/// 某供应商视图对应的模型 id (空则按 official/key 回落到合理默认)。
fn model_of(v: &ProviderView) -> String {
    if !v.model.trim().is_empty() {
        return v.model.trim().to_string();
    }
    if v.kind == "official" {
        "claude-opus-4-7".to_string()
    } else {
        "claude-sonnet-4-5".to_string()
    }
}

/// chat.rs 取「当前供应商/模型」拼成 pi 的 `--model provider/id`。
/// - official → `anthropic/<model>` (auth 走 auth.json / env / OAuth);
/// - key/custom → `<id>/<model>` (provider 已写进 models.json);
/// - codex/copilot/未配置 → None (chat.rs 不传 --model, 让 pi 用自身默认或报错)。
pub fn current_model_ref() -> Option<String> {
    let store = STORE.read().clone();
    let views = build_views(&store);
    let id = if store.current_id.is_empty() {
        "claude-official".to_string()
    } else {
        store.current_id.clone()
    };
    let v = views.iter().find(|v| v.id == id)?;
    match v.kind.as_str() {
        "official" => Some(format!("{}/{}", OFFICIAL_PI_PROVIDER, model_of(v))),
        "codex" | "copilot" => None,
        _ => {
            if v.auth_token.trim().is_empty() {
                None
            } else {
                Some(format!("{}/{}", v.id, model_of(v)))
            }
        }
    }
}

// ───────────────────────── Commands: 供应商 ─────────────────────────

#[tauri::command]
pub fn provider_list() -> Result<ProviderListResult, String> {
    let store = STORE.read().clone();
    let providers = build_views(&store);
    let current_id = detect_current(&providers, &store);
    Ok(ProviderListResult { providers, current_id })
}

#[tauri::command]
pub fn provider_switch(id: String) -> Result<String, String> {
    let store = STORE.read().clone();
    let views = build_views(&store);
    let v = views
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("供应商不存在: {id}"))?;

    if v.kind == "codex" || v.kind == "copilot" {
        return Err("该供应商说 OpenAI 协议, pi 轻量版未内置其授权路由 (建议改用 Anthropic 兼容端点)".to_string());
    }
    if v.kind != "official" && v.auth_token.trim().is_empty() {
        return Err("该供应商尚未配置 API Key, 请先在弹窗中填写".to_string());
    }

    if v.kind == "official" {
        // 官方: 把 token (若有) 写进 pi auth.json 的 anthropic 凭据
        write_anthropic_auth(&v.auth_token)?;
    } else {
        // key/custom: 同步进 models.json
        rebuild_pi_models_json(&store)?;
    }

    STORE.write().current_id = id.clone();
    persist();
    Ok(id)
}

#[tauri::command]
pub fn provider_save(input: ProviderInput) -> Result<String, String> {
    let token_field = input
        .token_field
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_TOKEN_FIELD.to_string());

    let cfg = if input.settings_config.is_object() {
        input.settings_config.clone()
    } else {
        json!({ "env": {} })
    };

    let id = input
        .id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("custom-{}", now_ms()));

    let item = StoredProvider {
        id: id.clone(),
        name: input.name.trim().to_string(),
        note: input.note.trim().to_string(),
        website_url: normalize_url(&input.website_url),
        token_field,
        model: input.model.clone().unwrap_or_default().trim().to_string(),
        settings_config: cfg,
    };

    {
        let mut store = STORE.write();
        if let Some(existing) = store.items.iter_mut().find(|i| i.id == id) {
            *existing = item;
        } else {
            store.items.push(item);
        }
    }
    persist();
    // 保存后即把最新供应商同步进 pi models.json
    let snapshot = STORE.read().clone();
    let _ = rebuild_pi_models_json(&snapshot);
    Ok(id)
}

#[tauri::command]
pub fn provider_delete(id: String) -> Result<(), String> {
    {
        let mut store = STORE.write();
        store.items.retain(|i| i.id != id);
        if store.current_id == id {
            store.current_id = "claude-official".to_string();
        }
    }
    persist();
    let snapshot = STORE.read().clone();
    let _ = rebuild_pi_models_json(&snapshot);
    Ok(())
}

// ───────────────────────── Commands: Codex 授权 (保留, 轻量版不路由) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus {
    pub installed: bool,
    pub logged_in: bool,
    pub auth_path: String,
}

fn codex_auth_path() -> Option<PathBuf> {
    UserDirs::new().map(|u| u.home_dir().join(".codex").join("auth.json"))
}

#[tauri::command]
pub fn codex_status() -> Result<CodexStatus, String> {
    use std::process::{Command, Stdio};
    let installed = Command::new("codex")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let auth_path = codex_auth_path();
    let logged_in = auth_path.as_ref().map(|p| p.exists()).unwrap_or(false);
    Ok(CodexStatus {
        installed,
        logged_in,
        auth_path: auth_path.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
    })
}

#[tauri::command]
pub fn codex_login() -> Result<(), String> {
    use std::process::{Command, Stdio};
    Command::new("codex")
        .arg("login")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            format!("无法启动 codex login (是否已安装 codex CLI? `npm i -g @openai/codex`): {e}")
        })?;
    Ok(())
}

// ───────────────────────── 用量看板 (读 usage.jsonl) ─────────────────────────

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBucket {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
    pub total: u64,
    pub requests: u64,
    pub cost: f64,
}

impl TokenBucket {
    fn add(&mut self, r: &UsageRecord) {
        self.input += r.input;
        self.output += r.output;
        self.cache_read += r.cache_read;
        self.cache_creation += r.cache_write;
        self.total += r.input + r.output + r.cache_read + r.cache_write;
        self.requests += 1;
        self.cost += r.cost;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub label: String,
    pub total: u64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub available: bool,
    pub today: TokenBucket,
    pub week: TokenBucket,
    pub month: TokenBucket,
    pub year: TokenBucket,
    pub daily: Vec<DailyUsage>,
}

struct UsageRecord {
    ts: u64,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
    cost: f64,
}

static USAGE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn usage_path() -> Option<PathBuf> {
    UserDirs::new().map(|u| {
        u.home_dir()
            .join("Polaris")
            .join("data")
            .join("usage.jsonl")
    })
}

/// chat.rs 把每条 assistant 消息的 token 用量落账到 usage.jsonl (供用量看板)。
pub fn record_usage(
    _provider: &str,
    model: &str,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
    cost: f64,
) {
    if input + output + cache_read + cache_write == 0 {
        return;
    }
    let Some(path) = usage_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = json!({
        "ts": ts,
        "model": model,
        "input": input,
        "output": output,
        "cacheRead": cache_read,
        "cacheWrite": cache_write,
        "cost": cost,
    })
    .to_string();
    let _guard = USAGE_LOCK.lock();
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{}", line);
    }
}

#[tauri::command]
pub fn usage_summary() -> Result<UsageSummary, String> {
    let Some(path) = usage_path() else {
        return Ok(empty_summary());
    };
    if !path.exists() {
        return Ok(empty_summary());
    }

    let today_days = today_utc_days();
    let today_str = ymd_string(today_days);
    let week_cut = ymd_string(today_days - 6);
    let month_cut = ymd_string(today_days - 29);
    let year_cut = ymd_string(today_days - 364);

    let mut trend_window: Vec<(String, String)> = Vec::with_capacity(14);
    for off in (0..14).rev() {
        let d = today_days - off;
        let s = ymd_string(d);
        let label = s.get(5..).unwrap_or(&s).to_string();
        trend_window.push((s, label));
    }
    let trend_set: HashSet<String> = trend_window.iter().map(|(s, _)| s.clone()).collect();
    let mut by_day: HashMap<String, (u64, f64)> = HashMap::new();

    let mut today = TokenBucket::default();
    let mut week = TokenBucket::default();
    let mut month = TokenBucket::default();
    let mut year = TokenBucket::default();

    let Ok(file) = fs::File::open(&path) else {
        return Ok(empty_summary());
    };
    let reader = BufReader::new(file);
    let mut any = false;
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let g = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
        let rec = UsageRecord {
            ts: g("ts"),
            input: g("input"),
            output: g("output"),
            cache_read: g("cacheRead"),
            cache_write: g("cacheWrite"),
            cost: v.get("cost").and_then(|x| x.as_f64()).unwrap_or(0.0),
        };
        let line_tokens = rec.input + rec.output + rec.cache_read + rec.cache_write;
        if line_tokens == 0 {
            continue;
        }
        any = true;
        let date = ymd_string((rec.ts / 86400) as i64);

        if date.as_str() >= year_cut.as_str() {
            year.add(&rec);
            if date.as_str() >= month_cut.as_str() {
                month.add(&rec);
                if date.as_str() >= week_cut.as_str() {
                    week.add(&rec);
                    if date == today_str {
                        today.add(&rec);
                    }
                }
            }
        }
        if trend_set.contains(&date) {
            let e = by_day.entry(date).or_insert((0, 0.0));
            e.0 += line_tokens;
            e.1 += rec.cost;
        }
    }

    let daily: Vec<DailyUsage> = trend_window
        .into_iter()
        .map(|(date, label)| {
            let (total, cost) = by_day.get(&date).copied().unwrap_or((0, 0.0));
            DailyUsage { date, label, total, cost }
        })
        .collect();

    Ok(UsageSummary {
        available: any,
        today,
        week,
        month,
        year,
        daily,
    })
}

fn empty_summary() -> UsageSummary {
    UsageSummary {
        available: false,
        today: TokenBucket::default(),
        week: TokenBucket::default(),
        month: TokenBucket::default(),
        year: TokenBucket::default(),
        daily: Vec::new(),
    }
}

// ───────────────────────── 工具函数 ─────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn today_utc_days() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (secs / 86400) as i64
}

/// 天数 → YYYY-MM-DD (Howard Hinnant civil_from_days, 无外部依赖)
fn ymd_string(z: i64) -> String {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02}", y, m, d)
}
