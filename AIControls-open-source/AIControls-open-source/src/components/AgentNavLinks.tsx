import { useEffect, useRef, useState, type MouseEvent } from "react";
import { createPortal } from "react-dom";
import { NavLink, useNavigate } from "react-router-dom";
import {
  addUserAgentFromPath,
  getAgentSkillPaths,
  listDetectedAgents,
  removeAgentFromSidebar,
  setAgentCustomSkillPaths,
  type AgentScanResult,
} from "../api/agents";
import { invalidateCachedAgentGlobalInventory } from "../api/agentInventoryCache";
import { revealPathInFolder } from "../api/reveal";
import { useI18n } from "../i18n/provider";
import { NavIconFolderPlus, NavIconForAgent } from "./navIcons";

function navClass(active: boolean) {
  return `side-nav-link${active ? " active" : ""}`;
}

type Props = {
  pendingActivePath?: string | null;
  onPendingActivePath?: (path: string) => void;
};

export default function AgentNavLinks({
  pendingActivePath = null,
  onPendingActivePath,
}: Props) {
  const { t, locale } = useI18n();
  const navigate = useNavigate();
  const [agents, setAgents] = useState<AgentScanResult[] | null>(null);
  const [listNonce, setListNonce] = useState(0);
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    agent: AgentScanResult;
  } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [configModal, setConfigModal] = useState<AgentScanResult | null>(null);

  useEffect(() => {
    listDetectedAgents().then(setAgents);
  }, [listNonce]);

  useEffect(() => {
    const bump = () => setListNonce((n) => n + 1);
    window.addEventListener("aicontrols-agents-changed", bump);
    return () => window.removeEventListener("aicontrols-agents-changed", bump);
  }, []);

  const pickAddAgentFolder = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title:
          locale === "zh"
            ? "选择以 . 开头的配置目录（如 .myagent）"
            : "Choose a dot-folder (e.g. .myagent)",
      });
      if (selected === null) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (typeof path !== "string" || path.length === 0) return;
      const r = await addUserAgentFromPath(path);
      if ("error" in r) {
        window.alert(r.error);
        return;
      }
      invalidateCachedAgentGlobalInventory();
      window.dispatchEvent(new Event("aicontrols-agents-changed"));
      navigate(`/agent/${r.id}`);
    } catch {
      const manual = window.prompt(
        locale === "zh"
          ? "无法打开文件夹对话框。请粘贴以 . 开头的配置目录完整路径："
          : "Folder picker unavailable. Paste the full path to a dot-folder:",
      );
      const trimmed = manual?.trim();
      if (!trimmed) return;
      const r = await addUserAgentFromPath(trimmed);
      if ("error" in r) {
        window.alert(r.error);
        return;
      }
      invalidateCachedAgentGlobalInventory();
      window.dispatchEvent(new Event("aicontrols-agents-changed"));
      navigate(`/agent/${r.id}`);
    }
  };

  const addAgentButton = (
    <button
      type="button"
      className="side-nav-link side-nav-action"
      onClick={() => void pickAddAgentFolder()}
      title={t("nav.addAgentTitle")}
    >
      <span className="side-nav-link__icon">
        <NavIconFolderPlus />
      </span>
      <span className="side-nav-link__label side-nav-link__label--cjk-optical">
        {t("nav.addAgent")}
      </span>
    </button>
  );

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

  const closeMenu = () => setMenu(null);

  const openAgentMenu = (e: MouseEvent, agent: AgentScanResult) => {
    e.preventDefault();
    e.stopPropagation();
    setMenu({ x: e.clientX, y: e.clientY, agent });
  };

  if (agents === null) {
    return (
      <div className="side-nav-sub-label" aria-live="polite">
        {t("nav.agentScanning")}
      </div>
    );
  }

  if (agents.length === 0) {
    return (
      <>
        <p
          className="side-nav-sub-label"
          title="安装 Cursor、Claude Code、Codex、Hermes、OpenClaw、Trae、Qoder、Kiro 或生成对应用户目录后重新打开"
        >
          {t("nav.noAgents")}
        </p>
        {addAgentButton}
      </>
    );
  }

  return (
    <>
      {agents.map((a) => {
        const to = `/agent/${a.id}`;
        return (
          <NavLink
            key={a.id}
            to={to}
            className={({ isActive }) =>
              navClass(pendingActivePath ? pendingActivePath === to : isActive)
            }
            title={a.rootPath ?? a.label}
            onContextMenuCapture={(e) => openAgentMenu(e, a)}
            onPointerDown={(e) => {
              if (e.button === 0) onPendingActivePath?.(to);
            }}
            onClick={() => onPendingActivePath?.(to)}
          >
            <span className="side-nav-link__icon">
              {NavIconForAgent(a.id)}
            </span>
            <span className="side-nav-link__label">{a.label}</span>
          </NavLink>
        );
      })}
      {addAgentButton}
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
              aria-label={locale === "zh" ? "Agent 操作" : "Agent actions"}
            >
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                disabled={!menu.agent.rootPath}
                onClick={() => {
                  if (menu.agent.rootPath) {
                    void revealPathInFolder(menu.agent.rootPath, {
                      alertOnError: true,
                    });
                  }
                  closeMenu();
                }}
              >
                {locale === "zh" ? "打开所在目录" : "Open containing folder"}
              </button>
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                onClick={() => {
                  setConfigModal(menu.agent);
                  closeMenu();
                }}
              >
                {locale === "zh" ? "配置…" : "Configure…"}
              </button>
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item card-context-menu__item--danger"
                onClick={() => {
                  void (async () => {
                    const ok =
                      locale === "zh"
                        ? window.confirm(
                            "从侧栏列表移除此 Agent？\n内置 Agent 可在「设置」中恢复显示。",
                          )
                        : window.confirm(
                            "Remove this agent from the sidebar?\nYou can restore built-in agents in Settings.",
                          );
                    if (!ok) return;
                    const r = await removeAgentFromSidebar(menu.agent.id);
                    if ("error" in r) {
                      window.alert(r.error);
                    } else {
                      invalidateCachedAgentGlobalInventory(menu.agent.id);
                      window.dispatchEvent(new Event("aicontrols-agents-changed"));
                      setListNonce((n) => n + 1);
                    }
                    closeMenu();
                  })();
                }}
              >
                {locale === "zh" ? "从列表移除" : "Remove from list"}
              </button>
            </div>,
            document.body,
          )
        : null}
      {configModal ? (
        <AgentConfigModal
          agent={configModal}
          locale={locale}
          onClose={() => setConfigModal(null)}
        />
      ) : null}
    </>
  );
}

