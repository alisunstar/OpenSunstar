import { useNavigate } from "react-router-dom";
import { useI18n } from "../i18n/provider";
import { appendProjectPath } from "../projectPathsStorage";
import { NavIconFolderPlus } from "./navIcons";

export default function AddProjectNavButton() {
  const { t, locale } = useI18n();
  const navigate = useNavigate();

  const pickFolder = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title: locale === "zh" ? "选择项目文件夹" : "Choose project folder",
      });
      if (selected === null) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (typeof path === "string" && path.length > 0) {
        appendProjectPath(path);
        navigate(`/project?path=${encodeURIComponent(path)}`);
      }
    } catch {
      const manual = window.prompt(
        locale === "zh"
          ? "无法打开系统文件夹对话框。\n请粘贴项目根目录的完整路径（或使用桌面客户端）："
          : "Unable to open system folder picker.\nPaste the full project root path:",
      );
      const trimmed = manual?.trim();
      if (trimmed) {
        appendProjectPath(trimmed);
        navigate(`/project?path=${encodeURIComponent(trimmed)}`);
      }
    }
  };

  return (
    <button
      type="button"
      className="side-nav-link side-nav-action"
      onClick={pickFolder}
      title={t("nav.addProjectTitle")}
    >
      <span className="side-nav-link__icon">
        <NavIconFolderPlus />
      </span>
      <span className="side-nav-link__label side-nav-link__label--cjk-optical">
        {t("nav.addProject")}
      </span>
    </button>
  );
}
