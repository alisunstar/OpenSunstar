import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  convertPromptToMySkill,
  getPromptLibrary,
  savePromptLibrary,
  type PromptFolder,
  type PromptItem,
  type PromptLibraryFile,
  type PromptType,
} from "../api/prompts";
import { getMySkillsLibrary } from "../api/mySkills";
import {
  AGENT_COMMAND_SEGMENT_SLUG_RE,
  agentCommandSegmentInvalidMessage,
  isValidAgentCommandSegmentInput,
} from "../agentCommandInput";
import { useI18n } from "../i18n/provider";

const TYPE_META: Record<PromptType, { label: string; rootName: string }> = {
  image: { label: "图片", rootName: "图片" },
  code: { label: "代码", rootName: "代码" },
  doc: { label: "文档", rootName: "文档" },
  text: { label: "纯文本", rootName: "纯文本" },
};

const TYPE_LABEL_EN: Record<PromptType, string> = {
  image: "Image",
  code: "Code",
  doc: "Document",
  text: "Text",
};

const PROMPT_TYPES = Object.keys(TYPE_META) as PromptType[];

type Toast = { message: string; kind: "success" | "error" };

const MAX_IMAGE_BYTES = 2 * 1024 * 1024;

function emptyLibrary(): PromptLibraryFile {
  return { version: 1, folders: [], items: [] };
}

function ensureRootFolders(lib: PromptLibraryFile): PromptLibraryFile {
  const next = { ...lib, folders: [...lib.folders] };
  for (const t of PROMPT_TYPES) {
    if (!next.folders.some((f) => f.id === t)) {
      next.folders.push({ id: t, name: TYPE_META[t].rootName, parentId: null });
    }
  }
  return next;
}

function isFolderInType(folderId: string, type: PromptType, folders: PromptFolder[]): boolean {
  if (folderId === type) return true;
  return folders.some((folder) => folder.id === folderId && folder.parentId === type);
}

