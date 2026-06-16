import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import {
  readLocalePreference,
  resolveLocale,
  saveLocalePreference,
  t as translate,
  type LocalePreference,
} from "./index";
import type { Locale } from "./types";

type I18nContextValue = {
  locale: Locale;
  preference: LocalePreference;
  setPreference: (pref: LocalePreference) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<LocalePreference>(() => {
    if (typeof window === "undefined") return "system";
    return readLocalePreference();
  });

  const locale = resolveLocale(preference);

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      preference,
      setPreference: (pref) => {
        setPreferenceState(pref);
        saveLocalePreference(pref);
      },
      t: (key, vars) => translate(locale, key, vars),
    }),
    [locale, preference],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}

