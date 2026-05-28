# 09 · Coworker 升级 PRD（WorkBuddy 化）

> 本文是一次大型升级的总规划，覆盖 PPT 能力、产物归属、参考资料文件夹、
> 沙箱替换（CubeSandbox / E2B）、对话多开、沙箱工位可视化、完成提醒等。
> 原则：**新模块独立分层，不带崩现有板块**（沿用 §16 板块边界铁律）。

状态图例：⬜ 未开始 · 🔧 进行中 · ✅ 完成 · ⚠️ 有外部约束

---

## 0. 背景与触发

用户在对话里让 Agent 生成 PPT，Agent 需要 `pip install python-pptx PyPDF2` 但被权限/环境挡住，
而**在宿主机直接用 Claude Code CLI 却能成功**。由此引出一系列「让 Polaris 成为真正能干活的
AI 协作伙伴」的诉求。

---

## 1. 需求清单（12 项）

| # | 需求 | 阶段 | 状态 |
|---|------|------|------|
| 1 | 生成 PPT 能力（读 PDF → 高级感 PPT） | P1 | ✅ pptx 技能 |
| 2 | 桌面报告：为什么 CLI 能出 PPT、应用不行 | P1 | ✅ 桌面 HTML |
| 3 | 对话产物优先存到该对话的工作文件夹 | P2 | ✅ `<kb_root>/conversations/<id>/outputs` |
| 4 | 「参考资料」改文件夹：按时间排列本对话产物，点开右栏速览 | P2 | ✅ artifact_list + 文件夹视图 |
| 5 | 历史对话搜索把过往输出文件也算入 | P2 | ✅ artifact_search 接入搜索 |
| 6 | 沙箱替换为 CubeSandbox（替掉 Docker 方案） | P3 | ✅ E2B 后端可选；⚠️ 端点需 Linux/KVM |
| 7 | 对话进程多开（像豆包 / WorkBuddy） | P3 | ✅ 后端并发 + 按会话 id 收尾 + 工位/未读 |
| 8 | 沙箱「工位」可视化（约 9 个）+ 古风数字人 | P4 | ✅ Workstations + DigitalWorker |
| 9 | 任务完成、用户未看 → 右侧对话加墨蓝色提醒点 | P5 | ✅ unreadConvs + 墨蓝点 |
| 10 | 新模块布局规划，避免带崩其他板块 | 贯穿 | ✅ 全程 additive + 双绿校验 |
| 11 | 修改 PRD | 贯穿 | ✅ 本文 + planning 索引 |
| 12 | 规划任务 | P0 | ✅ |

> 多开现状：后端 `chat_send` 每次 spawn 独立进程、按 req_id 流式，天然并发；前端切换对话时
> 解绑前台流（后台进程继续跑），完成事件按 `conversationId` 收尾（结束工位会话 + 未读墨蓝点 +
> 持久化历史）。**遗留增强**：返回仍在运行的后台对话时为「历史快照」而非实时续流（per-conv 实时
> buffer 后续做）。
>
> CubeSandbox 现状：作为 **E2B 兼容后端**可在沙箱页配置端点 URL + Key 并测连通；因其依赖
> Linux+KVM，**需部署在 Linux 主机 / WSL2 / 云**，Windows 宿主无法本地起服务（Docker 后端保留为默认）。

---

## 2. 关键技术判断

### 2.1 为什么「CLI 能出 PPT、应用不行」（需求 2 的结论）
- **宿主 CLI**：直接跑在用户机器上，拥有完整 Python + pip + 交互式授权，可随时 `pip install python-pptx` 再跑脚本。
- **Polaris 应用内**：对话经 `chat_send` 调起 claude，受两点限制——
  1. **环境缺依赖**：沙箱镜像（alpine + claude-code）或宿主环境未预装 `python-pptx`/`pypdf`，且无 Python 工具链。
  2. **非交互授权**：应用内 `pip install` 这类写操作要么被权限档位拦截、要么需要弹窗确认，自动流程中拿不到批准 → 卡住。
- **解决方向**：①把 PPT 生成做成**技能**（注入会显式引导先装依赖再生成）；②产物目录显式授权可写；③沙箱环境预置 Python 工具链（P3 CubeSandbox 镜像里装好）。

