import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { NavLink, useNavigate } from "react-router-dom";
import { useI18n } from "../i18n/provider";
import { revealPathInFolder } from "../api/reveal";
import { removeProjectPath } from "../projectPathsStorage";
import { NavIconFolder } from "./navIcons";

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "Project";
}

type Props = {
  projectPath: string;
  isCurrent: boolean;
  pendingActivePath?: string | null;
  onPendingActivePath?: (path: string) => void;
};

export default function ProjectNavItem({
  projectPath,
  isCurrent,
  pendingActivePath = null,
  onPendingActivePath,
}: Props) {
  const { locale } = useI18n();
  const navigate = useNavigate();
  const to = `/project?path=${encodeURIComponent(projectPath)}`;
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onPointerDown = (e: PointerEvent) => {
      if (menuRef.current?.contains(e.target as Node)) return;
      close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKey);
    };
  }, [menu]);

  const onContextMenuCapture = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setMenu({ x: e.clientX, y: e.clientY });
  };

  const closeMenu = () => setMenu(null);

  const handleRemove = () => {
    const msg =
      locale === "zh"
        ? `从侧栏移除「${folderBasename(projectPath)}」？\n不会删除磁盘上的文件夹。`
        : `Remove "${folderBasename(projectPath)}" from sidebar?\nThis will not delete files on disk.`;
    if (
      !window.confirm(msg)
    ) {
      return;
    }
    removeProjectPath(projectPath);
    closeMenu();
    if (isCurrent) {
      navigate("/");
    }
  };

  return (
    <>
      <div
        className="side-nav-project-item"
        onContextMenuCapture={onContextMenuCapture}
      >
        <NavLink
          to={to}
          className={() =>
            `side-nav-link${(pendingActivePath ? pendingActivePath === to : isCurrent) ? " active" : ""}`
          }
          title={projectPath}
          onPointerDown={() => onPendingActivePath?.(to)}
          onClick={() => onPendingActivePath?.(to)}
        >
          <span className="side-nav-link__icon">
            <NavIconFolder />
          </span>
          <span className="side-nav-link__label">
            {folderBasename(projectPath)}
          </span>
        </NavLink>
      </div>
      {menu
        ? createPortal(
            <div
              ref={menuRef}
              className="card-context-menu"
              style={{
                position: "fixed",
                left: menu.x,
                top: menu.y,
                zIndex: 10_000,
              }}
              role="menu"
              aria-label={locale === "zh" ? "项目操作" : "Project actions"}
            >
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                onClick={() => {
                  void revealPathInFolder(projectPath, { alertOnError: true });
                  closeMenu();
                }}
              >
                {locale === "zh" ? "打开所在目录" : "Open containing folder"}
              </button>
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item card-context-menu__item--danger"
                onClick={handleRemove}
              >
                {locale === "zh" ? "删除项目" : "Remove project"}
              </button>
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
