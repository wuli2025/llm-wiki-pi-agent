import { defineStore } from "pinia";
import { ref } from "vue";

const STORAGE_KEY = "polaris:enabled-skills";
// 存"已种入过的默认 id 列表"（旧版本可能存的是字符串 "1"）
const SEED_KEY = "polaris:default-skills-seeded";
// 软件自带、默认开启的技能：深度搜索 + 官方 Skill 创建向导 + CloakBrowser 默认浏览器
// （每个 id 只种一次；用户关掉后不会被重新打开）
const DEFAULT_ON = ["deep-research", "skill-creator", "cloak-browser"];

export const useSkillsStore = defineStore("skills", () => {
  const enabledSkills = ref<Set<string>>(new Set());

  function loadFromStorage() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const arr = JSON.parse(raw) as string[];
        enabledSkills.value = new Set(arr);
      }
    } catch {
      enabledSkills.value = new Set();
    }
  }

  /**
   * 种入默认开启的技能。每个 id 只种一次：
   * 新加进 DEFAULT_ON 的默认项会在下次启动补种，但用户手动关掉的不会被重新打开。
   */
  function seedDefaults() {
    let seeded: string[] = [];
    try {
      const raw = localStorage.getItem(SEED_KEY);
      if (raw === "1") {
        seeded = ["cloak-browser"]; // 兼容旧版：当时只种过 cloak-browser
      } else if (raw) {
        seeded = JSON.parse(raw) as string[];
      }
    } catch {
      seeded = [];
    }

    const seededSet = new Set(seeded);
    const toSeed = DEFAULT_ON.filter((id) => !seededSet.has(id));
    if (toSeed.length === 0) return;

    const next = new Set(enabledSkills.value);
    toSeed.forEach((id) => {
      next.add(id);
      seededSet.add(id);
    });
    enabledSkills.value = next;
    saveToStorage();
    localStorage.setItem(SEED_KEY, JSON.stringify(Array.from(seededSet)));
  }

  function saveToStorage() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(Array.from(enabledSkills.value)));
  }

  function toggle(id: string) {
    const next = new Set(enabledSkills.value);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    enabledSkills.value = next;
    saveToStorage();
  }

  /** 显式启用（安装 / 创建后自动激活；幂等，不会重复触发） */
  function enable(id: string) {
    if (enabledSkills.value.has(id)) return;
    const next = new Set(enabledSkills.value);
    next.add(id);
    enabledSkills.value = next;
    saveToStorage();
  }

  function remove(id: string) {
    if (!enabledSkills.value.has(id)) return;
    const next = new Set(enabledSkills.value);
    next.delete(id);
    enabledSkills.value = next;
    saveToStorage();
  }

  function has(id: string): boolean {
    return enabledSkills.value.has(id);
  }

  // 初始化时加载 + 种入默认插件
  loadFromStorage();
  seedDefaults();

  return {
    enabledSkills,
    toggle,
    enable,
    remove,
    has,
  };
});
