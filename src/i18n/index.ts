import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";
import ja from "./locales/ja.json";
import zh from "./locales/zh.json";
import zhTW from "./locales/zh-TW.json";

type Language = "zh" | "zh-TW" | "en" | "ja";

const DEFAULT_LANGUAGE: Language = "zh";

/**
 * 回落链按「哪份最全」排，而不是一律回落英文。
 *
 * 本产品的文案实际是中文先行：`zh.json` 3600+ 键最全，`en` 是它的严格子集，
 * `ja` 少 500+ 键。原来一律 `fallbackLng: "en"` 的后果是：ja 缺的键去问 en，
 * en 也没有 → i18next 直接把 **key 字符串**渲染到界面上（形如
 * `kanban.governance.title`）。全仓 ~1300 处 `t()` 没写 defaultValue，
 * 这些位置一个兜底都没有。
 *
 * 所以每条链的末端都必须是 `zh` —— 它是唯一保证有值的那份。宁可让日语用户
 * 看到一句中文，也好过看到一个点号分隔的变量名。`coverage.test.ts` 钉死这条。
 *
 * 各语言的缺口由 `pnpm i18n:check` 的 baseline 棘轮盯着往下走；这里只保证
 * 在补齐之前界面不出现裸 key。
 */
export const FALLBACK_CHAINS: Record<string, Language[]> = {
  "zh-TW": ["zh", "en"],
  ja: ["en", "zh"],
  en: ["zh"],
  default: ["zh", "en"],
};

const getInitialLanguage = (): Language => {
  if (typeof window !== "undefined") {
    try {
      const stored = window.localStorage.getItem("language");
      if (
        stored === "zh" ||
        stored === "zh-TW" ||
        stored === "en" ||
        stored === "ja"
      ) {
        return stored;
      }
    } catch (error) {
      console.warn("[i18n] Failed to read stored language preference", error);
    }
  }

  const navigatorLang =
    typeof navigator !== "undefined"
      ? (navigator.language?.toLowerCase() ??
        navigator.languages?.[0]?.toLowerCase())
      : undefined;

  if (navigatorLang === "zh") {
    return "zh";
  }

  if (
    navigatorLang?.startsWith("zh-tw") ||
    navigatorLang?.startsWith("zh-hk") ||
    navigatorLang?.startsWith("zh-mo") ||
    navigatorLang?.startsWith("zh-hant")
  ) {
    return "zh-TW";
  }

  if (navigatorLang?.startsWith("zh")) {
    return "zh";
  }

  if (navigatorLang?.startsWith("ja")) {
    return "ja";
  }

  if (navigatorLang?.startsWith("en")) {
    return "en";
  }

  return DEFAULT_LANGUAGE;
};

const resources = {
  en: {
    translation: en,
  },
  ja: {
    translation: ja,
  },
  zh: {
    translation: zh,
  },
  "zh-TW": {
    translation: zhTW,
  },
};

i18n.use(initReactI18next).init({
  resources,
  lng: getInitialLanguage(), // 根据本地存储或系统语言选择默认语言

  fallbackLng: FALLBACK_CHAINS,

  interpolation: {
    escapeValue: false, // React 已经默认转义
  },

  // 开发模式下显示调试信息
  debug: false,
});

export default i18n;