### 2.2 CubeSandbox 可行性（需求 6，⚠️ 外部约束）
- CubeSandbox = 腾讯云 **RustVMM + KVM** 微虚机沙箱，**依赖 Linux + KVM 硬件虚拟化**，**Windows 宿主无法原生运行**。
- 但它**原生兼容 E2B SDK**（「只需替换一个 URL 环境变量」）。
- **落地策略**：把 `polaris-sandbox` 从「docker CLI 包装」抽象成 **Backend trait**，新增 **E2B 协议 HTTP 客户端**后端，指向一个 CubeSandbox 端点（远程部署 / WSL2 / 云）。Docker 后端保留为降级。
- 因此「替换 Docker 方案」= **默认后端切到 E2B(CubeSandbox)**，Docker 降为可选 fallback；端点 URL 在设置里配。

---

## 3. 模块分层（需求 10：不带崩现有板块）

**前端（新增，全部 additive）**
```
src/features/coworker/          # 新板块根（与 features/sandbox 平级）
  stores/sessions.ts            # 多开会话状态（活跃/排队/完成未读）
  components/
    Workstations.vue            # 9 工位 + 古风数字人（沙箱视图替换体）
    DigitalWorker.vue           # 单个古风数字人（SVG，idle/busy 动画）
  api.ts                        # 工位/会话相关 invoke 包装
src/components/RightDrawer.vue  # 「参考资料」tab 改文件夹视图（改造，保留原结构）
src/stores/artifacts.ts         # 扩展 list/选中（不破坏 open/preview）
```

**后端（新增模块 + 抽象，不动现有命令签名）**
```
src-tauri/src/coworker.rs       # 多开会话编排 + 完成事件
src-tauri/crates/polaris-sandbox/
  src/backend.rs                # trait SandboxBackend { status/exec/... }
  src/docker.rs                 # 现有逻辑迁入（行为不变）
  src/e2b.rs                    # CubeSandbox(E2B) HTTP 客户端
src-tauri/src/chat.rs           # artifacts_dir → 工作文件夹；产物列表命令
```

**铁律**：跨板块只调公开 API / 事件；新命令一律新增，不改旧命令语义；每步 `cargo build` + `vue-tsc` 双绿后才提交。

---

## 4. 分阶段任务（可独立交付，逐阶段提交）

### P1 · PPT 能力 + 报告（独立 / 低风险）✅ 可立即做
- [ ] `templates/skills/pptx.md` 技能模板（python-pptx 高级感 PPT；读 PDF 用 pypdf；先装依赖再生成）
- [ ] skills.rs catalog 注册 `pptx`；tauri.ts browserStub 同步
- [ ] 桌面报告 `为什么CLI能出PPT而应用不行.html`（含解决方案）

### P2 · 产物归属工作文件夹 + 参考资料文件夹（中等）
- [ ] `artifacts_dir` 改为「工作文件夹 / conversations/<id>/outputs」（工作文件夹 = KB root；可配置）
- [ ] 新命令 `artifact_list(convId)` → 文件列表（mtime 倒序 + kind/size/time）
- [ ] RightDrawer「参考资料」→ 文件夹视图（时间倒序、文件风格、点击右栏预览）
- [ ] 搜索纳入产物文件（`kb_search` / conv 搜索扩展扫描 outputs）

### P3 · 沙箱替换 CubeSandbox(E2B) + 多开（高难 / ⚠️ 外部）
- [ ] polaris-sandbox 引入 `SandboxBackend` trait；现有 docker 迁入 `docker.rs`
- [ ] `e2b.rs`：E2B 协议客户端（create/exec/upload/download/kill）
- [ ] 设置项：沙箱后端选择 + CubeSandbox endpoint URL
- [ ] `coworker.rs`：每对话独立 session，支持并发多开（上限可配）
- [ ] CubeSandbox 镜像预置 Python + python-pptx + pypdf（闭合 P1 的环境缺口）

### P4 · 沙箱工位可视化 + 古风数字人（前端 / 自包含）
- [ ] `Workstations.vue`：9 工位网格（仿腾讯数字员工办公室）
- [ ] `DigitalWorker.vue`：古风数字小人 SVG，idle=摸鱼动画 / busy=工作动画
- [ ] 绑定 sessions：有会话加载→对应工位数字人进入工作态

### P5 · 完成提醒墨蓝点（前端 / 低风险）
- [ ] 会话 done 且未查看 → 侧栏对话项加墨蓝色未读点；点开后清除

### 贯穿 · PRD / 规划 / 提交
- [ ] 每阶段更新本文状态 + `docs/planning/README.md`
- [ ] 每阶段 build 双绿后 commit + push

---

## 5. 验收

效果对齐用户原话「直到我要的效果完成」：
PPT 能生成 · 产物落对话工作文件夹并在参考资料按时间可速览 · 搜索含历史产物 ·
沙箱走 CubeSandbox(E2B) 且对话可多开 · 工位有古风数字人随任务摸鱼/工作 ·
完成未读有墨蓝点提醒。
