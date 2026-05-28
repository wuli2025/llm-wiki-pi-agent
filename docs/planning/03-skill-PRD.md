# 板块 ③ Skill 技能库 · 规划 PRD (v0.2+)

> 状态: **规划中** (MVP v0.1 未实现)
> 上游 PRD: `c:\Users\mi\Desktop\新建文件夹\PRD-v6.html` §9
> 优先级: P1

## 一、板块边界

**做**:
- SKILL.md 单文件解析 (frontmatter + 正文)
- 技能 CRUD (安装 / 升级 / 禁用 / 编辑 / 删除)
- 技能签名校验 (后续接入)
- 技能中心 UI (我的技能 / 市场精选 双 tab)
- `skills::resolve(name)` 给 ① 对话核心拼 prompt

**不做**:
- 执行 skill (那由 ① 在沙箱内做)
- 进程调度 (那是 ④)
- 持久化 (走 ② Wiki 的 `storage::*`)

## 二、SKILL.md 格式 (沿用 v3-v5)

```markdown
---
name: kebab-case-name
description: 触发场景一句话
allowed-tools: Read, Grep, Bash(npm:*)
version: 0.1.0
---

# Skill 正文 (Markdown)

行为指南、示例、Red Flags …
```

## 三、公开 API

```rust
pub fn list(filter: SkillFilter) -> Vec<Skill>;
pub fn install_from_zip(path: &Path) -> Result<String /* skill_id */>;
pub fn install_from_md(path: &Path) -> Result<String>;
pub fn update(id: &str, patch: SkillPatch) -> Result<()>;
pub fn disable(id: &str) -> Result<()>;
pub fn resolve(name: &str) -> Result<PromptText>;  // 给 ① 用
```

## 四、目录布局

```
~/Polaris/skills/
├── my-skills/           # 用户自建
│   └── <skill-id>/
│       ├── SKILL.md
│       └── (assets / scripts)
└── installed/           # 从市场/zip 装的
```

## 五、与其他板块的连通

| 连接点 | 实现 |
|--------|------|
| ↔ ① 对话核心 | `skills::resolve(name) → PromptText` 注入 system prompt 末段;② 输入区「◇ 技能」pill 选择激活 skill |
| ↔ ② Wiki | skill metadata 走 `storage::kv_*`;skill 正文走 `storage::fs_*` |
| ↔ ④ 调度 | 安装 skill 视为低优先级 job (校验 + 复制) |

## 六、里程碑

- v0.2: SKILL.md 解析 + 本地 CRUD + 列表 UI
- v0.3: 接入 ① 对话(skill pill)
- v0.4: 远程市场 / 签名校验
- **v0.5（已实现）**: 外部导入 / 下载，不限来源（见 §七）

## 七、外部导入 / 下载（v0.5，已实现）

> 设计原则：**鼓励从外面拿任意技能，不设来源限制**。导入即激活（前端自动 `enable`），无需额外授权步骤。

### 命令
```rust
// 返回成功导入的 skill id 列表；前端逐个 enable
pub fn import_skill(source: String) -> Result<Vec<String>, String>;
```

### 支持来源（自动识别）
| 来源 | 处理 |
|------|------|
| 本地 `.md` | 解析 frontmatter（无则整篇即正文）→ 规范化写盘 |
| 本地 `.zip` / 目录 | 递归扫描所有 `SKILL.md` / `skill.md` 逐个导入 |
| 远程 `https://….md` | `curl` 下载 → 同本地 .md |
| 远程 `https://….zip` | `curl` 下载 → `tar` 解压 → 同目录 |
| git 仓库 URL（如 `github.com/obra/superpowers`） | `git clone --depth 1` → 扫描整套技能合集逐个装 |

### 实现取舍
- 用系统自带 `git` / `curl` / `tar`（Win11/macOS/Linux 均内置），**不新增 Rust 依赖**。
- 无 frontmatter 的 `SKILL.md` 降级处理：目录名作 id、全文作正文 → 兼容 Anthropic/obra 风格技能。
- 当前只注入 Markdown 正文作为 prompt；技能携带的脚本/资源为后续增强项。
- 前端入口：技能中心右上「导入/下载」按钮（URL/git/路径 输入框 + 本地文件选择）。
