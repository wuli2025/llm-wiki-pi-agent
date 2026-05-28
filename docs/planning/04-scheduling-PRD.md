# 板块 ④ 统一调度中心 · 规划 PRD (v0.2+)

> 状态: **规划中** (MVP v0.1 未实现, 仅 chat 模块直接调 docker exec)
> 上游 PRD: `PRD-v6.html` §10
> 优先级: **P0** (v0.2 优先落地)

## 一、板块边界

**v6 最大合并板块**,内部 4 个子模块:
- `api/`     — API provider CRUD + 切换 + Key + 用量
- `tools/`   — 工具注册 + MCP server + 权限策略(四档)
- `pool/`    — 进程池(并发 + 排队 + 心跳 + kill + 优雅退出)
- `cron/`    — 定时任务 + 三入口 + 通知

对外统一命名空间 `schedule::`。

## 二、四档权限模式 (对应主对话页右下按钮)

| 模式 | --permission-mode | 适用 |
|------|------------------|------|
| Manual           | default            | 首次 / 敏感 |
| AutoCurrent      | acceptEdits        | 当前会话 |
| AutoAll          | bypassPermissions  | 熟悉项目 |
| Deny             | plan               | 演示 / 只读 |

## 三、公开 API

```rust
// API 子模块
pub fn api_providers() -> Vec<Provider>;
pub fn api_create(p: NewProvider) -> Result<String>;
pub fn api_activate(id: &str) -> Result<()>;
pub fn api_current() -> Option<Provider>;
pub fn api_usage_record(event: UsageEvent) -> Result<()>;

// 工具子模块
pub fn tools_list() -> Vec<Tool>;
pub fn mcp_list() -> Vec<McpServer>;
pub fn mcp_add(server: NewMcpServer) -> Result<String>;

// 权限策略
pub fn permission_get(scope: PermissionScope) -> PermissionMode;
pub fn permission_set(scope: PermissionScope, mode: PermissionMode) -> Result<()>;
pub fn permission_check(scope: &PermissionScope, call: &ToolCall) -> PermissionDecision;

// 进程池
pub async fn pool_submit(job: Job) -> Result<JobHandle>;
pub async fn pool_cancel(handle: &JobHandle) -> Result<()>;
pub fn pool_stats() -> PoolStats;
pub fn pool_set_concurrency(n: u8) -> Result<()>;  // 4-8

// 定时任务
pub fn cron_create(task: NewScheduledTask) -> Result<String>;
pub fn cron_list(pid: Option<&str>) -> Vec<ScheduledTask>;
pub fn cron_trigger(id: &str, manual: bool) -> Result<()>;
pub fn cron_pause(id: &str) -> Result<()>;
pub fn cron_history(id: &str) -> Vec<RunRecord>;
```

## 四、进程池模型

```
┌─────────────────┐   ┌─────────────────────────────────┐
│   提交队列      │ → │   Worker 池 (默认 4, 可调 4-8)  │
├─────────────────┤   ├─────────────────────────────────┤
│ High: 前台对话  │   │ slot 0: busy 28s                │
│ Normal: 入库    │   │ slot 1: busy 12s                │
│ Low: 定时任务   │   │ slot 2: idle                    │
└─────────────────┘   │ slot 3: idle                    │
                      └─────────────────────────────────┘
```

容器复用: 所有 worker 共享一个长驻沙箱 (`polaris-sandbox`), 通过 `docker exec` 调起 claude CLI。

## 五、调度策略

| 策略 | 说明 |
|------|------|
| 并发上限 | 默认 4, 用户可调 4-8 |
| 优先级 | High(前台) > Normal(用户主动入库) > Low(定时) |
| 排队透明 | 气泡显示「排队中, 前面 X 个」 |
| 心跳检测 | 15s 心跳, 60s 无输出 → SIGKILL + 重试 1 次 |
| 用户取消 | ⏹ 停止 → SIGINT, 5s 后 SIGKILL |

## 六、里程碑

- v0.2: 权限策略落地 (替代 v0.1 直传 cli 参数)
- v0.3: 进程池 + 用量记录
- v0.4: 定时任务 + cron 三入口
- v0.5: MCP server 管理
