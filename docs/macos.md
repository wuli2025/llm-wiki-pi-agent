# Polaris Lite · macOS 支持说明

本项目（Tauri + Vue，内核为轻量 **pi** coding-agent）现已兼容 macOS。本文说明 **怎么出 mac 包**、**mac 用户怎么用**、以及 **签名/公证** 这件绕不过的事。

## 一、怎么构建 mac 包

> ⚠️ macOS 的 `.app` / `.dmg` **必须在 macOS 上构建**，Windows 无法交叉编译苹果包。
> 你不需要自己有 Mac —— 发版走 GitHub Actions 的 `macos-latest` 云机器即可。

### 推荐：打 tag 触发 CI（Windows + macOS 一起出）

`.github/workflows/release.yml` 已是 **matrix**，推一个 `v*` 标签即同时构建两端、
合并 `latest.json` 到同一个 Release：

```
git tag v0.2.10
git push origin v0.2.10
```

- Windows runner → NSIS 安装包（`.exe`）
- macOS runner → **universal** 通用二进制（同时覆盖 Intel x86_64 与 Apple Silicon aarch64）的 `.dmg` / `.app`

### 本地（仅当你手上有 Mac）

```bash
# 安装两个架构的 rust target（首次）
rustup target add aarch64-apple-darwin x86_64-apple-darwin
# 出通用包
npm install
npm run tauri build -- --target universal-apple-darwin --bundles app,dmg
```

## 二、mac 用户怎么用

1. 打开 `.dmg`，把 **Polaris Lite** 拖进「应用程序」。
2. 首次启动若被 Gatekeeper 拦（“无法验证开发者”），见下方「签名/公证」。
3. App 内「环境检测与配置」会自动检查轻量内核 **pi**（`@earendil-works/pi-coding-agent`）：
   - 有 `npm` → 一键安装（`npm i -g @earendil-works/pi-coding-agent`）。
   - 没 `npm` → pi 依赖 npm，请先用系统包管理器装 Node.js（如 `brew install node`，npm 随它一起来）再回来一键安装。
   - mac 自带 `sh`/`zsh`，**不需要** PowerShell 7（该项仅 Windows 显示）。

## 三、签名与公证（重要）

当前 CI **默认不强制** Apple 签名（`release.yml` 里的 `APPLE_*` secret 留空即跳过），
所以产出的是 **未签名包**：

- mac 用户首次打开会被 Gatekeeper 拦。绕过方式：
  - 右键点 App → **打开** → 在弹窗里再点「打开」；或
  - 终端执行：`xattr -dr com.apple.quarantine "/Applications/Polaris Lite.app"`
- **自动更新**：未签名时 macOS 的 Tauri updater 自替换可能不稳定，建议 mac 用户暂时手动下载新版 `.dmg`。

### 想消除提示 + 让自动更新稳定生效

需要一个 **Apple Developer 账号**（99 美元/年），然后：

1. 在仓库 Secrets 配 `APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`、
   `APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID`（workflow 已预留这些 env，配上即自动启用）。
2. 在 `src-tauri/tauri.conf.json` 的 `bundle.macOS` 里加 `"signingIdentity"` 等字段。

未配这些之前，未签名包也能用，只是首启需手动放行。

## 四、更新私钥（与 Windows 相同一把）

mac 的自动更新产物用 **同一把** Tauri 更新私钥签名（minisign），即 CI 里的
`TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。这与 macOS 的
苹果代码签名（上一节）是两回事：前者是 Tauri updater 校验更新包完整性用的，两端通用。
