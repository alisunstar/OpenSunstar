const SETTINGS_NAV_KEY = "opensunstar.settingsNav";

export interface SettingsNavIntent {
  tab?: "general" | "advanced" | "about";
  /** ProxyTabContent accordion values, e.g. `proxy`, `failover` */
  openSections?: string[];
}

export function setSettingsNavIntent(intent: SettingsNavIntent): void {
  sessionStorage.setItem(SETTINGS_NAV_KEY, JSON.stringify(intent));
}

export function consumeSettingsNavIntent(): SettingsNavIntent | null {
  const raw = sessionStorage.getItem(SETTINGS_NAV_KEY);
  if (!raw) return null;
  sessionStorage.removeItem(SETTINGS_NAV_KEY);
  try {
    return JSON.parse(raw) as SettingsNavIntent;
  } catch {
    return null;
  }
}

export function buildProxySettingsIntent(): SettingsNavIntent {
  return {
    tab: "advanced",
    openSections: ["proxy", "failover"],
  };
}

export function buildAiProviderSettingsIntent(): SettingsNavIntent {
  return {
    tab: "advanced",
    openSections: ["aiProvider"],
  };
}
