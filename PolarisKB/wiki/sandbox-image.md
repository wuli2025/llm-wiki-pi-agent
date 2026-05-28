---
title: 安全沙箱镜像
category: 概念
---

# 安全沙箱镜像 (板块 ⑤)

基于 `alpine:3.20` 的轻量 Docker 镜像,目标 < 200MB。

## 装了什么

- Alpine base (~7MB)
- Node 20 LTS (~50MB)
- `@anthropic-ai/claude-code` (~30MB)
- 工具:git, bash, ca-certificates

## 运行时策略

```
--memory=4g --cpus=2
--security-opt=no-new-privileges
-v ~/Polaris:/workspace        (读写)
-v ~/Polaris/PolarisKB:/kb:ro  (只读)
```

详见 [[wiki-knowledge-base]] 与 [[karpathy-wiki方法论]]。
