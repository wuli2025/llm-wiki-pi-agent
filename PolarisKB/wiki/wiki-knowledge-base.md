---
title: 维基知识库板块
category: 概念
---

# 维基知识库 (板块 ②)

Polaris MVP v0.1 的 KB 模块,实现了:

- 文件扫描 (`kb_scan`)
- 关键词加权评分搜索 (`kb_search`):标题 +10 / category +8 / 正文 +1
- 双链 `[[wiki-link]]` 解析 → 图谱节点 + 边
- 简易 ingest:复制外部 .md 到 `raw/`

详细方法论见 [[karpathy-wiki方法论]]。

## 与对话核心的连通

板块 ① 在 `chat_send` 时,如果勾选「注入 KB」,
会先调 `kb::render_kb_context(query, 3)` 把 Top-3 命中文件拼到 prompt 顶部。