function AgentConfigModal({
  agent,
  locale,
  onClose,
}: {
  agent: AgentScanResult;
  locale: "zh" | "en";
  onClose: () => void;
}) {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [defaultPaths, setDefaultPaths] = useState<string[]>([]);
  const [paths, setPaths] = useState<string[] | null>(null);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [editingDraft, setEditingDraft] = useState("");
  const [toast, setToast] = useState<{ message: string; kind: "success" | "error" } | null>(null);
  const editInputRef = useRef<HTMLInputElement>(null);

  const isModified = paths !== null;
  const displayPaths = paths ?? defaultPaths;

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const result = await getAgentSkillPaths(agent.id);
      if (cancelled) return;
      if ("error" in result) {
        setToast({ kind: "error", message: result.error });
        setDefaultPaths([]);
        setPaths([]);
      } else {
        setDefaultPaths(result.defaultPaths);
        setPaths(result.customPaths.length > 0 ? result.customPaths : null);
      }
      setLoading(false);
    })();
    return () => { cancelled = true; };
  }, [agent.id]);

  useEffect(() => {
    if (!toast) return;
    const t = window.setTimeout(() => setToast(null), 2000);
    return () => window.clearTimeout(t);
  }, [toast]);

  useEffect(() => {
    if (editingIndex === null) return;
    const id = window.requestAnimationFrame(() => {
      editInputRef.current?.focus();
      editInputRef.current?.select();
    });
    return () => window.cancelAnimationFrame(id);
  }, [editingIndex]);

  async function saveList(list: string[]) {
    setSaving(true);
    const result = await setAgentCustomSkillPaths(agent.id, list);
    if ("error" in result) {
      setToast({ kind: "error", message: result.error });
    } else {
      invalidateCachedAgentGlobalInventory(agent.id);
      window.dispatchEvent(new Event("aicontrols-agents-changed"));
      setToast({ kind: "success", message: locale === "zh" ? "已保存" : "Saved" });
    }
    setSaving(false);
  }

  async function pickFolder(): Promise<string | null> {
    try {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: locale === "zh" ? "选择 Skill 搜索路径" : "Choose skill search path",
      });
      if (selected === null) return null;
      const p = Array.isArray(selected) ? selected[0] : selected;
      return typeof p === "string" && p.length > 0 ? p : null;
    } catch {
      // dialog unavailable — fallback to prompt
      const manual = window.prompt(
        locale === "zh"
          ? "无法打开文件夹选择器，请粘贴路径："
          : "Folder picker unavailable. Paste the path:",
      );
      return manual?.trim() || null;
    }
  }

  async function addPath() {
    const folder = await pickFolder();
    if (!folder) return;
    const current = paths ?? [...defaultPaths];
    if (current.includes(folder)) {
      setToast({ kind: "error", message: locale === "zh" ? "路径已存在" : "Path already exists" });
      return;
    }
    const next = [...current, folder];
    setPaths(next);
    void saveList(next);
  }

  function startEdit(index: number) {
    setEditingIndex(index);
    setEditingDraft(displayPaths[index]);
  }

  async function browseForEdit() {
    if (editingIndex === null) return;
    const folder = await pickFolder();
    if (!folder) return;
    setEditingDraft(folder);
  }

  function confirmEdit() {
    if (editingIndex === null) return;
    const trimmed = editingDraft.trim();
    if (!trimmed) {
      setToast({ kind: "error", message: locale === "zh" ? "路径不能为空" : "Path cannot be empty" });
      return;
    }
    const current = paths ?? [...defaultPaths];
    if (trimmed === current[editingIndex]) {
      setEditingIndex(null);
      return;
    }
    const otherPaths = current.filter((_, i) => i !== editingIndex);
    if (otherPaths.includes(trimmed)) {
      setToast({ kind: "error", message: locale === "zh" ? "路径已存在" : "Path already exists" });
      return;
    }
    const next = current.map((p, i) => (i === editingIndex ? trimmed : p));
    setPaths(next);
    setEditingIndex(null);
    setEditingDraft("");
    void saveList(next);
  }

  function cancelEdit() {
    setEditingIndex(null);
    setEditingDraft("");
  }

  function removePath(index: number) {
    const current = paths ?? [...defaultPaths];
    const next = current.filter((_, i) => i !== index);
    setPaths(next);
    if (editingIndex === index) {
      setEditingIndex(null);
      setEditingDraft("");
    }
    void saveList(next);
  }

  function resetToDefault() {
    setPaths(null);
    setEditingIndex(null);
    setEditingDraft("");
    void saveList([]);
  }

  return createPortal(
    <div className="prompt-create-modal-root">
      <div className="prompt-create-modal-backdrop" onClick={onClose} aria-hidden />
      <div
        className="prompt-create-modal prompt-command-modal agent-config-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="agent-config-title"
      >
        <header className="prompt-create-modal__header">
          <div className="prompt-create-modal__header-text">
            <h2 id="agent-config-title" className="prompt-create-modal__title">
              {locale === "zh" ? `配置 · ${agent.label}` : `Configure · ${agent.label}`}
            </h2>
            <p className="prompt-create-modal__subtitle">
              {locale === "zh"
                ? "管理此 Agent 的 Skill 搜索路径。点击路径可重新选择目录，也可手动编辑。"
                : "Manage skill search paths. Click a path to re-select a directory, or edit manually."}
            </p>
          </div>
          <button
            type="button"
            className="prompt-create-modal__close"
            onClick={onClose}
            aria-label={locale === "zh" ? "关闭" : "Close"}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden>
              <path
                d="M6 6l12 12M18 6L6 18"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </header>

        <div className="prompt-create-modal__body">
          {loading ? (
            <p className="muted">{locale === "zh" ? "正在加载…" : "Loading…"}</p>
          ) : (
            <>
              <div className="agent-config-modal__section">
                <div className="agent-config-modal__section-head">
                  <h3 className="agent-config-modal__section-title">
                    {locale === "zh" ? "Skill 搜索路径" : "Skill Search Paths"}
                  </h3>
                  {isModified ? (
                    <button
                      type="button"
                      className="agent-config-modal__reset-btn"
                      disabled={saving}
                      onClick={resetToDefault}
                    >
                      {locale === "zh" ? "恢复默认" : "Reset to default"}
                    </button>
                  ) : (
                    <span className="agent-config-modal__default-badge">
                      {locale === "zh" ? "默认" : "Default"}
                    </span>
                  )}
                </div>
                <ul className="agent-config-modal__path-list">
                  {displayPaths.map((p, i) => (
                    <li key={i} className={`agent-config-modal__path-item${editingIndex === i ? " is-editing" : ""}`}>
                      {editingIndex === i ? (
                        <div className="agent-config-modal__edit-row">
                          <input
                            ref={editInputRef}
                            className="agent-config-modal__edit-input"
                            type="text"
                            value={editingDraft}
                            onChange={(e) => setEditingDraft(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Escape") cancelEdit();
                              if (e.key === "Enter") { e.preventDefault(); confirmEdit(); }
                            }}
                            autoComplete="off"
                            spellCheck={false}
                          />
                          <button
                            type="button"
                            className="agent-config-modal__browse-btn"
                            disabled={saving}
                            onClick={() => void browseForEdit()}
                            title={locale === "zh" ? "选择文件夹" : "Choose folder"}
                          >
                            {locale === "zh" ? "选择" : "Pick"}
                          </button>
                          <button
                            type="button"
                            className="agent-config-modal__edit-confirm"
                            disabled={saving || !editingDraft.trim()}
                            onClick={confirmEdit}
                            aria-label={locale === "zh" ? "确认" : "Confirm"}
                          >
                            ✓
                          </button>
                          <button
                            type="button"
                            className="agent-config-modal__path-remove agent-config-modal__path-remove--edit"
                            onClick={cancelEdit}
                            aria-label={locale === "zh" ? "取消" : "Cancel"}
                          >
                            ✕
                          </button>
                        </div>
                      ) : (
                        <>
                          <span
                            className="agent-config-modal__path-text agent-config-modal__path-text--editable"
                            title={p}
                            role="button"
                            tabIndex={0}
                            onClick={() => startEdit(i)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ") {
                                e.preventDefault();
                                startEdit(i);
                              }
                            }}
                          >
                            {p}
                          </span>
                          <button
                            type="button"
                            className="agent-config-modal__path-remove"
                            disabled={saving}
                            onClick={() => removePath(i)}
                            aria-label={locale === "zh" ? "移除路径" : "Remove path"}
                          >
                            ✕
                          </button>
                        </>
                      )}
                    </li>
                  ))}
                  {displayPaths.length === 0 ? (
                    <li className="agent-config-modal__path-item agent-config-modal__path-item--empty">
                      {locale === "zh" ? "暂无路径" : "No paths"}
                    </li>
                  ) : null}
                </ul>
                <div className="agent-config-modal__add-row">
                  <button
                    type="button"
                    className="agent-config-modal__add-btn"
                    disabled={saving}
                    onClick={() => void addPath()}
                  >
                    + {locale === "zh" ? "选择文件夹添加" : "Add folder"}
                  </button>
                </div>
              </div>
            </>
          )}
        </div>

        <footer className="prompt-create-modal__footer">
          <span className="prompt-create-modal__kbd-hint">
            {locale === "zh" ? "Esc 关闭" : "Esc to close"}
          </span>
          <div className="prompt-create-modal__actions">
            <button
              type="button"
              className="prompt-create-modal__cancel"
              onClick={onClose}
              disabled={saving}
            >
              {locale === "zh" ? "关闭" : "Close"}
            </button>
          </div>
        </footer>
      </div>
      {toast ? (
        <div className="toast-stack" style={{ position: "fixed", bottom: 24, right: 24, zIndex: 10001 }}>
          <div className={`toast ${toast.kind === "error" ? "toast--error" : "toast--success"}`}>
            <span className="toast__text">{toast.message}</span>
          </div>
        </div>
      ) : null}
    </div>,
    document.body,
  );
}
