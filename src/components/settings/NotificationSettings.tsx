import { useTranslation } from "react-i18next";
import type { SettingsFormState } from "@/hooks/useSettings";
import { useWorkspaceAlertFirst } from "@/hooks/useWorkspaceAlertFirst";
import {
  Bell,
  Wallet,
  AlertTriangle,
  CircleDot,
  LayoutDashboard,
} from "lucide-react";
import { ToggleRow } from "@/components/ui/toggle-row";

interface NotificationSettingsProps {
  settings: SettingsFormState;
  onChange: (updates: Partial<SettingsFormState>) => void;
}

export function NotificationSettings({
  settings,
  onChange,
}: NotificationSettingsProps) {
  const { t } = useTranslation();
  const { enabled: workspaceAlertFirst, setEnabled: setWorkspaceAlertFirst } =
    useWorkspaceAlertFirst();
  const prefs = settings.notificationPreferences ?? {
    budgetAlert: true,
    failoverAlert: true,
    trayBadge: true,
  };

  const updatePrefs = (
    key: "budgetAlert" | "failoverAlert" | "trayBadge",
    value: boolean,
  ) => {
    onChange({
      notificationPreferences: { ...prefs, [key]: value },
    });
  };

  return (
    <section className="space-y-4">
      <div className="flex items-center gap-2 pb-2 border-b border-border/40">
        <Bell className="h-4 w-4 text-primary" />
        <h3 className="text-sm font-medium">
          {t("settings.notificationPreferences")}
        </h3>
      </div>

      <div className="space-y-3">
        <ToggleRow
          icon={<Wallet className="h-4 w-4 text-orange-500" />}
          title={t("settings.budgetAlertNotifications")}
          description={t("settings.budgetAlertNotificationsDescription")}
          checked={!!prefs.budgetAlert}
          onCheckedChange={(value) => updatePrefs("budgetAlert", value)}
        />

        <ToggleRow
          icon={<AlertTriangle className="h-4 w-4 text-red-500" />}
          title={t("settings.failoverAlertNotifications")}
          description={t("settings.failoverAlertNotificationsDescription")}
          checked={!!prefs.failoverAlert}
          onCheckedChange={(value) => updatePrefs("failoverAlert", value)}
        />

        <ToggleRow
          icon={<CircleDot className="h-4 w-4 text-blue-500" />}
          title={t("settings.trayBadgeNotifications")}
          description={t("settings.trayBadgeNotificationsDescription")}
          checked={!!prefs.trayBadge}
          onCheckedChange={(value) => updatePrefs("trayBadge", value)}
        />

        <ToggleRow
          icon={<LayoutDashboard className="h-4 w-4 text-emerald-500" />}
          title={t("settings.workspaceAlertFirst")}
          description={t("settings.workspaceAlertFirstDescription")}
          checked={workspaceAlertFirst}
          onCheckedChange={setWorkspaceAlertFirst}
        />
      </div>
    </section>
  );
}
