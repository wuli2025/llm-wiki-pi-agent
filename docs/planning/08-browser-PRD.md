# 板块 ⑧ 浏览器插件 · 规划 PRD (v0.1)

> 状态: **MVP 已落地**（默认浏览器 = CloakBrowser，可随时移除）
> 优先级: P1
> 关联: ③ Skill 技能库（本插件以 skill 形态承载）· ① 对话核心（默认注入）

## 一、目标

给 Agent 一个**默认浏览器**，让所有「打开网页 / 抓取 / 填表 / 点击 / 截图 / 网页自动化」走统一、强反检测的通道，而不是各自用裸 `requests` / 原生 `playwright`。

**默认选用 [CloakBrowser](https://github.com/CloakHQ/CloakBrowser)**：源码级（58 处 C++ patch）改造的隐身 Chromium，drop-in 替换 Playwright/Puppeteer，过 Cloudflare Turnstile / reCAPTCHA v3 / FingerprintJS / BrowserScan，MIT、免费、无用量限制。

## 二、板块边界

**做**:
- 以「浏览器插件」抽象承载一个**默认浏览器**（当前实现 = CloakBrowser）
- 默认开启（首启种入），让 Agent 默认就用它
- **可插拔**：随时关闭 / 移除，Agent 回退普通浏览方式
- 离线副本：把插件源码放 `~/Polaris/plugins/<plugin>/`

**不做**:
- 不在 Rust/Tauri 里直接驱动浏览器（由 ① 的 `claude` CLI 经 Bash/Python 驱动）
- 不内置 Chromium 二进制（CloakBrowser 首次运行自动下载 ~200MB 并本地缓存）

## 三、实现形态（务实取舍）

本仓 skill = 注入给 `claude` CLI 的 system prompt 段；CLI 自带 Bash/Python，真正驱动浏览器。所以「浏览器插件」= **一个 preinstalled、默认开启、可关闭的 skill**：

| 件 | 位置 |
|----|------|
| 行为指南（prompt） | `src-tauri/src/templates/skills/cloak-browser.md` |
| 目录登记（preinstalled=true） | `skills.rs` → `catalog()` 的 `cloak-browser` 项 |
| 默认开启（种入一次，用户可关） | `src/stores/skills.ts` → `DEFAULT_ON = ["cloak-browser"]` + `seedDefaults()` |
| 离线源码副本 | `~/Polaris/plugins/cloakbrowser/`（用户给的 zip 解压而来） |

## 四、Agent 用法（drop-in）

```python
from cloakbrowser import launch          # 唯一改动：换 import
browser = launch(humanize=True)          # 拟人化鼠标/键盘/滚动，过行为检测
page = browser.new_page()
page.goto("https://protected-site")
# 抓取 / 点击 / 填表 / page.screenshot(...)
browser.close()
```
- 安装：在线 `pip install cloakbrowser`；离线 `pip install ~/Polaris/plugins/cloakbrowser`
- 代理+地理指纹：`launch(proxy="socks5://…", geoip=True)`
- 持久会话：`launch_persistent_context(...)`

## 五、随时拿掉（可逆性保证）

1. **临时关闭**：技能中心 → 「CloakBrowser 浏览器」开关关掉 → Agent 立即回退普通浏览方式（种入标记已置位，不会被重新打开）。
2. **彻底移除离线副本**：删除 `~/Polaris/plugins/cloakbrowser/`。
3. **从产品里下线**：移除 `catalog()` 里的 `cloak-browser` 项 + `templates/skills/cloak-browser.md` + `skills.ts` 的 `DEFAULT_ON` 条目 —— 无其它板块耦合，删干净即回到无默认浏览器状态。

## 六、后续增强

- 换默认浏览器为可配置项（多浏览器插件并存，settings 选默认）
- 把浏览器驱动收敛为统一脚手架脚本（封装 launch/抓取/截图常用流程）
- 插件签名 / 来源校验（与 ③ 的远程市场对齐）
