import { allMessages } from "./messages";
import type { Locale } from "./types";

export type LocalePreference = Locale | "system";

export const LOCALE_STORAGE_KEY = "aicontrols-locale-preference";

export function isLocale(v: string): v is Locale {
  return v === "zh" || v === "en";
}

export function detectSystemLocale(): Locale {
  if (typeof navigator === "undefined") return "en";
  const lang = navigator.language.toLowerCase();
  return lang.startsWith("zh") ? "zh" : "en";
}

export function resolveLocale(pref: LocalePreference): Locale {
  if (pref === "system") return detectSystemLocale();
  return pref;
}

export function readLocalePreference(): LocalePreference {
  try {
    const raw = window.localStorage.getItem(LOCALE_STORAGE_KEY);
    if (raw === "system") return "system";
    if (raw && isLocale(raw)) return raw;
    return "system";
  } catch {
    return "system";
  }
}

export function saveLocalePreference(pref: LocalePreference): void {
  try {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, pref);
  } catch {
    // ignore
  }
}

export function t(locale: Locale, key: string, vars?: Record<string, string | number>): string {
  const msg = allMessages[locale][key] ?? allMessages.en[key] ?? key;
  if (!vars) return msg;
  return msg.replace(/\{(\w+)\}/g, (_m, k: string) => String(vars[k] ?? ""));
}

