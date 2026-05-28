# 00-Architecture-PRD — 架构与工程基座

> 记录已落地的架构决策、工程约定和优化策略。代码先行，本文档后补归档。

---

## 1. 板块化架构 (Phase 0–5)

### 1.1 当前状态 (Phase 0–1 已完成)

```
src-tauri/
├─ Cargo.toml              [workspace] — host 仅装配
├─ src/                    host 层：chat / claude_md / conv / kb / lib.rs
└─ crates/
   ├─ polaris-core/        契约 crate：DTO + trait，零依赖
   └─ polaris-sandbox/     板块⑤：命令实现 + Dockerfile 模板
```

### 1.2 依赖规则

```
┌─────────────────────────────────────────────┐
│  polaris-app (host) — 唯一允许依赖所有板块   │
│  负责：启动装配、状态注入、Tauri 命令注册      │
└─────────────────────────────────────────────┘
         │
    ┌────┴────┬──────────┬──────────┐
    ▼         ▼          ▼          ▼
  core    sandbox     (kb)      (conv)
    │         │          │          │
    └─────────┴──────────┴──────────┘
              │
              ▼
        谁都不依赖 core 以外的东西
```

- **polaris-core**：只定义 `struct` / `trait` / `type`，不依赖任何板块
- **polaris-sandbox**：依赖 `polaris-core`，通过 `KbLocator` trait 获取 KB 根路径（不硬调 `kb::kb_root()`）
- **host (polaris-app)**：依赖 `polaris-core` + `polaris-sandbox`，启动时注入 `Arc<dyn KbLocator>`

### 1.3 依赖反转落地方式

```rust
// host (lib.rs) 启动时注入
app.manage(Arc::new(KbLocatorImpl) as Arc<dyn KbLocator>);

// sandbox 命令中取用
let locator: State<Arc<dyn KbLocator>> = app.state();
let kb_root = locator.root();
```

### 1.4 待完成路线图

| Phase | 内容 | 状态 |
|-------|------|------|
| 0 | Cargo workspace 建立 | ✅ |
| 1 | 提取 polaris-core + polaris-sandbox | ✅ |
| 2 | 提取 polaris-kb | ⏳ |
| 3 | 提取 polaris-conv | ⏳ |
| 4 | 提取 polaris-chat | ⏳ |
| 5 | 提取 polaris-claude-md | ⏳ |

> Phase 2–5 建议随功能演进推进，不必专门停下来重构。

---

## 2. 前端目录结构约定

### 2.1 Feature-Sliced 分层

```
src/
├─ features/
│  └─ sandbox/
│     ├─ api.ts              该 feature 的 API + 类型定义
│     └─ components/         该 feature 的组件
│        └─ SandboxStatus.vue
├─ tauri.ts                  全局桥接：只导出核心 invoke 包装
└─ App.vue
```

### 2.2 规则

- `tauri.ts` 只保留跨 feature 的通用桥接代码
- 每个 feature 自包含：API + 组件 + 类型，不 import 其他 feature 内部
- invoke 命令名全局唯一，前后端同步

---

## 3. Rust 编译体积优化

### 3.1 问题

Debug 模式下 `target/debug/` 达 **6.4 GB**，无任何优化配置。

### 3.2 解决方案

#### Cargo.toml [profile.release]

```toml
[profile.release]
opt-level = 3          # 最高优化级别
lto = true             # 链接时优化，合并所有 crate
codegen-units = 1      # 单 codegen unit，最大化优化
panic = "abort"        # panic 直接终止，不展开栈
strip = true           # 自动剥离符号表

[profile.release.package."*"]
opt-level = 3
```

#### 精简依赖

移除零使用依赖：
- `tokio`（原 features = ["full"]，无任何代码引用）
- `chrono`（声明了但无代码 import）
- `thiserror`（声明了但无代码引用）

#### 打包配置 (tauri.conf.json)

```json
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "windows": {
      "webviewInstallMode": { "type": "downloadBootstrapper" },
      "nsis": { "compression": "lzma" }
    }
  }
}
```

- `targets: ["nsis"]`：只打 Windows 安装包（不打包全平台）
- `downloadBootstrapper`：WebView2 用引导器下载（不内嵌，省 ~40MB）
- `lzma`：NSIS 最大压缩算法

### 3.3 成果

| 阶段 | 体积 | 说明 |
|------|------|------|
| Debug target/ | 6,400 MB | 含全部中间产物、PDB、rlib |
| Release exe | 6.39 MB | 剥离符号、LTO 优化后 |
| NSIS 安装包 | **2.09 MB** | LZMA 压缩 + 不内嵌 WebView2 |

> 从 6.4GB → 2.09MB，缩减 **99.97%**。

---

## 4. 开发/构建约定

### 4.1 开发

```bash
npm run tauri:dev      # Debug 模式，编译快，热重载
```

### 4.2 打包分发

```bash
npm run tauri:build    # Release 模式，走 profile.release 优化
```

输出：`src-tauri/target/release/bundle/nsis/Polaris_*.exe`

### 4.3 独立测试

```bash
cargo test -p polaris-sandbox    # 不拉起整个 app，板块独立测试
cargo check --workspace          # 全 workspace 类型检查
```

---

## 5. 已知坑与解法

| 坑 | 场景 | 解法 |
|----|------|------|
| `#[tauri::command]` 不能放 lib.rs 根 | crate 的 lib.rs 直接写命令函数 | 命令放子模块（如 `commands.rs`），lib.rs 只做 `pub mod` + 再导出 |
| Tauri 插件在 workspace crate 中注册 | polaris-sandbox 需要暴露命令给 host | 命令在 crate 内定义，host 的 `lib.rs` 通过 `tauri::generate_handler![]` 注册 |

---

## 6. 网络访问配置

- `vite.config.ts`：`host: "0.0.0.0"`
- 效果：前端服务监听所有网卡，同局域网设备可通过 `http://<局域网IP>:1420` 访问
- Tauri 桌面窗口不受此影响，照常通过 WebView2 渲染