function slugifyCommandName(input: string): string {
  const out = input
    .trim()
    .toLowerCase()
    .replace(/^\/?cp-/, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .replace(/-{2,}/g, "-");
  return out || "prompt";
}

function commandNameForItem(item: PromptItem): string {
  return slugifyCommandName(item.commandName || item.title);
}

function displayPromptCommand(commandName: string): string {
  return `/cp-${commandName}`;
}

function reconcileConvertedSkillStatus(
  lib: PromptLibraryFile,
  mySkillIds: Set<string>,
): { library: PromptLibraryFile; changed: boolean } {
  let changed = false;
  const items = lib.items.map((item) => {
    const convertedSkillId = item.convertedSkillId?.trim();
    if (!convertedSkillId || mySkillIds.has(convertedSkillId)) return item;

    changed = true;
    return {
      ...item,
      convertedSkillId: null,
    };
  });

  return {
    library: changed ? { ...lib, items } : lib,
    changed,
  };
}

export default function PromptLibraryPage() {
  const { locale } = useI18n();
  const typeLabel = (t: PromptType) => (locale === "zh" ? TYPE_META[t].label : TYPE_LABEL_EN[t]);

  const [library, setLibrary] = useState<PromptLibraryFile>(emptyLibrary());
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);
  const [activeType, setActiveType] = useState<PromptType>("image");
  const [activeFolderId, setActiveFolderId] = useState<string>("image");
  const [search, setSearch] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  /** 非空表示在编辑已有条目，否则为新建 */
  const [editingItemId, setEditingItemId] = useState<string | null>(null);
  const [newItem, setNewItem] = useState({
    title: "",
    prompt: "",
    outputType: "image" as PromptType,
    outputExample: "",
    relatedLink: "",
  });
  const [newOutputImageDataUrl, setNewOutputImageDataUrl] = useState<string | null>(null);
  const [cardContextMenu, setCardContextMenu] = useState<{
    x: number;
    y: number;
    item: PromptItem;
  } | null>(null);
  const [commandEditor, setCommandEditor] = useState<{
    itemId: string;
    commandName: string;
  } | null>(null);
  const [skillConvertEditor, setSkillConvertEditor] = useState<{
    itemId: string;
    skillName: string;
  } | null>(null);
  const [groupPicker, setGroupPicker] = useState<{
    itemId: string;
  } | null>(null);
  /** Tauri WebView 中 window.prompt 不可用，用模态框输入组名 */
  const [groupNameModal, setGroupNameModal] = useState<{
    promptType: PromptType;
    assignItemId: string | null;
  } | null>(null);
  const [newGroupNameDraft, setNewGroupNameDraft] = useState("");
  const cardContextMenuRef = useRef<HTMLDivElement>(null);
  const createTitleInputRef = useRef<HTMLInputElement>(null);
  const newGroupNameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let cancelled = false;
    let frameId: number | null = null;
    let timerId: number | null = null;

    const loadPromptLibrary = async () => {
      try {
        setLoading(true);
        const [rawLib, mySkillsLib] = await Promise.all([
          getPromptLibrary(),
          getMySkillsLibrary(),
        ]);
        const lib = ensureRootFolders(rawLib);
        const mySkillIds = new Set(
          mySkillsLib.items.map((item) => item.id.trim()).filter(Boolean),
        );
        const { library: syncedLib, changed } = reconcileConvertedSkillStatus(
          lib,
          mySkillIds,
        );
        if (changed) {
          await savePromptLibrary(syncedLib);
        }
        if (!cancelled) {
          setLibrary(syncedLib);
          setActiveFolderId(activeType);
        }
      } catch (e) {
        if (!cancelled) setErr(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    frameId = window.requestAnimationFrame(() => {
      timerId = window.setTimeout(() => {
        void loadPromptLibrary();
      }, 0);
    });

    return () => {
      cancelled = true;
      if (frameId !== null) window.cancelAnimationFrame(frameId);
      if (timerId !== null) window.clearTimeout(timerId);
    };
  }, []);

  useEffect(() => {
    if (!toast) return;
    const t = window.setTimeout(() => setToast(null), 1800);
    return () => window.clearTimeout(t);
  }, [toast]);

  useEffect(() => {
    if (!cardContextMenu) return;
    const onPointerDown = (e: PointerEvent) => {
      if (cardContextMenuRef.current?.contains(e.target as Node)) return;
      setCardContextMenu(null);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setCardContextMenu(null);
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKey);
    };
  }, [cardContextMenu]);

  const folderDescendants = useMemo(() => {
    const ids = new Set<string>([activeFolderId]);
    let added = true;
    while (added) {
      added = false;
      for (const f of library.folders) {
        if (f.parentId && ids.has(f.parentId) && !ids.has(f.id)) {
          ids.add(f.id);
          added = true;
        }
      }
    }
    return ids;
  }, [activeFolderId, library.folders]);

  const activeGroups = useMemo(
    () =>
      library.folders
        .filter((folder) => folder.parentId === activeType)
        .slice()
        .sort((a, b) => a.name.localeCompare(b.name)),
    [activeType, library.folders],
  );

  const activeTypeItemCount = useMemo(
    () => library.items.filter((item) => item.type === activeType).length,
    [activeType, library.items],
  );

  const groupCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const item of library.items) {
      counts.set(item.folderId, (counts.get(item.folderId) ?? 0) + 1);
    }
    return counts;
  }, [library.items]);

  const filteredItems = useMemo(() => {
    const q = search.trim().toLowerCase();
    return library.items
      .filter((item) => item.type === activeType)
      .filter((item) => folderDescendants.has(item.folderId))
      .filter((item) => {
        if (!q) return true;
        return (
          item.title.toLowerCase().includes(q) ||
          item.prompt.toLowerCase().includes(q) ||
          (item.commandName ? displayPromptCommand(item.commandName).toLowerCase().includes(q) : false) ||
          (item.outputExample ?? "").toLowerCase().includes(q)
        );
      })
      .sort((a, b) => b.updatedAt - a.updatedAt);
  }, [activeType, folderDescendants, library.items, search]);

  const masonryColumns = useMemo(() => {
    const cols: [PromptItem[], PromptItem[], PromptItem[]] = [[], [], []];
    for (let i = 0; i < filteredItems.length; i += 1) {
      cols[i % 3].push(filteredItems[i]);
    }
    return cols;
  }, [filteredItems]);

  async function persist(next: PromptLibraryFile) {
    try {
      setSaving(true);
      await savePromptLibrary(next);
      setLibrary(next);
      setErr(null);
    } catch (e) {
      setErr(String(e));
      throw e;
    } finally {
      setSaving(false);
    }
  }

  function groupNameExists(type: PromptType, name: string): boolean {
    const normalized = name.trim().toLowerCase();
    return library.folders.some(
      (folder) => folder.parentId === type && folder.name.trim().toLowerCase() === normalized,
    );
  }

  function openNewGroupModal(type: PromptType, assignItemId: string | null) {
    setGroupNameModal({ promptType: type, assignItemId });
    setNewGroupNameDraft("");
  }

  async function confirmNewGroup() {
    if (!groupNameModal) return;
    const name = newGroupNameDraft.trim();
    if (!name) {
      setToast({ kind: "error", message: locale === "zh" ? "请填写组名" : "Please enter a group name" });
      return;
    }
    const { promptType: type, assignItemId } = groupNameModal;
    if (groupNameExists(type, name)) {
      setToast({ kind: "error", message: locale === "zh" ? "同名组已存在" : "A group with this name already exists" });
      return;
    }
    const group: PromptFolder = {
      id: `group-${crypto.randomUUID()}`,
      name,
      parentId: type,
    };
    try {
      if (assignItemId) {
        const next = {
          ...library,
          folders: [...library.folders, group],
          items: library.items.map((x) =>
            x.id === assignItemId ? { ...x, folderId: group.id, updatedAt: Date.now() } : x,
          ),
        };
        await persist(next);
        setToast({
          kind: "success",
          message: locale === "zh" ? "已创建组并添加" : "Group created and item added",
        });
      } else {
        const next = { ...library, folders: [...library.folders, group] };
        await persist(next);
        setToast({ kind: "success", message: locale === "zh" ? "组已创建" : "Group created" });
      }
      setGroupNameModal(null);
      setNewGroupNameDraft("");
      setActiveType(type);
      setActiveFolderId(group.id);
    } catch {
      /* persist 已 setErr */
    }
  }

  async function assignItemToGroup(item: PromptItem, folderId: string) {
    if (!isFolderInType(folderId, item.type, library.folders)) {
      setToast({ kind: "error", message: locale === "zh" ? "组不存在或类型不匹配" : "Group does not exist or type mismatch" });
      return;
    }
    const next = {
      ...library,
      items: library.items.map((x) =>
        x.id === item.id ? { ...x, folderId, updatedAt: Date.now() } : x,
      ),
    };
    await persist(next);
    setGroupPicker(null);
    setActiveType(item.type);
    setActiveFolderId(folderId);
    setToast({ kind: "success", message: locale === "zh" ? "已添加到组" : "Added to group" });
  }

  function resetCreateState() {
    setNewItem({
      title: "",
      prompt: "",
      outputType: activeType,
      outputExample: "",
      relatedLink: "",
    });
    setNewOutputImageDataUrl(null);
  }

  function closeCreateModal() {
    setShowCreate(false);
    setEditingItemId(null);
    resetCreateState();
  }

  function openCreateModal() {
    setCardContextMenu(null);
    setEditingItemId(null);
    setShowCreate(true);
    resetCreateState();
  }

  function openEditModal(item: PromptItem) {
    setCardContextMenu(null);
    setEditingItemId(item.id);
    const outputType = item.outputType ?? item.type;
    setNewItem({
      title: item.title,
      prompt: item.prompt,
      outputType,
      outputExample: outputType === "image" ? "" : (item.outputExample ?? ""),
      relatedLink: item.relatedLink ?? "",
    });
    setNewOutputImageDataUrl(item.type === "image" ? (item.imageDataUrl ?? null) : null);
    setShowCreate(true);
  }

  useEffect(() => {
    if (!showCreate) return;
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeCreateModal();
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = prevOverflow;
      document.removeEventListener("keydown", onKey);
    };
  }, [showCreate]);

  useEffect(() => {
    if (!showCreate) return;
    const id = window.requestAnimationFrame(() => {
      createTitleInputRef.current?.focus();
    });
    return () => window.cancelAnimationFrame(id);
  }, [showCreate]);

  useEffect(() => {
    if (!groupNameModal) return;
    const prevOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setGroupNameModal(null);
        setNewGroupNameDraft("");
      }
    };
    document.addEventListener("keydown", onKey);
    const id = window.requestAnimationFrame(() => {
      newGroupNameInputRef.current?.focus();
    });
    return () => {
      document.body.style.overflow = prevOverflow;
      document.removeEventListener("keydown", onKey);
      window.cancelAnimationFrame(id);
    };
  }, [groupNameModal]);

  async function onSubmitEditor() {
    const title = newItem.title.trim();
    const prompt = newItem.prompt.trim();
    const outputType = newItem.outputType;
    const outputExample = newItem.outputExample.trim();
    const relatedLink = newItem.relatedLink.trim();
    if (!title) {
      setToast({ kind: "error", message: locale === "zh" ? "请填写标题" : "Please enter a title" });
      return;
    }
    if (outputType !== "image" && !prompt) {
      setToast({ kind: "error", message: locale === "zh" ? "请填写 Prompt" : "Please enter prompt" });
      return;
    }
    const prev =
      editingItemId !== null ? library.items.find((x) => x.id === editingItemId) ?? null : null;
    if (editingItemId !== null && !prev) {
      setToast({ kind: "error", message: locale === "zh" ? "条目不存在或已删除" : "Item does not exist or was deleted" });
      closeCreateModal();
      return;
    }
    const now = Date.now();
    if (editingItemId !== null && prev) {
      const imageDataUrl =
        outputType === "image"
          ? (newOutputImageDataUrl ?? (prev.imageDataUrl ?? null))
          : null;
      const nextFolderId =
        prev.type === outputType && isFolderInType(prev.folderId, outputType, library.folders)
          ? prev.folderId
          : outputType;
      const updated: PromptItem = {
        ...prev,
        type: outputType,
        title,
        prompt,
        outputType,
        outputExample: outputType === "image" ? "" : outputExample,
        relatedLink: relatedLink || null,
        imageDataUrl,
        folderId: nextFolderId,
        updatedAt: now,
      };
      const next = {
        ...library,
        items: library.items.map((x) => (x.id === editingItemId ? updated : x)),
      };
      await persist(next);
      closeCreateModal();
      setActiveType(outputType);
      setActiveFolderId(nextFolderId);
      setToast({ kind: "success", message: locale === "zh" ? "已保存修改" : "Changes saved" });
      return;
    }

    const newFolderId =
      activeType === outputType && isFolderInType(activeFolderId, outputType, library.folders)
        ? activeFolderId
        : outputType;
    const item: PromptItem = {
      id: crypto.randomUUID(),
      type: outputType,
      title,
      prompt,
      outputType,
      outputExample: outputType === "image" ? "" : outputExample,
      relatedLink: relatedLink || null,
      imageDataUrl: outputType === "image" ? newOutputImageDataUrl : null,
      tags: [],
      note: "",
      folderId: newFolderId,
      createdAt: now,
      updatedAt: now,
    };
    const next = { ...library, items: [item, ...library.items] };
    await persist(next);
    closeCreateModal();
    setActiveType(outputType);
    setActiveFolderId(newFolderId);
    setToast({ kind: "success", message: locale === "zh" ? "已保存" : "Saved" });
  }

  async function onDeleteItem(id: string) {
    const next = { ...library, items: library.items.filter((x) => x.id !== id) };
    await persist(next);
    setToast({ kind: "success", message: locale === "zh" ? "已删除" : "Deleted" });
  }

  async function copyPrompt(prompt: string) {
    try {
      await navigator.clipboard.writeText(prompt);
      setToast({ kind: "success", message: locale === "zh" ? "已复制 Prompt" : "Prompt copied" });
    } catch {
      setToast({ kind: "error", message: locale === "zh" ? "复制失败" : "Copy failed" });
    }
  }

  async function copyImageDataUrl(dataUrl: string) {
    try {
      const blob = dataUrlToBlob(dataUrl);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const ClipboardItemCtor = (window as any).ClipboardItem as
        | (new (items: Record<string, Blob>) => ClipboardItem)
        | undefined;
      if (!ClipboardItemCtor || !navigator.clipboard?.write) {
        throw new Error("clipboard image write unsupported");
      }
      await navigator.clipboard.write([new ClipboardItemCtor({ [blob.type || "image/png"]: blob })]);
      setToast({ kind: "success", message: locale === "zh" ? "已复制图片" : "Image copied" });
    } catch {
      setToast({ kind: "error", message: locale === "zh" ? "复制图片失败" : "Failed to copy image" });
    }
  }

  async function copyOutputExample(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setToast({ kind: "success", message: locale === "zh" ? "已复制输出示例" : "Output example copied" });
    } catch {
      setToast({ kind: "error", message: locale === "zh" ? "复制失败" : "Copy failed" });
    }
  }

  async function copyPromptCommand(commandName: string) {
    try {
      await navigator.clipboard.writeText(displayPromptCommand(commandName));
      setToast({ kind: "success", message: locale === "zh" ? "已复制 /cp 命令" : "/cp command copied" });
    } catch {
      setToast({ kind: "error", message: locale === "zh" ? "复制失败" : "Copy failed" });
    }
  }

  function commandNameExists(name: string, exceptId: string): boolean {
    return library.items.some(
      (item) =>
        item.id !== exceptId &&
        item.commandEnabled &&
        (item.commandName ?? "").trim() === name,
    );
  }

  function openPromptCommandEditor(item: PromptItem) {
    if (!item.prompt.trim()) {
      setToast({ kind: "error", message: locale === "zh" ? "请先填写 Prompt" : "Please add prompt content first" });
      return;
    }
    setCommandEditor({
      itemId: item.id,
      commandName: commandNameForItem(item),
    });
  }

  async function publishPromptCommand(item: PromptItem, rawName: string) {
    const trimmed = rawName.trim();
    if (!isValidAgentCommandSegmentInput(trimmed)) {
      setToast({ kind: "error", message: agentCommandSegmentInvalidMessage(locale) });
      return;
    }
    const commandName = slugifyCommandName(trimmed);
    if (!AGENT_COMMAND_SEGMENT_SLUG_RE.test(commandName)) {
      setToast({ kind: "error", message: agentCommandSegmentInvalidMessage(locale) });
      return;
    }
    if (commandNameExists(commandName, item.id)) {
      setToast({ kind: "error", message: locale === "zh" ? "该 /cp 命令名已被占用" : "This /cp command is already used" });
      return;
    }
    const now = Date.now();
    const next = {
      ...library,
      items: library.items.map((x) =>
        x.id === item.id
          ? { ...x, commandName, commandEnabled: true, updatedAt: now }
          : x,
      ),
    };
    await persist(next);
    setCommandEditor(null);
    setToast({ kind: "success", message: `${locale === "zh" ? "已发布" : "Published"} ${displayPromptCommand(commandName)}` });
  }

  async function unpublishPromptCommand(item: PromptItem) {
    const next = {
      ...library,
      items: library.items.map((x) =>
        x.id === item.id ? { ...x, commandEnabled: false, updatedAt: Date.now() } : x,
      ),
    };
    await persist(next);
    setToast({ kind: "success", message: locale === "zh" ? "已取消 /cp 发布" : "/cp command unpublished" });
  }

  function openSkillConvertEditor(item: PromptItem) {
    if (!item.prompt.trim()) {
      setToast({ kind: "error", message: locale === "zh" ? "请先填写 Prompt" : "Please add prompt content first" });
      return;
    }
    if (item.convertedSkillId) {
      const ok = window.confirm(
        locale === "zh"
          ? "这条 Prompt 已转过 Skill，是否继续创建一个新的 Skill？"
          : "This prompt was already converted. Create another skill?",
      );
      if (!ok) return;
    }
    setSkillConvertEditor({
      itemId: item.id,
      skillName: commandNameForItem(item),
    });
  }

  async function convertPromptItemToSkill(item: PromptItem, rawSkillName: string) {
    const trimmed = rawSkillName.trim();
    if (!isValidAgentCommandSegmentInput(trimmed)) {
      setToast({ kind: "error", message: agentCommandSegmentInvalidMessage(locale) });
      return;
    }
    const skillName = slugifyCommandName(trimmed);
    if (!AGENT_COMMAND_SEGMENT_SLUG_RE.test(skillName)) {
      setToast({ kind: "error", message: agentCommandSegmentInvalidMessage(locale) });
      return;
    }
    try {
      setSaving(true);
      const skill = await convertPromptToMySkill({
        title: item.title,
        prompt: item.prompt,
        outputType: item.outputType ?? item.type,
        outputExample: item.outputExample ?? "",
        commandName: skillName,
      });
      const next = {
        ...library,
        items: library.items.map((x) =>
          x.id === item.id
            ? { ...x, convertedSkillId: skill.id, updatedAt: Date.now() }
            : x,
        ),
      };
      await savePromptLibrary(next);
      setLibrary(next);
      setErr(null);
      setSkillConvertEditor(null);
      setToast({
        kind: "success",
        message: locale === "zh" ? "已转为 Skill，可在我的 Skills 中同步" : "Converted to Skill. Sync it from My Skills.",
      });
    } catch (e) {
      const msg = String(e);
      setErr(msg);
      setToast({ kind: "error", message: msg });
    } finally {
      setSaving(false);
    }
  }

  function switchType(t: PromptType) {
    setActiveType(t);
    setActiveFolderId(t);
    setCardContextMenu(null);
  }

  function onMasonryCardContextMenu(e: React.MouseEvent, item: PromptItem) {
    e.preventDefault();
    e.stopPropagation();
    const pad = 8;
    const approxW = 200;
    const approxH = 168;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const x = Math.min(Math.max(pad, e.clientX), Math.max(pad, vw - approxW - pad));
    const y = Math.min(Math.max(pad, e.clientY), Math.max(pad, vh - approxH - pad));
    setCardContextMenu({ x, y, item });
  }

  function dataUrlByteLength(dataUrl: string): number {
    const idx = dataUrl.indexOf(",");
    if (idx < 0) return 0;
    const base64 = dataUrl.slice(idx + 1);
    const padding = (base64.match(/=*$/)?.[0].length ?? 0);
    return Math.max(0, (base64.length * 3) / 4 - padding);
  }

  async function loadPastedImageData(file: File) {
    const raw = await fileToDataUrl(file);
    const bytes = dataUrlByteLength(raw);
    if (bytes <= MAX_IMAGE_BYTES) {
      setNewOutputImageDataUrl(raw);
      setToast({ kind: "success", message: locale === "zh" ? "图片已粘贴" : "Image pasted" });
      return;
    }
    const compressed = await compressDataUrlToMax(raw, MAX_IMAGE_BYTES);
    setNewOutputImageDataUrl(compressed);
    setToast({ kind: "success", message: locale === "zh" ? "图片已压缩并粘贴" : "Image compressed and pasted" });
  }

  async function onPasteOutputExample(e: React.ClipboardEvent<HTMLTextAreaElement>) {
    if (newItem.outputType !== "image") return;
    const imageItem = Array.from(e.clipboardData.items).find((x) => x.type.startsWith("image/"));
    if (!imageItem) return;
    const file = imageItem.getAsFile();
    if (!file) return;
    e.preventDefault();
    try {
      await loadPastedImageData(file);
    } catch {
      setToast({ kind: "error", message: locale === "zh" ? "图片处理失败" : "Image processing failed" });
    }
  }

  return (
    <div className="prompt-lib">
      <div className="page-header">
        <div className="page-header__title-bar">
          <div className="page-title__row">
            <h2>{locale === "zh" ? "Prompt 库" : "Prompt Library"}</h2>
            <span className="count-badge">{library.items.length}</span>
          </div>
          <button
            type="button"
            className="page-header__primary-action"
            onClick={openCreateModal}
            disabled={loading || saving}
          >
            <span className="page-header__primary-action-icon" aria-hidden>
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none">
                <path d="M12 5v14M5 12h14" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
              </svg>
            </span>
            <span>{locale === "zh" ? "新建收藏" : "New Item"}</span>
          </button>
        </div>
      </div>

      <div className="toolbar prompt-lib__toolbar">
        <div className="toolbar__left prompt-lib__toolbar-left">
          <div className="prompt-lib__toolbar-main">
            <div className="seg" role="tablist" aria-label={locale === "zh" ? "Prompt 类型" : "Prompt type"}>
              {PROMPT_TYPES.map((t) => (
                <button
                  key={t}
                  className={`seg__item${activeType === t ? " active" : ""}`}
                  onClick={() => switchType(t)}
                >
                  {typeLabel(t)}
                </button>
              ))}
            </div>
            <label className="search">
              <span className="search__icon" aria-hidden>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M10.5 18a7.5 7.5 0 100-15 7.5 7.5 0 000 15zM16.5 16.5L21 21"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </span>
              <input
                className="search__input"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder={locale === "zh" ? "搜索标题 / Prompt / 输出示例" : "Search title / prompt / output example"}
              />
            </label>
          </div>

          <div className="prompt-lib__group-bar" role="tablist" aria-label={locale === "zh" ? "Prompt 组" : "Prompt groups"}>
            <button
              type="button"
              className={`prompt-lib__group-chip${activeFolderId === activeType ? " is-active" : ""}`}
              onClick={() => setActiveFolderId(activeType)}
            >
              <span>{locale === "zh" ? "全部" : "All"}</span>
              <span className="prompt-lib__group-count">{activeTypeItemCount}</span>
            </button>
            {activeGroups.map((group) => (
              <button
                key={group.id}
                type="button"
                className={`prompt-lib__group-chip${activeFolderId === group.id ? " is-active" : ""}`}
                onClick={() => setActiveFolderId(group.id)}
                title={group.name}
              >
                <span className="prompt-lib__group-name">{group.name}</span>
                <span className="prompt-lib__group-count">{groupCounts.get(group.id) ?? 0}</span>
              </button>
            ))}
            <button
              type="button"
              className="prompt-lib__group-chip prompt-lib__group-chip--add"
              disabled={loading || saving}
              onClick={() => openNewGroupModal(activeType, null)}
            >
              {locale === "zh" ? "+ 新建组" : "+ New group"}
            </button>
          </div>
        </div>
      </div>

      <div className="prompt-lib__layout">
        <section className="prompt-lib__browse">
          {loading ? (
            <p className="muted">{locale === "zh" ? "正在加载 Prompt 库…" : "Loading prompt library…"}</p>
          ) : filteredItems.length === 0 ? (
            <p className="muted">{locale === "zh" ? "当前分类暂无收藏" : "No items in this category"}</p>
          ) : (
            <div className="prompt-lib__masonry">
              {masonryColumns.map((columnItems, columnIndex) => (
                <div key={`col-${columnIndex}`} className="prompt-lib__masonry-col">
                  {columnItems.map((item) => (
                    <article
                      key={item.id}
                      className={`prompt-lib__masonry-card${item.type === "image" ? "" : " prompt-lib__masonry-card--text-output"}`}
                      onContextMenu={(e) => onMasonryCardContextMenu(e, item)}
                    >
                      <div
                        className="prompt-lib__masonry-preview-hit"
                        role="button"
                        tabIndex={0}
                        aria-label={locale === "zh" ? "编辑此收藏" : "Edit this item"}
                        onClick={(e) => {
                          e.stopPropagation();
                          if (!loading && !saving) openEditModal(item);
                        }}
                        onKeyDown={(e) => {
                          if (e.key !== "Enter" && e.key !== " ") return;
                          e.preventDefault();
                          if (!loading && !saving) openEditModal(item);
                        }}
                      >
                        <MasonryCardOutput item={item} locale={locale} />
                      </div>
                      <div className="prompt-lib__masonry-body">
                        <div className="prompt-lib__masonry-main">
                          <h3 className="prompt-lib__masonry-title" title={item.title}>
                            {item.title}
                          </h3>
                          <PromptPublishBadges item={item} locale={locale} />
                        </div>
                        <button
                          type="button"
                          className="prompt-lib__masonry-copy"
                          onClick={(e) => {
                            e.stopPropagation();
                            const p = item.prompt.trim();
                            if (item.type === "image" && !p && item.imageDataUrl) {
                              void copyImageDataUrl(item.imageDataUrl);
                              return;
                            }
                            void copyPrompt(item.prompt);
                          }}
                        >
                          {item.type === "image" && !item.prompt.trim()
                            ? locale === "zh"
                              ? "复制图片"
                              : "Copy image"
                            : locale === "zh"
                              ? "复制 Prompt"
                              : "Copy prompt"}
                        </button>
                      </div>
                    </article>
                  ))}
                </div>
              ))}
            </div>
          )}
        </section>
      </div>

      {showCreate
        ? createPortal(
            <div className="prompt-create-modal-root">
              <div
                className="prompt-create-modal-backdrop"
                onClick={() => closeCreateModal()}
                aria-hidden
              />
              <div
                className="prompt-create-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="prompt-editor-title"
              >
                <header className="prompt-create-modal__header">
                  <div className="prompt-create-modal__header-text">
                    <h2 id="prompt-editor-title" className="prompt-create-modal__title">
                      {editingItemId
                        ? locale === "zh"
                          ? "编辑收藏"
                          : "Edit item"
                        : locale === "zh"
                          ? "新建收藏"
                          : "New item"}
                    </h2>
                    <p className="prompt-create-modal__subtitle">
                      {editingItemId
                        ? locale === "zh"
                          ? "修改标题、Prompt、输出类型或示例后保存。"
                          : "Update title, prompt, output type or example, then save."
                        : locale === "zh"
                          ? "保存输出示例与相关信息，便于复制与对照。"
                          : "Save examples and metadata for quick reuse."}
                    </p>
                  </div>
                  <button
                    type="button"
                    className="prompt-create-modal__close"
                    onClick={() => closeCreateModal()}
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

                <form
                  className="prompt-create-modal__form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    void onSubmitEditor();
                  }}
                >
                  <div className="prompt-create-modal__body">
                    <div className="prompt-create-modal__cluster">
                      <label className="prompt-create-modal__field" htmlFor="pcm-title">
                        <span className="prompt-create-modal__label">{locale === "zh" ? "标题" : "Title"}</span>
                        <input
                          ref={createTitleInputRef}
                          id="pcm-title"
                          className="prompt-create-modal__input"
                          value={newItem.title}
                          onChange={(e) => setNewItem((v) => ({ ...v, title: e.target.value }))}
                          placeholder={locale === "zh" ? "简要命名这条收藏" : "Give this item a concise title"}
                          autoComplete="off"
                        />
                      </label>
                      <div className="prompt-create-modal__field prompt-create-modal__field--flush">
                        <span className="prompt-create-modal__label" id="pcm-output-type-label">
                          {locale === "zh" ? "输出类型" : "Output type"}
                        </span>
                        <div
                          className="prompt-create-modal__type-row"
                          role="radiogroup"
                          aria-labelledby="pcm-output-type-label"
                        >
                          {PROMPT_TYPES.map((t) => (
                            <button
                              key={t}
                              type="button"
                              role="radio"
                              aria-checked={newItem.outputType === t}
                              className={`prompt-create-modal__type-pill${
                                newItem.outputType === t ? " is-active" : ""
                              }`}
                              onClick={() => {
                                setNewItem((v) => {
                                  if (t === "image") return { ...v, outputType: t, outputExample: "" };
                                  const keepExample = v.outputType !== "image";
                                  return {
                                    ...v,
                                    outputType: t,
                                    outputExample: keepExample ? v.outputExample : "",
                                  };
                                });
                                if (t === "image") {
                                  setNewOutputImageDataUrl((cur) => {
                                    if (cur) return cur;
                                    const ed = editingItemId
                                      ? library.items.find((x) => x.id === editingItemId)
                                      : undefined;
                                    return ed?.imageDataUrl ?? null;
                                  });
                                } else {
                                  setNewOutputImageDataUrl(null);
                                }
                              }}
                            >
                              <span className="prompt-create-modal__type-icon" aria-hidden>
                                <OutputTypeGlyph type={t} />
                              </span>
                              <span className="prompt-create-modal__type-label">{typeLabel(t)}</span>
                            </button>
                          ))}
                        </div>
                      </div>
                    </div>

                    <div className="prompt-create-modal__rule" role="presentation" />

                    <label className="prompt-create-modal__field" htmlFor="pcm-prompt">
                      <span className="prompt-create-modal__label">Prompt</span>
                      <textarea
                        id="pcm-prompt"
                        className="prompt-create-modal__textarea prompt-create-modal__textarea--prompt"
                        rows={6}
                        value={newItem.prompt}
                        onChange={(e) => setNewItem((v) => ({ ...v, prompt: e.target.value }))}
                        placeholder={locale === "zh" ? "完整指令内容" : "Full prompt content"}
                        spellCheck={false}
                      />
                    </label>

                    <div className="prompt-create-modal__rule" role="presentation" />

                    {newItem.outputType === "image" ? (
                      <div className="prompt-create-modal__field">
                        <span className="prompt-create-modal__label" id="pcm-image-example-label">
                          {locale === "zh" ? "输出示例（图片）" : "Output example (image)"}{" "}
                          <span className="prompt-create-modal__label-optional">
                            {locale === "zh" ? "选填" : "optional"}
                          </span>
                        </span>
                        <div
                          className={`prompt-create-modal__paste-board${
                            newOutputImageDataUrl ? " has-preview" : ""
                          }`}
                        >
                          <textarea
                            className="prompt-create-modal__paste-target"
                            rows={2}
                            placeholder={
                              locale === "zh"
                                ? "聚焦后粘贴截图（⌘V / Ctrl+V）"
                                : "Focus here and paste screenshot (⌘V / Ctrl+V)"
                            }
                            onPaste={(e) => void onPasteOutputExample(e)}
                            aria-labelledby="pcm-image-example-label"
                          />
                          {!newOutputImageDataUrl ? (
                            <p className="prompt-create-modal__paste-hint">
                              {locale === "zh"
                                ? "支持从浏览器或设计工具粘贴；体积过大会自动压缩。"
                                : "Paste from browser/design tools; oversized images are compressed automatically."}
                            </p>
                          ) : null}
                        </div>
                        {newOutputImageDataUrl ? (
                          <div className="prompt-create-modal__preview-wrap">
                            <img
                              src={newOutputImageDataUrl}
                              alt={locale === "zh" ? "已粘贴的输出示例预览" : "Pasted output example preview"}
                              className="prompt-create-modal__preview-img"
                            />
                          </div>
                        ) : null}
                      </div>
                    ) : (
                      <label className="prompt-create-modal__field" htmlFor="pcm-output-example">
                        <span className="prompt-create-modal__label">
                          {locale === "zh" ? "输出示例" : "Output example"}{" "}
                          <span className="prompt-create-modal__label-optional">
                            {locale === "zh" ? "选填" : "optional"}
                          </span>
                        </span>
                        <textarea
                          id="pcm-output-example"
                          className="prompt-create-modal__textarea"
                          rows={4}
                          value={newItem.outputExample}
                          onChange={(e) => setNewItem((v) => ({ ...v, outputExample: e.target.value }))}
                          placeholder={
                            locale === "zh"
                              ? "一段代表性的文本、代码或文档片段"
                              : "A representative text, code, or document snippet"
                          }
                          spellCheck={false}
                        />
                      </label>
                    )}

                    <div className="prompt-create-modal__rule" role="presentation" />

                    <label className="prompt-create-modal__field" htmlFor="pcm-link">
                      <span className="prompt-create-modal__label">
                        {locale === "zh" ? "相关链接" : "Related link"}{" "}
                        <span className="prompt-create-modal__label-optional">
                          {locale === "zh" ? "选填" : "optional"}
                        </span>
                      </span>
                      <input
                        id="pcm-link"
                        className="prompt-create-modal__input"
                        value={newItem.relatedLink}
                        onChange={(e) => setNewItem((v) => ({ ...v, relatedLink: e.target.value }))}
                        placeholder="https://"
                        inputMode="url"
                        autoComplete="off"
                      />
                    </label>
                  </div>

                  <footer className="prompt-create-modal__footer">
                    <span className="prompt-create-modal__kbd-hint">
                      {locale === "zh" ? "Esc 关闭" : "Esc to close"}
                    </span>
                    <div className="prompt-create-modal__actions">
                      <button
                        type="button"
                        className="prompt-create-modal__cancel"
                        onClick={() => closeCreateModal()}
                        disabled={saving}
                      >
                        {locale === "zh" ? "取消" : "Cancel"}
                      </button>
                      <button type="submit" className="prompt-create-modal__submit" disabled={saving}>
                        {locale === "zh" ? "保存" : "Save"}
                      </button>
                    </div>
                  </footer>
                </form>
              </div>
            </div>,
            document.body,
          )
        : null}

      {cardContextMenu
        ? createPortal(
            <div
              ref={cardContextMenuRef}
              className="card-context-menu"
              style={{
                position: "fixed",
                left: cardContextMenu.x,
                top: cardContextMenu.y,
                zIndex: 10_000,
              }}
              role="menu"
              aria-label={locale === "zh" ? "收藏操作" : "Item actions"}
            >
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                disabled={saving}
                onClick={() => {
                  const item = cardContextMenu.item;
                  setCardContextMenu(null);
                  openEditModal(item);
                }}
              >
                {locale === "zh" ? "编辑" : "Edit"}
              </button>
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                disabled={saving}
                onClick={() => {
                  const item = cardContextMenu.item;
                  setCardContextMenu(null);
                  setGroupPicker({ itemId: item.id });
                }}
              >
                {locale === "zh" ? "添加到组…" : "Add to group..."}
              </button>
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                disabled={saving}
                onClick={() => {
                  const item = cardContextMenu.item;
                  setCardContextMenu(null);
                  const p = item.prompt.trim();
                  if (item.type === "image" && !p && item.imageDataUrl) {
                    void copyImageDataUrl(item.imageDataUrl);
                    return;
                  }
                  void copyPrompt(item.prompt);
                }}
              >
                {cardContextMenu.item.type === "image" && !cardContextMenu.item.prompt.trim()
                  ? locale === "zh"
                    ? "复制图片"
                    : "Copy image"
                  : locale === "zh"
                    ? "复制 Prompt"
                    : "Copy prompt"}
              </button>
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                disabled={saving}
                onClick={() => {
                  const item = cardContextMenu.item;
                  setCardContextMenu(null);
                  openPromptCommandEditor(item);
                }}
              >
                {cardContextMenu.item.commandEnabled
                  ? locale === "zh"
                    ? "修改 /cp 命令"
                    : "Edit /cp command"
                  : locale === "zh"
                    ? "发布 /cp"
                    : "Publish /cp"}
              </button>
              {cardContextMenu.item.commandEnabled && cardContextMenu.item.commandName ? (
                <>
                  <button
                    type="button"
                    role="menuitem"
                    className="card-context-menu__item"
                    disabled={saving}
                    onClick={() => {
                      const commandName = cardContextMenu.item.commandName ?? "";
                      setCardContextMenu(null);
                      void copyPromptCommand(commandName);
                    }}
                  >
                    {locale === "zh" ? "复制 /cp 命令" : "Copy /cp command"}
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    className="card-context-menu__item"
                    disabled={saving}
                    onClick={() => {
                      const item = cardContextMenu.item;
                      setCardContextMenu(null);
                      void unpublishPromptCommand(item);
                    }}
                  >
                    {locale === "zh" ? "取消发布 /cp" : "Unpublish /cp"}
                  </button>
                </>
              ) : null}
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                disabled={saving}
                onClick={() => {
                  const item = cardContextMenu.item;
                  setCardContextMenu(null);
                  openSkillConvertEditor(item);
                }}
              >
                {cardContextMenu.item.convertedSkillId
                  ? locale === "zh"
                    ? "再次转为 Skill"
                    : "Convert to Skill again"
                  : locale === "zh"
                    ? "转为 Skill"
                    : "Convert to Skill"}
              </button>
              {cardContextMenu.item.type !== "image" && cardContextMenu.item.outputExample?.trim() ? (
                <button
                  type="button"
                  role="menuitem"
                  className="card-context-menu__item"
                  disabled={saving}
                  onClick={() => {
                    const ex = cardContextMenu.item.outputExample ?? "";
                    setCardContextMenu(null);
                    void copyOutputExample(ex);
                  }}
                >
                  {locale === "zh" ? "复制输出示例" : "Copy output example"}
                </button>
              ) : null}
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item card-context-menu__item--danger"
                disabled={saving}
                onClick={() => {
                  const id = cardContextMenu.item.id;
                  setCardContextMenu(null);
                  void onDeleteItem(id);
                }}
              >
                {locale === "zh" ? "删除" : "Delete"}
              </button>
            </div>,
            document.body,
          )
        : null}

      {groupPicker
        ? (() => {
            const item = library.items.find((x) => x.id === groupPicker.itemId);
            if (!item) return null;
            const groups = library.folders
              .filter((folder) => folder.parentId === item.type)
              .slice()
              .sort((a, b) => a.name.localeCompare(b.name));
            return createPortal(
              <div className="prompt-create-modal-root">
                <div
                  className="prompt-create-modal-backdrop"
                  onClick={() => setGroupPicker(null)}
                  aria-hidden
                />
                <div
                  className="prompt-create-modal prompt-command-modal"
                  role="dialog"
                  aria-modal="true"
                  aria-labelledby="prompt-group-picker-title"
                >
                  <header className="prompt-create-modal__header">
                    <div className="prompt-create-modal__header-text">
                      <h2 id="prompt-group-picker-title" className="prompt-create-modal__title">
                        {locale === "zh" ? "添加到组" : "Add to group"}
                      </h2>
                      <p className="prompt-create-modal__subtitle">
                        {locale === "zh"
                          ? `为「${item.title}」选择一个${typeLabel(item.type)}组。`
                          : `Choose a ${typeLabel(item.type)} group for "${item.title}".`}
                      </p>
                    </div>
                    <button
                      type="button"
                      className="prompt-create-modal__close"
                      onClick={() => setGroupPicker(null)}
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
                    <div className="prompt-lib__group-picker">
                      <button
                        type="button"
                        className={`prompt-lib__group-picker-item${
                          item.folderId === item.type ? " is-active" : ""
                        }`}
                        disabled={saving}
                        onClick={() => void assignItemToGroup(item, item.type)}
                      >
                        <span>{locale === "zh" ? "全部（不放入组）" : "All (no group)"}</span>
                        <span className="prompt-lib__group-count">
                          {library.items.filter((x) => x.type === item.type && x.folderId === item.type).length}
                        </span>
                      </button>
                      {groups.map((group) => (
                        <button
                          key={group.id}
                          type="button"
                          className={`prompt-lib__group-picker-item${
                            item.folderId === group.id ? " is-active" : ""
                          }`}
                          disabled={saving}
                          onClick={() => void assignItemToGroup(item, group.id)}
                        >
                          <span>{group.name}</span>
                          <span className="prompt-lib__group-count">{groupCounts.get(group.id) ?? 0}</span>
                        </button>
                      ))}
                    </div>
                  </div>
                  <footer className="prompt-create-modal__footer">
                    <button
                      type="button"
                      className="prompt-create-modal__cancel"
                      onClick={() => setGroupPicker(null)}
                      disabled={saving}
                    >
                      {locale === "zh" ? "取消" : "Cancel"}
                    </button>
                    <button
                      type="button"
                      className="prompt-create-modal__submit"
                      onClick={() => {
                        setGroupPicker(null);
                        openNewGroupModal(item.type, item.id);
                      }}
                      disabled={saving}
                    >
                      {locale === "zh" ? "+ 新建组并添加" : "+ New group and add"}
                    </button>
                  </footer>
                </div>
              </div>,
              document.body,
            );
          })()
        : null}

      {groupNameModal
        ? createPortal(
            <div className="prompt-create-modal-root">
              <div
                className="prompt-create-modal-backdrop"
                onClick={() => {
                  setGroupNameModal(null);
                  setNewGroupNameDraft("");
                }}
                aria-hidden
              />
              <div
                className="prompt-create-modal prompt-command-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="prompt-new-group-title"
              >
                <header className="prompt-create-modal__header">
                  <div className="prompt-create-modal__header-text">
                    <h2 id="prompt-new-group-title" className="prompt-create-modal__title">
                      {locale === "zh" ? "新建组" : "New group"}
                    </h2>
                    <p className="prompt-create-modal__subtitle">
                      {groupNameModal.assignItemId
                        ? locale === "zh"
                          ? `在「${typeLabel(groupNameModal.promptType)}」下创建，并将当前卡片加入该组。`
                          : `Create under ${typeLabel(groupNameModal.promptType)} and add the selected item.`
                        : locale === "zh"
                          ? `组将出现在「${typeLabel(groupNameModal.promptType)}」分类下。`
                          : `The group appears under ${typeLabel(groupNameModal.promptType)}.`}
                    </p>
                  </div>
                  <button
                    type="button"
                    className="prompt-create-modal__close"
                    onClick={() => {
                      setGroupNameModal(null);
                      setNewGroupNameDraft("");
                    }}
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
                <form
                  className="prompt-create-modal__form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    void confirmNewGroup();
                  }}
                >
                  <div className="prompt-create-modal__body">
                    <label className="prompt-create-modal__field" htmlFor="prompt-new-group-name">
                      <span className="prompt-create-modal__label">
                        {locale === "zh" ? "组名" : "Group name"}
                      </span>
                      <input
                        ref={newGroupNameInputRef}
                        id="prompt-new-group-name"
                        className="prompt-create-modal__input"
                        value={newGroupNameDraft}
                        onChange={(e) => setNewGroupNameDraft(e.target.value)}
                        placeholder={locale === "zh" ? "例如：工作流 / 海报" : "e.g. workflow / posters"}
                        autoComplete="off"
                      />
                    </label>
                  </div>
                  <footer className="prompt-create-modal__footer">
                    <span className="prompt-create-modal__kbd-hint">
                      {locale === "zh" ? "Esc 关闭" : "Esc to close"}
                    </span>
                    <div className="prompt-create-modal__actions">
                      <button
                        type="button"
                        className="prompt-create-modal__cancel"
                        onClick={() => {
                          setGroupNameModal(null);
                          setNewGroupNameDraft("");
                        }}
                        disabled={saving}
                      >
                        {locale === "zh" ? "取消" : "Cancel"}
                      </button>
                      <button type="submit" className="prompt-create-modal__submit" disabled={saving}>
                        {locale === "zh" ? "创建" : "Create"}
                      </button>
                    </div>
                  </footer>
                </form>
              </div>
            </div>,
            document.body,
          )
        : null}

      {commandEditor
        ? createPortal(
            <div className="prompt-create-modal-root">
              <div
                className="prompt-create-modal-backdrop"
                onClick={() => setCommandEditor(null)}
                aria-hidden
              />
              <div
                className="prompt-create-modal prompt-command-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="prompt-command-title"
              >
                <header className="prompt-create-modal__header">
                  <div className="prompt-create-modal__header-text">
                    <h2 id="prompt-command-title" className="prompt-create-modal__title">
                      {locale === "zh" ? "发布 /cp 命令" : "Publish /cp command"}
                    </h2>
                    <p className="prompt-create-modal__subtitle">
                      {locale === "zh"
                        ? "命令名不需要输入 /cp-，保存后会显示为 /cp-xxx。"
                        : "Enter the name without /cp-. It will be shown as /cp-xxx."}
                    </p>
                  </div>
                  <button
                    type="button"
                    className="prompt-create-modal__close"
                    onClick={() => setCommandEditor(null)}
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
                <form
                  className="prompt-create-modal__form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    const item = library.items.find((x) => x.id === commandEditor.itemId);
                    if (!item) {
                      setCommandEditor(null);
                      return;
                    }
                    void publishPromptCommand(item, commandEditor.commandName);
                  }}
                >
                  <div className="prompt-create-modal__body">
                    <label className="prompt-create-modal__field" htmlFor="prompt-command-name">
                      <span className="prompt-create-modal__label">
                        {locale === "zh" ? "命令名" : "Command name"}
                      </span>
                      <div className="prompt-command-modal__input-row">
                        <span className="prompt-command-modal__prefix">/cp-</span>
                        <input
                          id="prompt-command-name"
                          className="prompt-create-modal__input"
                          value={commandEditor.commandName}
                          onChange={(e) =>
                            setCommandEditor((cur) =>
                              cur ? { ...cur, commandName: e.target.value } : cur,
                            )
                          }
                          placeholder="prd-review"
                          autoComplete="off"
                          autoFocus
                        />
                      </div>
                    </label>
                  </div>
                  <footer className="prompt-create-modal__footer">
                    <span className="prompt-create-modal__kbd-hint">
                      {displayPromptCommand(slugifyCommandName(commandEditor.commandName))}
                    </span>
                    <div className="prompt-create-modal__actions">
                      <button
                        type="button"
                        className="prompt-create-modal__cancel"
                        onClick={() => setCommandEditor(null)}
                        disabled={saving}
                      >
                        {locale === "zh" ? "取消" : "Cancel"}
                      </button>
                      <button
                        type="submit"
                        className="prompt-create-modal__submit"
                        disabled={
                          saving ||
                          !isValidAgentCommandSegmentInput(commandEditor.commandName)
                        }
                      >
                        {locale === "zh" ? "发布" : "Publish"}
                      </button>
                    </div>
                  </footer>
                </form>
              </div>
            </div>,
            document.body,
          )
        : null}

      {skillConvertEditor
        ? createPortal(
            <div className="prompt-create-modal-root">
              <div
                className="prompt-create-modal-backdrop"
                onClick={() => setSkillConvertEditor(null)}
                aria-hidden
              />
              <div
                className="prompt-create-modal prompt-command-modal"
                role="dialog"
                aria-modal="true"
                aria-labelledby="prompt-skill-title"
              >
                <header className="prompt-create-modal__header">
                  <div className="prompt-create-modal__header-text">
                    <h2 id="prompt-skill-title" className="prompt-create-modal__title">
                      {locale === "zh" ? "转为 Skill" : "Convert to Skill"}
                    </h2>
                    <p className="prompt-create-modal__subtitle">
                      {locale === "zh"
                        ? "Skill 名不需要输入 cps-，生成后会作为 /cps-xxx 同步到 Agent。"
                        : "Enter the name without cps-. It will be generated as /cps-xxx for agents."}
                    </p>
                  </div>
                  <button
                    type="button"
                    className="prompt-create-modal__close"
                    onClick={() => setSkillConvertEditor(null)}
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
                <form
                  className="prompt-create-modal__form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    const item = library.items.find((x) => x.id === skillConvertEditor.itemId);
                    if (!item) {
                      setSkillConvertEditor(null);
                      return;
                    }
                    void convertPromptItemToSkill(item, skillConvertEditor.skillName);
                  }}
                >
                  <div className="prompt-create-modal__body">
                    <label className="prompt-create-modal__field" htmlFor="prompt-skill-name">
                      <span className="prompt-create-modal__label">
                        {locale === "zh" ? "Skill 名" : "Skill name"}
                      </span>
                      <div className="prompt-command-modal__input-row">
                        <span className="prompt-command-modal__prefix">/cps-</span>
                        <input
                          id="prompt-skill-name"
                          className="prompt-create-modal__input"
                          value={skillConvertEditor.skillName}
                          onChange={(e) =>
                            setSkillConvertEditor((cur) =>
                              cur ? { ...cur, skillName: e.target.value } : cur,
                            )
                          }
                          placeholder="prd-review"
                          autoComplete="off"
                          autoFocus
                        />
                      </div>
                    </label>
                  </div>
                  <footer className="prompt-create-modal__footer">
                    <span className="prompt-create-modal__kbd-hint">
                      {`/cps-${slugifyCommandName(skillConvertEditor.skillName)}`}
                    </span>
                    <div className="prompt-create-modal__actions">
                      <button
                        type="button"
                        className="prompt-create-modal__cancel"
                        onClick={() => setSkillConvertEditor(null)}
                        disabled={saving}
                      >
                        {locale === "zh" ? "取消" : "Cancel"}
                      </button>
                      <button
                        type="submit"
                        className="prompt-create-modal__submit"
                        disabled={
                          saving ||
                          !isValidAgentCommandSegmentInput(skillConvertEditor.skillName)
                        }
                      >
                        {locale === "zh" ? "生成 Skill" : "Create Skill"}
                      </button>
                    </div>
                  </footer>
                </form>
              </div>
            </div>,
            document.body,
          )
        : null}

      {err ? <p className="error">{err}</p> : null}
      {toast ? (
        <div className="toast-stack">
          <div className={`toast ${toast.kind === "error" ? "toast--error" : "toast--success"}`}>
            <span className="toast__text">{toast.message}</span>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function PromptPublishBadges({ item, locale }: { item: PromptItem; locale: "zh" | "en" }) {
  const commandName = item.commandEnabled && item.commandName ? item.commandName : null;
  const converted = Boolean(item.convertedSkillId);
  if (!commandName && !converted) {
    return (
      <div className="prompt-lib__publish-badges" aria-label={locale === "zh" ? "发布状态" : "Publish status"}>
        <span className="prompt-lib__publish-badge prompt-lib__publish-badge--muted">
          {locale === "zh" ? "未发布" : "Unpublished"}
        </span>
      </div>
    );
  }
  return (
    <div className="prompt-lib__publish-badges" aria-label={locale === "zh" ? "发布状态" : "Publish status"}>
      {commandName ? (
        <span className="prompt-lib__publish-badge prompt-lib__publish-badge--cp">
          {displayPromptCommand(commandName)}
        </span>
      ) : null}
      {converted ? (
        <span className="prompt-lib__publish-badge prompt-lib__publish-badge--skill">
          {locale === "zh" ? "已转 Skill" : "Skill"}
        </span>
      ) : null}
    </div>
  );
}

function MasonryCardOutput({ item, locale }: { item: PromptItem; locale: "zh" | "en" }) {
  if (item.type === "image") {
    return item.imageDataUrl ? (
      <img src={item.imageDataUrl} alt={item.title} className="prompt-lib__masonry-image" />
    ) : (
      <div className="prompt-lib__masonry-fallback">{locale === "zh" ? "暂无图片" : "No image"}</div>
    );
  }
  const example = (item.outputExample ?? "").trim();
  const promptBody = (item.prompt ?? "").trim();
  const raw = example || promptBody;
  if (!raw) {
    return (
      <div className="prompt-lib__masonry-fallback">
        {locale === "zh" ? "暂无 Prompt 与输出示例" : "No prompt or output example"}
      </div>
    );
  }
  const kind = item.type === "code" ? "code" : item.type === "doc" ? "doc" : "text";
  return (
    <div className={`prompt-lib__masonry-output prompt-lib__masonry-output--${kind}`}>
      <pre className={`prompt-lib__masonry-text-pre prompt-lib__masonry-text-pre--${kind}`}>{raw}</pre>
    </div>
  );
}

function OutputTypeGlyph({ type }: { type: PromptType }) {
  const stroke = "currentColor" as const;
  const sw = 1.65;
  switch (type) {
    case "image":
      return (
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden>
          <rect x="3" y="5" width="18" height="14" rx="2" fill="none" stroke={stroke} strokeWidth={sw} />
          <circle cx="8.5" cy="10" r="1.5" fill={stroke} />
          <path
            d="M3 17l5.5-5.5a1.5 1.5 0 012.1 0L15 16l2.5-2.5a1.5 1.5 0 012.1 0L21 15"
            fill="none"
            stroke={stroke}
            strokeWidth={sw}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      );
    case "code":
      return (
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden>
          <path
            d="M8 8l-3.5 4L8 16M16 8l3.5 4L16 16"
            fill="none"
            stroke={stroke}
            strokeWidth={sw}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      );
    case "doc":
      return (
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden>
          <path
            d="M7 3h7l5 5v13a1 1 0 01-1 1H7a1 1 0 01-1-1V4a1 1 0 011-1z"
            fill="none"
            stroke={stroke}
            strokeWidth={sw}
            strokeLinejoin="round"
          />
          <path d="M14 3v4h4M8 13h8M8 16.5h8M8 10h5" fill="none" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
        </svg>
      );
    case "text":
      return (
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden>
          <path d="M6 6h12M6 10h12M6 14h9M6 18h11" fill="none" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
        </svg>
      );
    default:
      return null;
  }
}

function fileToDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(new Error("read failed"));
    reader.readAsDataURL(file);
  });
}

