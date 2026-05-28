---
title: Karpathy 维基方法论
category: 方法
---

# Karpathy 维基方法论

> Andrej Karpathy 提出的「结构化 wiki + 长上下文」优于「平铺文档 + 向量检索」的方法论。

## 一、核心论点

1. **结构化优于扁平**:让 LLM 沿着双链跳页,而不是丢一堆向量召回结果
2. **长上下文优于 RAG**:Claude / GPT 现在能吃下数十万 tokens,把整本 wiki 都丢进去也无妨
3. **人维护少量结构 > LLM 维护大量散文**

## 二、三层目录铁律

参见 [[wiki-index]] 第 0 节。

- `raw/`  只读原始
- `output/` LLM 生成
- `wiki/` 人工确认知识

## 三、与本项目的关系

Polaris 在板块 ② [[wiki-knowledge-base]] 中实现了这套方法论的最小可行版。
