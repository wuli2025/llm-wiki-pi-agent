# 板块 ⑦ 设置中心 · 规划 PRD (v0.2)

> 状态: **规划中** (MVP v0.1 仅显示占位文字)
> 上游 PRD: `PRD-v6.html` §13
> 优先级: P2

## 一、板块边界

**做**:
- 设置 UI
- 快捷键管理
- 诊断日志导出
- 关于页

**不做**:
- KV 存储 (走 ② Wiki `storage::kv_*`)
- API Key 管理 (走 ④ 调度 `schedule::api_*`)
- 项目设置 (走 ① 对话 `conv::projects_*`)
- 权限设置 (走 ④ `schedule::permission_*`)

## 二、设置页结构

| 分组 | 内容 |
|------|------|
| 常规 | 语言 / 启动行为 / 开机自启 / 关闭行为 |
| 外观 | 主题 / 字号 / 列表密度 / 侧栏宽度记忆 / 抽屉宽度记忆 |
| 对话 | 默认模型 / 默认 KB 策略 / 自动归档天数 / 回收站保留天数 |
| 工具权限 | 跳转 ④ 调度中心权限页(四档默认 + 高危白名单) |
| 沙箱 | 跳转 ⑤ 沙箱状态页 |
| 语音 / 多模态 | 跳转 ⑥ 多模态工坊;ASR 引擎切换 / 麦克风 / 热词 |
| 快捷键 | 所有 hotkey 配置 (Ctrl+K 搜索 / Ctrl+[ 收侧栏 / Ctrl+] 收抽屉) |
| 诊断 | 导出 zip (日志 + 配置 + 沙箱版本) / 反馈 / 关于 |

## 三、公开 API

```rust
pub fn get(key: &str) -> Option<Value>;
pub fn set(key: &str, val: Value) -> Result<()>;
pub fn export_diagnostic() -> Result<PathBuf>;  // zip 路径
```

## 四、与其他板块的连通

- 所有 K-V 存储委托 `kb::storage::kv_*` (不重复造存储层)
- 工具权限页是 ④ `schedule::permission_*` 的视图
- 沙箱页是 ⑤ `sandbox::sandbox_status` 的视图

## 五、里程碑

- v0.2: 基础设置 (主题切换 / 收侧栏宽度记忆)
- v0.3: 快捷键管理 + 诊断导出
- v0.4: 跳转其他板块的整合视图
