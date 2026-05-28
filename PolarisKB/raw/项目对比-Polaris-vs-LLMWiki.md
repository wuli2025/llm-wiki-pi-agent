# 项目对比-Polaris-vs-LLMWiki

<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>项目对比 · LLM Wiki（我的） vs Polaris（他的）</title>
<style>
  :root{
    --ink:#1a2238; --ink2:#2b3a67; --paper:#f7f5ef; --card:#ffffff;
    --blue:#2b5fa6; --blue-soft:#e8f0fb; --gold:#b8860b; --gold-soft:#fbf3df;
    --green:#2e7d52; --green-soft:#e6f4ec; --red:#b3402f; --red-soft:#fbeae6;
    --line:#e3e0d6; --muted:#6b7280; --shadow:0 4px 18px rgba(26,34,56,.08);
  }
  *{box-sizing:border-box}
  body{margin:0;font-family:"Segoe UI","PingFang SC","Microsoft YaHei",system-ui,sans-serif;
    background:linear-gradient(180deg,#f0eee6 0%,#f7f5ef 240px);color:var(--ink);line-height:1.7;}
  .wrap{max-width:1080px;margin:0 auto;padding:0 22px 80px;}
  header.hero{background:linear-gradient(135deg,#1a2238 0%,#2b3a67 55%,#3a5a8a 100%);
    color:#fff;padding:54px 22px 46px;border-bottom:4px solid var(--gold);position:relative;overflow:hidden;}
  header.hero::after{content:"❖";position:absolute;right:10px;top:-20px;font-size:220px;opacity:.06;}
  .hero-inner{max-width:1080px;margin:0 auto;position:relative;z-index:1;}
  .hero h1{margin:0 0 8px;font-size:32px;letter-spacing:.5px;}
  .hero p{margin:6px 0 0;opacity:.9;font-size:15px;}
  .tag{display:inline-block;background:rgba(255,255,255,.14);border:1px solid rgba(255,255,255,.25);
    padding:3px 11px;border-radius:20px;font-size:12.5px;margin:10px 8px 0 0;}
  .meta{margin-top:16px;font-size:13px;opacity:.75;}

  h2{font-size:22px;margin:46px 0 14px;padding-left:12px;border-left:5px solid var(--blue);}
  h2 .num{color:var(--gold);font-weight:800;margin-right:8px;}
  h3{font-size:17px;margin:26px 0 8px;color:var(--ink2);}
  p.lead{color:#444;font-size:15px;}

  .twocol{display:grid;grid-template-columns:1fr 1fr;gap:18px;margin-top:18px;}
  @media(max-width:760px){.twocol{grid-template-columns:1fr;}}
  .card{background:var(--card);border:1px solid var(--line);border-radius:14px;padding:20px 22px;box-shadow:var(--shadow);}
  .card.mine{border-top:4px solid var(--blue);}
  .card.his{border-top:4px solid var(--gold);}
  .card h4{margin:0 0 4px;font-size:18px;}
  .card .sub{color:var(--muted);font-size:13px;margin-bottom:12px;}
  .card ul{margin:8px 0 0;padding-left:18px;}
  .card li{margin:5px 0;font-size:14px;}
  .pill{display:inline-block;font-size:11.5px;padding:2px 9px;border-radius:12px;margin:0 6px 6px 0;}
  .pill.b{background:var(--blue-soft);color:var(--blue);}
  .pill.g{background:var(--gold-soft);color:var(--gold);}

  table{width:100%;border-collapse:collapse;margin-top:16px;background:#fff;border-radius:12px;overflow:hidden;box-shadow:var(--shadow);font-size:14px;}
  th,td{padding:12px 14px;text-align:left;border-bottom:1px solid var(--line);vertical-align:top;}
  th{background:var(--ink);color:#fff;font-weight:600;}
  th.mine{background:var(--blue);} th.his{background:var(--gold);}
  tr:last-child td{border-bottom:none;}
  td.dim{color:var(--muted);font-weight:600;white-space:nowrap;}
  .yes{color:var(--green);font-weight:700;} .no{color:#b0b0b0;} .partial{color:var(--gold);font-weight:600;}

  .learn{background:var(--gold-soft);border:1px solid #ecd9a8;border-left:5px solid var(--gold);
    border-radius:12px;padding:18px 22px;margin:18px 0;box-shadow:var(--shadow);}
  .learn h3{margin-top:0;color:#8a6508;display:flex;align-items:center;gap:8px;}
  .learn .why{margin:10px 0;}
  .learn .why b{color:var(--gold);}
  .badge{font-size:12px;background:var(--gold);color:#fff;border-radius:6px;padding:2px 8px;margin-right:8px;}

  pre{background:#1a2238;color:#e8eefb;border-radius:10px;padding:14px 16px;overflow:auto;font-size:12.5px;line-height:1.6;
    font-family:"Cascadia Code","Consolas",monospace;margin:12px 0;}
  pre .c{color:#7c8db0;} pre .k{color:#ffb86c;} pre .s{color:#a5e8b0;}
  code.inl{background:#eef0f4;border-radius:5px;padding:1px 6px;font-family:"Consolas",monospace;font-size:13px;color:#b3402f;}

  .flow{display:flex;align-items:stretch;gap:10px;flex-wrap:wrap;margin:16px 0;}
  .flow .box{flex:1;min-width:150px;background:#fff;border:1px solid var(--line);border-radius:10px;padding:12px 14px;font-size:13px;text-align:center;box-shadow:var(--shadow);}
  .flow .arrow{align-self:center;color:var(--gold);font-size:22px;font-weight:bold;}
  .flow .box b{display:block;font-size:14px;margin-bottom:3px;}
  .flow .box span{color:var(--muted);font-size:12px;}

  .verdict{background:var(--green-soft);border:1px solid #bfe3cd;border-left:5px solid var(--green);border-radius:12px;padding:18px 22px;margin:16px 0;}
  .verdict h3{margin-top:0;color:var(--green);}
  .reco{counter-reset:r;list-style:none;padding:0;margin:14px 0;}
  .reco li{position:relative;background:#fff;border:1px solid var(--line);border-radius:10px;padding:14px 16px 14px 56px;margin:10px 0;box-shadow:var(--shadow);}
  .reco li::before{counter-increment:r;content:counter(r);position:absolute;left:14px;top:14px;width:28px;height:28px;
    background:var(--blue);color:#fff;border-radius:50%;display:flex;align-items:center;justify-content:center;font-weight:700;font-size:14px;}
  .reco li b{color:var(--ink2);}
  .prio{font-size:11px;padding:2px 8px;border-radius:10px;margin-left:8px;}
  .prio.high{background:var(--red-soft);color:var(--red);} .prio.mid{background:var(--gold-soft);color:var(--gold);}
  footer{text-align:center;color:var(--muted);font-size:12.5px;margin-top:50px;padding-top:20px;border-top:1px solid var(--line);}
  .note{font-size:12.5px;color:var(--muted);margin-top:6px;}
</style>
</head>
<body>

<header class="hero">
  <div class="hero-inner">
    <h1>🧭 项目对比分析报告</h1>
    <p><b>我的项目：</b>LLM Wiki <span style="opacity:.7">（v0.4.13 · React 19 + Tauri）</span> &nbsp;⚔️&nbsp; <b>他的项目：</b>Polaris 北极星 <span style="opacity:.7">（MVP v0.1 · Vue 3 + Tauri）</span></p>
    <div>
      <span class="tag">同源思想：Karpathy LLM Wiki 方法论</span>
      <span class="tag">同栈：Tauri 2 + Rust 后端</span>
      <span class="tag">本地优先知识库</span>
    </div>
    <p class="meta">生成日期：2026-05-25 ｜ 聚焦：他的项目有哪些值得我学习的设计</p>
  </div>
</header>

<div class="wrap">

  <h2><span class="num">00</span>一句话结论</h2>
  <p class="lead">
    两个项目都是「把 Karpathy 的 LLM Wiki 方法论做成桌面应用」，但走了<strong>两条根本不同的路线</strong>：
    我的 LLM Wiki 是<strong>「App 直接调 LLM API」</strong>的成熟产品（功能极其丰富、测试完备）；
    他的 Polaris 是<strong>「App 驱动 Claude Agent CLI + Docker 沙箱」</strong>的精简 MVP。
    他的功能远不如我多，但在<strong>架构理念、安全隔离、工程纪律</strong>三方面有几个非常值得我借鉴的亮点。
  </p>

  <!-- ───────────── 总览对比 ───────────── -->
  <h2><span class="num">01</span>两个项目总览</h2>
  <div class="twocol">
    <div class="card mine">
      <h4>📘 我的：LLM Wiki</h4>
      <div class="sub">v0.4.13 · 成熟开源产品（nashsu）</div>
      <span class="pill b">React 19 + TS</span><span class="pill b">Zustand</span><span class="pill b">shadcn/ui + Tailwind v4</span>
      <span class="pill b">sigma.js 图谱</span><span class="pill b">LanceDB 向量</span><span class="pill b">Milkdown 编辑器</span>
      <ul>
        <li><b>规模庞大</b>：前端 ~200 个 .ts/.tsx，后端 Rust 17 个文件（fs.rs 8 万行级、api_server.rs 4 万行级）</li>
        <li><b>自带 LLM 客户端</b>：直接 fetch 调 OpenAI / Anthropic / Google / Ollama / Custom，自己实现流式</li>
        <li><b>功能全家桶</b>：两步式 Ingest、知识图谱 4 信号关联、Louvain 社区检测、图谱洞察、向量语义检索、Deep Research（Tavily/SerpApi/SearXNG）、Chrome 剪藏插件、本地 HTTP API + Agent Skill、多格式文档（PDF/DOCX/PPTX/XLSX）、级联删除</li>
        <li><b>测试完备</b>：海量 .test.ts、real-llm 集成测试、property 测试、scenario 测试</li>
        <li><b>跨平台 CI/CD</b>：mac/Win/Linux 自动构建；i18n 中英日</li>
      </ul>
    </div>
    <div class="card his">
      <h4>🧭 他的：Polaris 北极星</h4>
      <div class="sub">MVP v0.1 · 精简原型（墨蓝水墨风）</div>
      <span class="pill g">Vue 3 + TS</span><span class="pill g">Pinia</span><span class="pill g">cytoscape 图谱</span>
      <span class="pill g">Docker 沙箱</span><span class="pill g">Claude CLI</span><span class="pill g">PRD 驱动</span>
      <ul>
        <li><b>规模精简</b>：前端 13 个 .vue，后端 Rust 仅 7 个核心文件（最大 16K）</li>
        <li><b>不调 API，而是驱动 Agent</b>：spawn <code class="inl">claude</code> CLI 子进程，解析 <code class="inl">stream-json</code></li>
        <li><b>核心三板块</b>：① 对话核心 ② 维基知识库 ⑤ 安全沙箱，其余 4 板块写了规划 PRD 待实现</li>
        <li><b>Docker 安全沙箱</b>：alpine 轻量镜像跑 Claude CLI，内存/CPU 限制 + 只读挂载 KB</li>
        <li><b>刻意不做向量</b>：信奉 Karpathy「结构化 wiki + 长上下文 > 向量」论点</li>
      </ul>
    </div>
  </div>

  <!-- ───────────── 功能矩阵 ───────────── -->
  <h2><span class="num">02</span>功能/能力对照矩阵</h2>
  <table>
    <tr><th>维度</th><th class="mine">我的 LLM Wiki</th><th class="his">他的 Polaris</th></tr>
    <tr><td class="dim">前端框架</td><td>React 19 + Zustand + shadcn/ui</td><td>Vue 3 + Pinia</td></tr>
    <tr><td class="dim">LLM 接入方式</td><td>App 直接调多家 API（自带流式客户端）</td><td><b>驱动 Claude Agent CLI 子进程</b>（stream-json）</td></tr>
    <tr><td class="dim">安全沙箱</td><td class="no">无（直接读写本机文件）</td><td class="yes">✔ Docker alpine 隔离 + 资源限制</td></tr>
    <tr><td class="dim">Agent 工具能力</td><td class="partial">部分（自己编排 ingest/research 流程）</td><td class="yes">✔ 复用 Claude 全套 Agent 能力（读写文件/执行）</td></tr>
    <tr><td class="dim">向量语义检索</td><td class="yes">✔ LanceDB，召回 58%→71%</td><td class="no">刻意不做（关键词加权评分）</td></tr>
    <tr><td class="dim">知识图谱分析</td><td class="yes">✔ 4 信号关联 + Louvain + 洞察</td><td class="partial">基础双链图（节点+边）</td></tr>
    <tr><td class="dim">多格式文档</td><td class="yes">✔ PDF/DOCX/PPTX/XLSX/图片</td><td class="no">仅 .md/.txt</td></tr>
    <tr><td class="dim">Deep Research / 联网</td><td class="yes">✔ Tavily/SerpApi/SearXNG</td><td class="no">未实现（规划中）</td></tr>
    <tr><td class="dim">浏览器插件</td><td class="yes">✔ Chrome 剪藏</td><td class="no">无</td></tr>
    <tr><td class="dim">CLAUDE.md 主上下文注入</td><td class="no">无此机制</td><td class="yes">✔ 项目级 + KB 级「一体注入」</td></tr>
    <tr><td class="dim">权限分级</td><td class="partial">设置内配置</td><td class="yes">✔ 四档权限直映 CLI 标志</td></tr>
    <tr><td class="dim">PRD / 规划文档</td><td class="partial">超详细 README</td><td class="yes">✔ 模块化 PRD + 边界铁律 + 演进路线</td></tr>
    <tr><td class="dim">自动化测试</td><td class="yes">✔ 极完备（单元/集成/property/scenario）</td><td class="no">基本无</td></tr>
    <tr><td class="dim">成熟度</td><td class="yes">✔ v0.4 产品级、跨平台发布</td><td class="partial">v0.1 MVP 原型</td></tr>
  </table>
  <p class="note">说明：✔ 表示该项明显占优；矩阵意在客观呈现取舍，而非评高低——精简 MVP 的"少"很多是<strong>刻意的设计取舍</strong>。</p>

  <!-- ───────────── 核心架构差异 ───────────── -->
  <h2><span class="num">03</span>最根本的差异：两种 LLM 接入范式</h2>
  <p class="lead">这是两个项目最值得我深思的分水岭，决定了各自的能力边界与安全模型。</p>

  <div class="twocol">
    <div class="card mine">
      <h4>📘 我：调 API（Completion 范式）</h4>
      <div class="flow" style="flex-direction:column;gap:8px;">
        <div class="box"><b>App 编排</b><span>自己写 ingest/research/lint 流程</span></div>
        <div class="arrow" style="transform:rotate(90deg)">➜</div>
        <div class="box"><b>fetch → LLM API</b><span>OpenAI / Anthropic / Ollama…</span></div>
        <div class="arrow" style="transform:rotate(90deg)">➜</div>
        <div class="box"><b>App 解析 + 落盘</b><span>App 拥有全部控制权</span></div>
      </div>
      <ul>
        <li>✅ 控制力强、可换任意模型、可做精细流程</li>
        <li>✅ 不依赖任何外部 CLI 安装</li>
        <li>⚠️ 文件操作由 App 亲自做，<b>无隔离</b></li>
        <li>⚠️ Agent 式"自主读写执行"要自己造</li>
      </ul>
    </div>
    <div class="card his">
      <h4>🧭 他：驱动 Agent CLI（Agent 范式）</h4>
      <div class="flow" style="flex-direction:column;gap:8px;">
        <div class="box"><b>App 拼装 Prompt</b><span>注入 CLAUDE.md + KB 召回</span></div>
        <div class="arrow" style="transform:rotate(90deg)">➜</div>
        <div class="box"><b>spawn claude CLI</b><span>在 Docker 沙箱内 / 宿主机</span></div>
        <div class="arrow" style="transform:rotate(90deg)">➜</div>
        <div class="box"><b>解析 stream-json</b><span>delta / tool_use / result</span></div>
      </div>
      <ul>
        <li>✅ <b>白嫖整套 Agent 能力</b>：工具调用、文件编辑、多步推理</li>
        <li>✅ 配合 Docker，天然<b>安全隔离</b></li>
        <li>✅ App 代码量大幅减少（少造很多轮子）</li>
        <li>⚠️ 强依赖 Claude CLI；换模型不灵活</li>
      </ul>
    </div>
  </div>

  <!-- ───────────── 值得学习的点 ───────────── -->
  <h2><span class="num">04</span>🌟 他的项目值得我学习的 6 个亮点</h2>

  <div class="learn">
    <h3><span class="badge">亮点 1</span>Docker 安全沙箱：让 Agent 在"笼子"里读写文件</h3>
    <p>他用 <code class="inl">std::process::Command</code> 直接包装 docker CLI（零运行时依赖，连 bollard 都不用），把 Claude CLI 关进 alpine 轻量镜像里跑：</p>
    <pre><span class="c"># Dockerfile.sandbox：成品镜像 &lt; 200MB</span>
FROM alpine:3.20
RUN apk add --no-cache nodejs npm git bash ca-certificates
RUN adduser -D -u 1000 -G polaris polaris   <span class="c"># 非 root 用户</span>
RUN npm install -g @anthropic-ai/claude-code

<span class="c"># 启动时的安全约束（docker run 注入）：</span>
docker run -d --name polaris-sandbox \
  <span class="k">--memory=4g --cpus=2</span> \
  <span class="k">--security-opt=no-new-privileges</span> \
  -v ~/Polaris:/workspace \           <span class="c"># 工作区可读写</span>
  -v ~/Polaris/PolarisKB:<span class="k">/kb:ro</span> \      <span class="c"># 知识库只读挂载</span>
  polaris-sandbox:alpine sleep infinity</pre>
    <div class="why"><b>为什么值得学：</b>我的 LLM Wiki 让 App 直接读写用户本机任意文件，<strong>没有任何隔离</strong>。一旦未来引入 Agent 自主操作，风险很大。他这套"<b>资源限制 + 非 root + KB 只读挂载 + no-new-privileges</b>"是教科书级的最小权限沙箱。</div>
    <div class="why"><b>怎么用到我项目：</b>即使我继续走 API 范式，也可以为"让 LLM 自动执行/写文件"的功能加一个<strong>可选的 Docker 沙箱执行后端</strong>，把危险操作关进容器。</div>
  </div>

  <div class="learn">
    <h3><span class="badge">亮点 2</span>CLAUDE.md「一体注入」：把知识库召回直接喂进上下文</h3>
    <p>他在 <code class="inl">claude_md.rs</code> 里做了一个很聪明的设计——发对话前，后端<strong>一次性</strong>把三样东西拼进 prompt：① KB 级 CLAUDE.md 行为指南 ② 基于用户问题的 <code class="inl">kb_search</code> top-3 全文召回 ③ 当前项目的 CLAUDE.md：</p>
    <pre><span class="c">// 关键：不让 LLM 自己去调 kb_search 工具，而是后端预查好嵌进去</span>
block.push_str(<span class="s">"#### 知识库自动召回 (top 3, 已在后端预查, 无需再调任何工具)"</span>);
<span class="c">// + placeholder 标记机制：CLAUDE.md 含 `polaris:placeholder` 行 = 未填写，不注入</span></pre>
    <div class="why"><b>为什么值得学：</b>这是「<b>KB-first</b>」哲学的优雅落地——把检索结果<strong>提前注入</strong>而不是依赖 Agent 多轮工具调用，既省 token/往返，又保证回答一定基于知识库。还用 placeholder marker 优雅区分"未配置"。</div>
    <div class="why"><b>怎么用到我项目：</b>我可以借鉴这套「项目级 + 全局级行为指南文件 + 自动召回预注入」的分层上下文组织方式，让用户能用一个 Markdown 文件定制每个知识库的回答规则。</div>
  </div>

  <div class="learn">
    <h3><span class="badge">亮点 3</span>PRD 驱动开发 + 模块边界铁律：小项目也有大纪律</h3>
    <p>他的 <code class="inl">docs/planning/</code> 里给每个未实现板块都写了独立 PRD，并定下了<strong>板块边界铁律</strong>：</p>
    <pre><span class="c"># 板块边界铁律</span>
1. 跨板块只能调公开 API，不能 import 对方内部 struct/fn
2. 事件优先于直接调用（tauri::Emitter::emit + listen）
3. 每个板块独立测试

<span class="c"># 清晰的演进路线</span>
v0.1 → ①+②+⑤ 跑通核心闭环
v0.2 → ④ 调度中心（权限策略 + 进程池）
v0.3 → ③ Skill 库 + ⑥ 多模态</pre>
    <div class="why"><b>为什么值得学：</b>每个 Rust 文件头部都标注 <code class="inl">//! 设计依据: PRD-v6 §X</code>，代码与需求文档一一对应。即便是 MVP，模块解耦、演进路线、边界纪律都想得很清楚——<strong>"先改 PRD 再动代码"</strong>的纪律性很强。</div>
    <div class="why"><b>怎么用到我项目：</b>我的功能多但靠超长 README 堆叠，可以补一份<strong>模块化的架构/演进文档</strong>，并在核心模块头部加"设计依据"注释，降低后续维护与协作成本。</div>
  </div>

  <div class="learn">
    <h3><span class="badge">亮点 4</span>四档权限直映 CLI 标志：清晰的人机授权 UX</h3>
    <pre><span class="c">// chat.rs：枚举 → claude CLI --permission-mode 值</span>
Manual      → <span class="s">"default"</span>            <span class="c">// 手动授权</span>
AutoCurrent → <span class="s">"acceptEdits"</span>        <span class="c">// 自动批准当前编辑</span>
AutoAll     → <span class="s">"bypassPermissions"</span>  <span class="c">// 全自动</span>
Deny        → <span class="s">"plan"</span>               <span class="c">// 只规划不执行</span></pre>
    <div class="why"><b>为什么值得学：</b>把"AI 能动多少手"抽象成<strong>四个清晰档位</strong>放在对话框上，用户对授权范围一目了然——这是 Agent 类应用很关键的信任体验。</div>
  </div>

  <div class="learn">
    <h3><span class="badge">亮点 5</span>健壮的子进程处理：stderr 透传 + 退出码 + 防死锁取消</h3>
    <pre><span class="c">// chat.rs：双线程分别读 stdout / stderr</span>
- stderr 每一行都 emit 成 error 事件（实时可见）并累积
- stdout 解析 stream-json；非 JSON 行降级当 delta 显示（调试友好）
- 子进程退出后检查 exit code，非 0 时把累积的 stderr 一并报出
- <span class="k">取消时不持锁 wait</span>（先从 HashMap remove 再 kill，避免 chat_cancel 死锁）</pre>
    <div class="why"><b>为什么值得学：</b>调外部进程最容易踩的坑——错误吞掉、卡死、取消死锁——他都处理得很干净。这种<strong>对失败路径的细致处理</strong>是工程成熟度的体现。</div>
  </div>

  <div class="learn">
    <h3><span class="badge">亮点 6</span>克制的"少即是多"：三层目录 + 刻意不做向量</h3>
    <p>他把知识库定为 <code class="inl">raw/ → output/ → wiki/</code> 三层（比我多一个 <b>output 产物层</b>，专放 LLM 生成的文章/报告，如 PolarisKB 里那些 <code class="inl">.html</code> 成品）；并<strong>刻意不做 embedding</strong>，理由直接写在代码注释里：</p>
    <pre><span class="c">//! MVP 缩水:</span>
<span class="c">//! - 不做 Embedding (Karpathy 论点: 结构化 wiki + 长上下文 > 向量)</span>
<span class="c">//! - 索引常驻内存, 进程重启重扫 (后续走 SQLite)</span></pre>
    <div class="why"><b>为什么值得学：</b>每一处"没做"都写明了<strong>取舍理由和后续计划</strong>，而不是单纯缺失。这种"知道自己为什么不做某事"的克制，对我这种功能已经很重的项目是很好的提醒——<strong>不是所有能加的都该加</strong>。</div>
  </div>

  <!-- ───────────── 我的优势 ───────────── -->
  <h2><span class="num">05</span>反过来：我的项目明显更强的地方</h2>
  <div class="verdict">
    <h3>✅ 客观地说，这些方面我领先很多</h3>
    <ul>
      <li><b>功能广度与成熟度</b>：向量检索、知识图谱深度分析（4 信号 + Louvain 社区 + 洞察）、多格式文档、Deep Research 联网、Chrome 剪藏、本地 HTTP API + Agent Skill——他大多还在规划 PRD 里。</li>
      <li><b>模型自由度</b>：自带流式 LLM 客户端，支持 OpenAI/Anthropic/Google/Ollama/Custom 任意切换；他强绑定 Claude CLI。</li>
      <li><b>测试工程</b>：海量单元/集成/property/scenario/real-llm 测试；他基本没有自动化测试。</li>
      <li><b>产品化</b>：跨平台 CI/CD 发布（mac/Win/Linux）、i18n 中英日、级联删除、增量缓存、持久化队列等生产级打磨。</li>
    </ul>
    <p style="margin-bottom:0">一句话：<b>我是"功能完备的成熟产品"，他是"架构清晰的精悍原型"</b>。学他的不是抄功能，而是学他的<strong>架构理念与工程纪律</strong>。</p>
  </div>

  <!-- ───────────── 行动建议 ───────────── -->
  <h2><span class="num">06</span>给我的可落地行动清单</h2>
  <ol class="reco">
    <li><b>引入可选的 Docker 沙箱执行后端</b><span class="prio high">高优先</span><br>
      为"让 LLM 自动写文件 / 执行操作"的能力加一层容器隔离（资源限制 + 非 root + 关键目录只读挂载）。这是我目前最大的安全短板。</li>
    <li><b>支持"驱动 Agent CLI"作为第二种接入范式</b><span class="prio mid">中优先</span><br>
      在现有 API 范式之外，增加可选的 "Claude/Codex CLI" 后端——其实我已有 <code class="inl">claude-cli-transport.ts</code> / <code class="inl">codex-cli-transport.ts</code> 雏形，可参考他的 stream-json 解析与沙箱组合做得更完整。</li>
    <li><b>引入分层 CLAUDE.md 行为指南 + 召回预注入</b><span class="prio mid">中优先</span><br>
      让用户用一个 Markdown 文件定制每个知识库/项目的回答规则，并在发送前把检索结果预注入上下文，减少多轮工具往返。</li>
    <li><b>补一份模块化架构 / 演进 PRD 文档</b><span class="prio mid">中优先</span><br>
      把超长 README 里的设计意图沉淀成模块文档，核心文件头加"设计依据"注释，提升可维护性与协作效率。</li>
    <li><b>对话框增加清晰的"授权档位"UX</b><span class="prio mid">中优先</span><br>
      参考他四档权限的做法，把"AI 能动多少手"做成用户一眼可懂的开关。</li>
    <li><b>增加 output/ 产物层概念</b><span class="prio mid">可选</span><br>
      把"LLM 生成的文章/报告/Lint 结果"与"wiki 知识层"在目录上分开，语义更清晰。</li>
  </ol>

  <footer>
    本报告由对两个项目源码的实际逐文件分析生成（含 sandbox.rs / chat.rs / claude_md.rs / kb.rs / conv.rs / Dockerfile 等核心实现）。<br>
    我的项目：<code class="inl">C:\Users\mi\Desktop\llm_wiki-main</code> ｜ 对比项目：<code class="inl">D:\polaris\polaris-app</code><br>
    生成于 2026-05-25 · 仅供学习借鉴参考
  </footer>

</div>
</body>
</html>
