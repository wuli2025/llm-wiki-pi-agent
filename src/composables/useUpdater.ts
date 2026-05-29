// ─────────────────────────────────────────────────────────────
// 自动更新（GitHub Releases 托管）
// 启动时静默检查新版本；发现后在屏幕中央浮出一个「豆包式」轻薄对话框，
// 点「立即更新」会后台下载并安装，随后自动重启生效 —— 即「关掉、过一会
// 再开就是新版」的丝滑体验。
// 「更新」板块（UpdatePanel）可随时手动「检查更新」，并显示当前版本。
// 无网络 / 还没发布 release / 非 Tauri 运行时都会被静默吞掉，不打扰用户。
// ─────────────────────────────────────────────────────────────
import { ref } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";

export const currentVersion = ref<string>(""); // 当前已安装版本
export const updateVersion = ref<string | null>(null); // 可更新到的新版本号（有值=有更新）
export const updateNotes = ref<string>(""); // release 说明（可选）
export const updating = ref(false); // 正在下载 / 安装
export const updateProgress = ref(0); // 0–100
export const updateError = ref("");
export const checking = ref(false); // 正在检查（手动检查的转圈反馈）
export const upToDate = ref(false); // 手动检查的结果：已是最新
export const lastCheckedAt = ref<number | null>(null); // 上次检查时间戳(ms)
// 弹窗被「以后再说」关掉：仅隐藏中央对话框，「更新」板块仍可操作
export const dialogDismissed = ref(false);

let pending: Update | null = null;
let autoChecked = false;

async function ensureCurrentVersion(): Promise<void> {
  if (currentVersion.value) return;
  try {
    currentVersion.value = await getVersion();
  } catch {
    /* 非 Tauri 运行时（纯浏览器预览）拿不到，忽略 */
  }
}

/** 执行一次检查，更新共享状态；返回是否发现新版本。 */
async function runCheck(): Promise<boolean> {
  await ensureCurrentVersion();
  const u = await check();
  lastCheckedAt.value = Date.now();
  if (u) {
    pending = u;
    updateVersion.value = u.version;
    updateNotes.value = u.body ?? "";
    upToDate.value = false;
    return true;
  }
  pending = null;
  updateVersion.value = null;
  upToDate.value = true;
  return false;
}

/** 启动时调用一次：静默检查是否有新版本（出错不打扰）。 */
export async function checkForUpdate(): Promise<void> {
  if (autoChecked) return;
  autoChecked = true;
  await ensureCurrentVersion();
  try {
    await runCheck();
  } catch (e) {
    // 静默：开发态 / 无网 / 未发布 release 时不弹错
    console.warn("[updater] auto check skipped:", e);
  }
}

/** 用户在「更新」板块点「检查更新」：带 UI 反馈（转圈 / 已是最新 / 报错）。 */
export async function manualCheck(): Promise<void> {
  if (checking.value || updating.value) return;
  checking.value = true;
  updateError.value = "";
  upToDate.value = false;
  dialogDismissed.value = false; // 手动检查后允许中央对话框再次出现
  try {
    await runCheck();
  } catch (e: any) {
    updateError.value = e?.message ?? String(e);
  } finally {
    checking.value = false;
  }
}

/** 用户点「立即更新」：下载 + 安装 + 重启。 */
export async function applyUpdate(): Promise<void> {
  if (!pending || updating.value) return;
  updating.value = true;
  updateError.value = "";
  updateProgress.value = 0;
  let total = 0;
  let got = 0;
  try {
    await pending.downloadAndInstall((ev) => {
      // 进度回调：累计已下载字节 / 总字节
      if (ev.event === "Started") {
        total = ev.data.contentLength ?? 0;
      } else if (ev.event === "Progress") {
        got += ev.data.chunkLength;
        updateProgress.value = total ? Math.round((got / total) * 100) : 0;
      } else if (ev.event === "Finished") {
        updateProgress.value = 100;
      }
    });
    // 安装完成 → 自动重启到新版本（Windows 由安装器拉起，Mac/Linux 靠这句）
    await relaunch();
  } catch (e: any) {
    updateError.value = e?.message ?? String(e);
    updating.value = false;
  }
}

/** 「以后再说」：只关中央对话框，本次会话不再自动弹（板块入口仍在）。 */
export function dismissUpdate(): void {
  dialogDismissed.value = true;
}