function dataUrlToBlob(dataUrl: string): Blob {
  const m = dataUrl.match(/^data:([^;]+);base64,(.*)$/);
  if (!m) return new Blob([dataUrl], { type: "text/plain" });
  const mime = m[1] || "application/octet-stream";
  const b64 = m[2] || "";
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) bytes[i] = bin.charCodeAt(i);
  return new Blob([bytes], { type: mime });
}

function dataUrlToImage(dataUrl: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("image load failed"));
    img.src = dataUrl;
  });
}

async function compressDataUrlToMax(dataUrl: string, maxBytes: number): Promise<string> {
  const img = await dataUrlToImage(dataUrl);
  const canvas = document.createElement("canvas");
  const maxEdge = 1600;
  const scale = Math.min(1, maxEdge / Math.max(img.width, img.height));
  canvas.width = Math.max(1, Math.round(img.width * scale));
  canvas.height = Math.max(1, Math.round(img.height * scale));
  const ctx = canvas.getContext("2d");
  if (!ctx) return dataUrl;
  ctx.drawImage(img, 0, 0, canvas.width, canvas.height);

  let quality = 0.9;
  let out = canvas.toDataURL("image/webp", quality);
  while (quality > 0.4) {
    const idx = out.indexOf(",");
    const b64 = idx >= 0 ? out.slice(idx + 1) : "";
    const padding = (b64.match(/=*$/)?.[0].length ?? 0);
    const bytes = Math.max(0, (b64.length * 3) / 4 - padding);
    if (bytes <= maxBytes) return out;
    quality -= 0.1;
    out = canvas.toDataURL("image/webp", quality);
  }
  return out;
}
