# 板块 ⑩ 毛主席资料库与人格 · PRD

> 状态:✅ 已实现(v0.2.x)。本文件记录「默认毛主席资料库 + 毛主席人格项目 + 请教毛主席」一整套需求与落地方案。改动涉及板块 ①(对话核心)②(维基知识库)⑥(CLAUDE.md 主上下文),按板块边界铁律只走各自公开 API。

## 1. 目标

把毛主席资料库做成 Polaris 开箱即用的「默认资料库」与「默认人格」,让任何新用户装好软件就能:

- 翻看毛主席的资料(《毛泽东选集》《毛泽东全集》等);
- 在一个内置的「毛主席」项目里,直接与毛主席人格对话;
- 一键「请教毛主席」,用毛主席的思想方法对任意问题做客观分析,并产出标注来源的 HTML。

## 2. 需求拆解与验收

| # | 需求 | 落地 | 验收 |
|---|------|------|------|
| 1 | 毛主席资料库随安装包一起打包 | `tauri.conf.json` → `bundle.resources` 打包 `resources/seed-kb/**/*`;源为 `PolarisKB/raw/毛主席` 的副本(252 个 .md,约 3.45 MB,压缩进包约 +1.5 MB) | 安装包体积仍约 5 MB 级 |
| 2 | 默认资料库 = 毛主席资料库,首启自动落地 | `kb::seed_default_kb`:首启用一次性 marker `<root>/.polaris_seeded`,把 seed-kb 拷到 `<KB>/raw/毛主席`;之后即便清空/删除也**不重播** | 全新用户启动后「浏览」即有毛主席资料 |
| 3 | 默认赠送每个用户一个「毛主席」项目夹 | `conv::init`:首启用一次性 marker `seeded_mao`,在项目列表**最前**插入「毛主席」项目 | 启动默认进入「毛主席」项目 |
| 4 | 项目内 `CLAUDE.md` 写毛主席人格,该项目对话默认是毛主席在说话 | `src-tauri/src/templates/mao_persona_claude.md` 写到 `~/Polaris/projects/<id>/CLAUDE.md`;`claude_md::render_for_project` 原样注入,无需额外开关 | 在该项目对话,回答即毛主席口吻 |
| 5 | 对话框下加「请教毛主席」按钮 | `ChatPanel` 工具栏新增朱红按钮;`ChatSendArgs.consultMao` 透传到后端 | 按钮可见可点 |
| 6 | 点击后调资料库客观分析,用毛主席分析方法 + 大白话 + 毛选口吻,引用克制,生成标来源 HTML | `chat::mao_consult_directive` 注入指令;资料库召回由 `render_for_project` 预置 | 产出毛选式分析 + 末尾列「来源」的 HTML |
| 7 | 称呼用「同志/小同志」,回答不违反共产主义 | 人格模板 + consult 指令双重约束:称呼同志、立场拥护社会主义/共产主义、不传播违背社会主义核心价值观的内容 | 口吻与立场符合 |
| 8 | 毛主席项目空对话彩蛋 | `ChatPanel` 空状态在该项目显示「小同志,你好。」+ 资料库说明 + 「为建设共产主义事业而奋斗」 | 未对话时中部显示彩蛋三段 |
| 9 | 资料库「浏览」里每个 md 右侧 × 删除 | `kb_delete(relPath)` + `WikiBrowse` 浏览页行内 × | 点 × 删除单份资料,索引即时刷新 |
| 10 | 「管理」里加「清空资料库」键 | `kb_clear()` + `WikiBrowse` 管理页危险卡片 | 清空 `raw/` 全部资料,二次确认 |

## 3. 关键设计

### 3.1 打包与播种(一次性 + 尊重删除)

- **打包**:把 `PolarisKB/raw/毛主席` 复制一份到 `src-tauri/resources/seed-kb/毛主席`,由 Tauri `bundle.resources` 打进安装包。资料全是纯文本 Markdown,压缩率高,体积影响很小。
- **播种**:`kb::init` → `ensure_skeleton` 后调用 `seed_default_kb`。
  - 发布版从 `resource_dir()/resources/seed-kb` 取种子;开发期回退 `CARGO_MANIFEST_DIR/resources/seed-kb`。
  - 一次性 marker `<KB_root>/.polaris_seeded`:存在则跳过。`copy_dir_recursive` 只补不存在的文件(不覆盖用户改动)。
  - **尊重删除**:`kb_clear` / `kb_delete` 都不动 marker,故用户清空/删除后重启**不会**自动恢复默认资料。

### 3.2 毛主席项目与人格

- `State` 增 `seeded_mao` 一次性 marker。`conv::init` 首启确保存在「毛主席」项目(无则插到最前),并把人格模板写到该项目 `CLAUDE.md`(已存在不覆盖)。
- 人格注入复用既有 `claude_md::render_for_project`:项目 `CLAUDE.md` 非占位即注入。无新机制、无侵入。
- 前端按项目名 `"毛主席"` 识别该项目(`MAO_PROJECT_NAME`),用于彩蛋空状态。

### 3.3 请教毛主席

- 一次性动作(非持久开关):按钮以输入框内容为题,带 `consultMao=true` 发送。
- 后端 `mao_consult_directive` 注入:毛选口吻、称呼同志、矛盾分析/实事求是/调查研究/具体问题具体分析/群众路线、引用克制、立场底线(不违反共产主义)、产出标来源的自包含 HTML 到产物目录。
- 资料库召回仍由 `render_for_project` 在发送前预置(默认 KB 即毛主席资料库,召回自然命中)。

## 4. 涉及文件

- 后端:`kb.rs`(播种 + `kb_delete`/`kb_clear`)、`conv.rs`(毛主席项目 + 人格)、`chat.rs`(`consultMao` + consult 指令)、`lib.rs`(注册命令)、`tauri.conf.json`(resources)、`templates/mao_persona_claude.md`(新增)、`resources/seed-kb/毛主席/**`(新增打包源)。
- 前端:`tauri.ts`(`kb.delete`/`kb.clear` + `consultMao`)、`stores/chat.ts`(透传)、`components/ChatPanel.vue`(按钮 + 彩蛋)、`components/WikiBrowse.vue`(× 删除 + 清空)。

## 5. 边界与后续

- 立场约束是**人格设定层面的角色与口吻**,通过 CLAUDE.md/指令实现,非硬过滤。
- 若日后资料库改为含扫描版 PDF/图片,再评估「按需下载/外置目录」,避免安装包随内容膨胀(当前纯文本无需)。
