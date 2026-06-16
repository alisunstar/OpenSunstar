import {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import { createPortal } from "react-dom";
import { useSearchParams } from "react-router-dom";
import {
  deepseekClassifyInventory,
  deepseekRegenerateCategories,
  deepseekReclassifyWithCategories,
  deepseekResummarizeAsset,
  deepseekSummarizeInventory,
  deepseekTranslateCustomCategories,
  getCustomCategories as loadCustomCategoriesFromStorage,
  getDeepseekSettings,
  resetAllCategories,
  type CustomCategory,
} from "../api/deepseek";
import {
  getAgentGlobalInventoryCached,
  invalidateCachedAgentGlobalInventory,
  invalidateCachedProjectInventory,
  scanProjectDirectoryCached,
} from "../api/agentInventoryCache";
import {
  copySkillPackage,
  deleteSkillAtPath,
  listDetectedAgents,
  type AgentInventory,
  type AssetEntry,
} from "../api/agents";
import {
  addSkillToMyLibrary,
  getMySkillsLibrary,
  removeMySkill,
  type MySkillItem,
  type MySkillsLibraryFile,
} from "../api/mySkills";
import {
  applyPromptCommandToAgent,
  getPromptLibrary,
  type PromptItem,
  type PromptLibraryFile,
} from "../api/prompts";
import {
  agentCommandSegmentInvalidMessage,
  isValidAgentCommandSegmentInput,
  normalizePromptApplyCommandSegment,
} from "../agentCommandInput";
import { useProjectPaths } from "../projectPathsStorage";
import { PageRefreshButton } from "../components/PageRefreshButton";
import { SkillCopyDestinationDialog } from "../components/SkillCopyDestinationDialog";
import { SkillDetailPanel, type DetailEntry } from "../components/SkillDetailPanel";
import {
  getScenarioHint,
  getScenarioLabel,
  rowMatchesScenarioChip,
  rowMatchesCustomScenario,
  SCENARIO_ORDER,
  type ScenarioKey,
} from "../skillScenarioCategories";
import {
  bucketInventoryByAgent,
  filterInventoryForAgent,
  inferAgentIdFromAssetPath,
  inventoryAssetCount,
} from "../agentAssetGrouping";
import { revealPathInFolder } from "../api/reveal";
import {
  buildCopySkillMenuSections,
  buildPromptApplyMenuSections,
  type PromptApplyAgentTarget,
} from "../skillCopyTargets";
import { useI18n } from "../i18n/provider";
import { open } from "@tauri-apps/plugin-dialog";

type AssetKind = "skill" | "mcp" | "rule";
type BrowseKind = AssetKind | "prompt";

type BrowseRow = {
  id: string;
  sourceId: string;
  title: string;
  desc: string;
  descSource: "ai" | "source";
  kind: BrowseKind;
  ecosystem: string;
  agentId?: string;
  tags: string[];
  active: boolean;
  /** 本机路径或占位 id */
  sourcePath?: string;
  /** 技能包内除主 SKILL.md 外的文件（与 AssetEntry.skill_extra_files 一致） */
  skillExtraFiles?: string[];
  /** AIControls 我的 Skills 暴露给 Agent 的 /cs 命令名 */
  csCommand?: string;
  /** AIControls 我的 Prompts 暴露出的 /cp 命令名 */
  cpCommand?: string;
  promptText?: string;
  promptCommandName?: string;
  mineSkillSourceKind?: "prompt" | null;
  /** DeepSeek 分类 slug；未命中时用关键词兜底 */
  scenario?: string | null;
};

type BrowseSection = {
  key: string;
  /** 空字符串：不展示分组标题（如「全部」汇总） */
  title: string;
  rows: BrowseRow[];
};

type AgentFilterOption = {
  id: string;
  label: string;
  count: number;
};

/** 与 App 侧栏 Agent 名称一致；在「全部」汇总页用于兜底扫描 */
const AGENT_LABEL_BY_ID: Record<string, string> = {
  cursor: "Cursor",
  claude: "Claude Code",
  codex: "Codex",
  hermes: "Hermes",
  openclaw: "OpenClaw",
  trae: "Trae",
  qoder: "Qoder",
  kiro: "Kiro",
  opencode: "opencode",
};

const FALLBACK_AGENT_IDS = [
  "cursor",
  "claude",
  "codex",
  "hermes",
  "openclaw",
  "trae",
  "qoder",
  "kiro",
  "opencode",
] as const;

function agentLabelForId(id: string, locale: "zh" | "en"): string {
  if (id === "__other__") return locale === "zh" ? "其他" : "Other";
  return AGENT_LABEL_BY_ID[id] ?? id;
}

function customCategoryLabel(cat: CustomCategory, locale: "zh" | "en"): string {
  if (locale === "zh") return cat.labelZh;
  const en = cat.labelEn?.trim();
  if (en) return en;
  return cat.slug
    .split(/[_-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "Project";
}

function normalizeSkillPathForCompare(path?: string | null): string {
  return (path ?? "").trim().replace(/\\/g, "/").replace(/\/+$/, "");
}

function normalizedPathBasename(path?: string | null): string {
  const normalized = normalizeSkillPathForCompare(path);
  if (!normalized) return "";
  return normalized.split("/").pop() ?? "";
}

function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function rowAppliedSkillFolderPrefixes(row: BrowseRow): string[] {
  const base = normalizedPathBasename(row.sourcePath) || slugifyCommandSegment(row.title);
  const prefixes =
    row.mineSkillSourceKind === "prompt"
      ? ["cps-"]
      : row.mineSkillSourceKind === null
        ? ["cs-", "cps-"]
        : ["cs-"];
  return prefixes.map((prefix) => `${prefix}${base}`.toLowerCase());
}

function rowMatchesAppliedSkill(row: BrowseRow, asset: AssetEntry): boolean {
  if (asset.kind !== "skill") return false;
  const folder = normalizedPathBasename(asset.path).toLowerCase();
  const title = asset.title.trim().toLowerCase();
  return rowAppliedSkillFolderPrefixes(row).some((prefix) => {
    const re = new RegExp(`^${escapeRegExp(prefix)}(?:-\\d+)?$`);
    return re.test(folder) || re.test(title);
  });
}

/** 列表里技能包为目录路径；散装 `SKILL.md` 以文件名结尾，只支持复制、不提供「删除文件夹」 */
function skillBrowsePathIsDeletableFolder(sourcePath: string): boolean {
  const t = sourcePath.trim().replace(/\\/g, "/");
  return !/\/SKILL\.md$/i.test(t);
}

function slugifyCommandSegment(input: string): string {
  const out = input
    .trim()
    .toLowerCase()
    .replace(/^\/?prompts:/, "")
    .replace(/^\/?(cps|cs|cp)-/, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .replace(/-{2,}/g, "-");
  return out || "skill";
}

function csCommandForSkill(title: string, sourcePath?: string): string {
  const folder = sourcePath?.trim() ? folderBasename(sourcePath) : "";
  const base = folder || title;
  return `/cs-${slugifyCommandSegment(base)}`;
}

function cpsCommandForSkill(title: string, sourcePath?: string): string {
  const folder = sourcePath?.trim() ? folderBasename(sourcePath) : "";
  const base = folder || title;
  return `/cps-${slugifyCommandSegment(base)}`;
}

function isCsSkillEntry(entry: AssetEntry): boolean {
  const title = entry.title.trim().toLowerCase();
  const folder = folderBasename(entry.path).trim().toLowerCase();
  return title.startsWith("cs-") || folder.startsWith("cs-") || title.startsWith("cps-") || folder.startsWith("cps-") || title.startsWith("cp-") || folder.startsWith("cp-");
}

function csCommandForInstalledSkill(row: BrowseRow): string {
  const title = row.title.trim().toLowerCase();
  const base = title.startsWith("cs-") || title.startsWith("cps-")
    ? row.title
    : folderBasename(row.sourcePath ?? row.title);
  const folder = folderBasename(row.sourcePath ?? "").toLowerCase();
  if (title.startsWith("cp-") || folder.startsWith("cp-")) {
    return `/cp-${slugifyCommandSegment(base)}`;
  }
  return title.startsWith("cps-") || folder.startsWith("cps-")
    ? `/cps-${slugifyCommandSegment(base)}`
    : `/cs-${slugifyCommandSegment(base)}`;
}

function codexPromptCommand(commandName: string): string {
  return `/prompts:cp-${slugifyCommandSegment(commandName)}`;
}

function promptCommandForInstalledPrompt(row: BrowseRow): string {
  const title = row.title.trim().toLowerCase();
  const folder = folderBasename(row.sourcePath ?? "").toLowerCase();
  const base = title.startsWith("cp-") || folder.startsWith("cp-")
    ? row.title
    : folderBasename(row.sourcePath ?? row.title);
  if (row.ecosystem === "codex") {
    return codexPromptCommand(base);
  }
  return `/cp-${slugifyCommandSegment(base)}`;
}

function cpCommandForPrompt(item: PromptItem): string | null {
  if (!item.commandEnabled || !item.commandName?.trim()) return null;
  return `/cp-${slugifyCommandSegment(item.commandName)}`;
}

function promptDescriptionForCard(item: PromptItem, fallback: string): string {
  return (
    item.prompt.trim() ||
    item.note?.trim() ||
    item.outputExample?.trim() ||
    fallback
  );
}

function mineAssetEntryFromSkill(item: MySkillItem): AssetEntry {
  return {
    id: item.id,
    kind: "skill",
    title: item.title,
    description: item.description,
    path: item.path,
    active: true,
    scenario: null,
    brief_zh: null,
    brief_en: null,
    skill_extra_files: null,
  };
}

function mineAssetEntryFromPrompt(item: PromptItem, command: string): AssetEntry {
  return {
    id: item.id,
    kind: "prompt",
    title: item.title,
    description: promptDescriptionForCard(item, command),
    path: command,
    active: true,
    scenario: null,
    brief_zh: null,
    brief_en: null,
    skill_extra_files: null,
  };
}

function mineLibrariesToInventory(
  mySkillsLib: MySkillsLibraryFile | null,
  promptLibrary: PromptLibraryFile | null,
): AgentInventory {
  const skills = [
    ...(mySkillsLib?.items ?? []).map(mineAssetEntryFromSkill),
    ...(promptLibrary?.items ?? []).flatMap((item) => {
      const command = cpCommandForPrompt(item);
      return command ? [mineAssetEntryFromPrompt(item, command)] : [];
    }),
  ];
  return { skills, mcp: [], rules: [] };
}

/** HTML `id` 安全片段（来自路径等分组 key） */
function sectionIdSafeFragment(sectionKey: string): string {
  const s = sectionKey.replace(/\W/g, "_");
  return s.length > 0 ? s : "sec";
}

type AggregateSnapshot = {
  agents: { id: string; title: string; inv: AgentInventory | null }[];
  projects: { path: string; inv: AgentInventory | null }[];
  anyInventoryFailed: boolean;
};

type AggregateSnapshotCache = {
  version: 2;
  savedAt: number;
  projectSignature: string;
  snapshot: AggregateSnapshot;
  mySkillsLibrary: MySkillsLibraryFile | null;
  promptLibrary: PromptLibraryFile | null;
};

const AGGREGATE_SNAPSHOT_CACHE_KEY = "aicontrols.aggregateSnapshot.v2";
const AGGREGATE_SNAPSHOT_CACHE_TTL_MS = 10 * 60 * 1000;
const AGGREGATE_RENDER_PAGE_SIZE = 12;
const MY_SKILL_APPLIED_LOCATIONS_KEY = "aicontrols.mySkillAppliedLocations.v1";

type MySkillAppliedLocation = {
  label: string;
  path: string;
};

type MySkillAppliedLocationsFile = Record<string, MySkillAppliedLocation[]>;

function aggregateProjectSignature(paths: readonly string[]): string {
  return [...paths].map((p) => p.trim()).filter(Boolean).sort().join("\n");
}

function readAggregateSnapshotCache(
  projectPaths: readonly string[],
): AggregateSnapshotCache | null {
  if (typeof localStorage === "undefined") return null;
  try {
    const raw = localStorage.getItem(AGGREGATE_SNAPSHOT_CACHE_KEY);
    if (!raw) return null;
    const data = JSON.parse(raw) as AggregateSnapshotCache;
    if (data.version !== 2) return null;
    if (data.projectSignature !== aggregateProjectSignature(projectPaths)) return null;
    if (!data.snapshot) return null;
    return data;
  } catch {
    return null;
  }
}

function writeAggregateSnapshotCache(
  projectPaths: readonly string[],
  snapshot: AggregateSnapshot,
  mySkillsLibrary: MySkillsLibraryFile | null,
  promptLibrary: PromptLibraryFile | null,
): void {
  if (typeof localStorage === "undefined") return;
  try {
    const data: AggregateSnapshotCache = {
      version: 2,
      savedAt: Date.now(),
      projectSignature: aggregateProjectSignature(projectPaths),
      snapshot,
      mySkillsLibrary,
      promptLibrary,
    };
    localStorage.setItem(AGGREGATE_SNAPSHOT_CACHE_KEY, JSON.stringify(data));
  } catch {
    // Cache is a startup optimization only; ignore quota/private-mode failures.
  }
}

function clearAggregateSnapshotCache(): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.removeItem(AGGREGATE_SNAPSHOT_CACHE_KEY);
  } catch {
    // Ignore storage failures.
  }
}

function readMySkillAppliedLocations(): MySkillAppliedLocationsFile {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(MY_SKILL_APPLIED_LOCATIONS_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as MySkillAppliedLocationsFile;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function writeMySkillAppliedLocations(data: MySkillAppliedLocationsFile): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(MY_SKILL_APPLIED_LOCATIONS_KEY, JSON.stringify(data));
  } catch {
    // This is only a UI warning aid; ignore storage failures.
  }
}

function rememberMySkillAppliedLocation(
  skillId: string,
  location: MySkillAppliedLocation,
): void {
  const id = skillId.trim();
  const path = normalizeSkillPathForCompare(location.path);
  if (!id || !path) return;
  const data = readMySkillAppliedLocations();
  const existing = data[id] ?? [];
  if (existing.some((item) => normalizeSkillPathForCompare(item.path) === path)) return;
  data[id] = [...existing, { ...location, path }];
  writeMySkillAppliedLocations(data);
}

function forgetMySkillAppliedLocations(skillId: string): void {
  const id = skillId.trim();
  if (!id) return;
  const data = readMySkillAppliedLocations();
  if (!(id in data)) return;
  delete data[id];
  writeMySkillAppliedLocations(data);
}

function dedupeMergeInventories(parts: AgentInventory[]): AgentInventory {
  const seen = new Set<string>();
  const skills: AssetEntry[] = [];
  const mcp: AssetEntry[] = [];
  const rules: AssetEntry[] = [];
  const pushUnique = (bucket: AssetEntry[], e: AssetEntry) => {
    if (seen.has(e.id)) return;
    seen.add(e.id);
    bucket.push(e);
  };
  for (const inv of parts) {
    for (const e of inv.skills) pushUnique(skills, e);
    for (const e of inv.mcp) pushUnique(mcp, e);
    for (const e of inv.rules) pushUnique(rules, e);
  }
  return { skills, mcp, rules };
}

function scenarioMapFromInventory(inv: AgentInventory): Map<string, string> {
  const m = new Map<string, string>();
  for (const e of [...inv.skills, ...inv.mcp, ...inv.rules]) {
    if (e.scenario) m.set(e.id, e.scenario);
  }
  return m;
}

function briefMapFromInventory(inv: AgentInventory, locale: "zh" | "en"): Map<string, string> {
  const m = new Map<string, string>();
  for (const e of [...inv.skills, ...inv.mcp, ...inv.rules]) {
    const brief = (locale === "zh" ? e.brief_zh : e.brief_en)?.trim();
    if (brief) m.set(e.id, brief);
  }
  return m;
}

function patchAgentInventory(
  inv: AgentInventory,
  map: Map<string, string>,
): AgentInventory {
  const patch = (e: AssetEntry): AssetEntry => ({
    ...e,
    scenario: map.get(e.id) ?? e.scenario ?? null,
  });
  return {
    skills: inv.skills.map(patch),
    mcp: inv.mcp.map(patch),
    rules: inv.rules.map(patch),
  };
}

function patchAggregateSnapshot(
  snap: AggregateSnapshot,
  map: Map<string, string>,
): AggregateSnapshot {
  return {
    agents: snap.agents.map((a) => ({
      ...a,
      inv: a.inv ? patchAgentInventory(a.inv, map) : null,
    })),
    projects: snap.projects.map((p) => ({
      ...p,
      inv: p.inv ? patchAgentInventory(p.inv, map) : null,
    })),
    anyInventoryFailed: snap.anyInventoryFailed,
  };
}

function patchAgentInventoryBrief(
  inv: AgentInventory,
  locale: "zh" | "en",
  map: Map<string, string>,
): AgentInventory {
  const patch = (e: AssetEntry): AssetEntry => ({
    ...e,
    brief_zh: locale === "zh" ? map.get(e.id) ?? e.brief_zh ?? null : e.brief_zh ?? null,
    brief_en: locale === "en" ? map.get(e.id) ?? e.brief_en ?? null : e.brief_en ?? null,
  });
  return {
    skills: inv.skills.map(patch),
    mcp: inv.mcp.map(patch),
    rules: inv.rules.map(patch),
  };
}

function patchAggregateSnapshotBrief(
  snap: AggregateSnapshot,
  locale: "zh" | "en",
  map: Map<string, string>,
): AggregateSnapshot {
  return {
    agents: snap.agents.map((a) => ({
      ...a,
      inv: a.inv ? patchAgentInventoryBrief(a.inv, locale, map) : null,
    })),
    projects: snap.projects.map((p) => ({
      ...p,
      inv: p.inv ? patchAgentInventoryBrief(p.inv, locale, map) : null,
    })),
    anyInventoryFailed: snap.anyInventoryFailed,
  };
}

function patchEntryBriefInInventory(
  inv: AgentInventory,
  sourceId: string,
  locale: "zh" | "en",
  brief: string,
): AgentInventory {
  const patch = (e: AssetEntry): AssetEntry =>
    e.id === sourceId
      ? {
          ...e,
          brief_zh: locale === "zh" ? brief : e.brief_zh ?? null,
          brief_en: locale === "en" ? brief : e.brief_en ?? null,
        }
      : e;
  return {
    skills: inv.skills.map(patch),
    mcp: inv.mcp.map(patch),
    rules: inv.rules.map(patch),
  };
}

type FilterKey = "all" | AssetKind;

const FILTER_LABEL: Record<FilterKey, string> = {
  all: "All",
  skill: "Skill",
  mcp: "MCP",
  rule: "Rule",
};

const SEGMENT_KEYS: FilterKey[] = ["all", "skill", "mcp", "rule"];
type MineKindFilter = "all" | "skill" | "prompt";

const MINE_KIND_LABEL: Record<MineKindFilter, string> = {
  all: "全部",
  skill: "Skill",
  prompt: "Prompt",
};

function inventoryToRows(
  inv: AgentInventory,
  ecosystem: string,
  agentTitle: string,
  locale: "zh" | "en",
): BrowseRow[] {
  const rows: BrowseRow[] = [];
  const push = (e: AssetEntry, kind: BrowseKind) => {
    const brief = (locale === "zh" ? e.brief_zh : e.brief_en)?.trim();
    const label = kind === "prompt" ? "Prompt" : FILTER_LABEL[kind];
    rows.push({
      id: e.id,
      sourceId: e.id,
      title: e.title,
      desc: brief || e.description,
      descSource: brief ? "ai" : "source",
      kind,
      ecosystem,
      tags: [label, agentTitle],
      active: e.active,
      sourcePath: e.path,
      skillExtraFiles: e.skill_extra_files ?? undefined,
      scenario: e.scenario ?? null,
    });
  };
  for (const e of inv.skills) push(e, e.kind === "prompt" ? "prompt" : "skill");
  for (const e of inv.mcp) push(e, "mcp");
  for (const e of inv.rules) push(e, "rule");
  return rows;
}

function zeroScenarioCounts(): Record<ScenarioKey, number> {
  return {
    all: 0,
    dev: 0,
    office: 0,
    creative: 0,
    data: 0,
    network: 0,
    ops: 0,
    collab: 0,
  };
}

/** 在当前类型与搜索筛选下，各场景匹配条数（与点击场景芯片的判定一致） */
function scenarioCountsFromRows(rows: BrowseRow[]): Record<ScenarioKey, number> {
  const counts = zeroScenarioCounts();
  counts.all = rows.length;
  for (const row of rows) {
    for (const key of SCENARIO_ORDER) {
      if (rowMatchesScenarioChip(row, key)) counts[key]++;
    }
  }
  return counts;
}

function agentOptionsFromRows(
  rows: BrowseRow[],
  locale: "zh" | "en",
): AgentFilterOption[] {
  const counts = new Map<string, number>();
  for (const row of rows) {
    const id = row.agentId ?? row.ecosystem;
    counts.set(id, (counts.get(id) ?? 0) + 1);
  }

  return [...counts.entries()]
    .map(([id, count]) => ({
      id,
      label: agentLabelForId(id, locale),
      count,
    }))
    .sort((a, b) => b.count - a.count || a.label.localeCompare(b.label));
}

function stringSetsEqual(a: Set<string>, b: Set<string>): boolean {
  if (a.size !== b.size) return false;
  for (const x of a) {
    if (!b.has(x)) return false;
  }
  return true;
}

type Props = {
  title: string;
  /** 与侧栏 Agent 一致时展示该生态；支持扫描到的全部 id */
  ecosystem?: string;
  /** `project` 为所选目录；`aggregate` 汇总全部 Agent 全局配置与侧栏全部项目 */
  dataSet?: "skills" | "project" | "aggregate";
  /** 页标题下方一行说明（例如来自 ?path=） */
  subtitle?: string;
  /** 项目根目录（仅 `dataSet="project"`），由 ?path= 传入 */
  projectRoot?: string;
};

export default function SkillBrowseShell({
  title,
  ecosystem,
  dataSet = "skills",
  subtitle,
  projectRoot,
}: Props) {
  const { locale } = useI18n();
  const searchFieldId = useId();
  const browseSectionDomPrefix = useId().replace(/\W/g, "");
  const aggregateLoadMoreRef = useRef<HTMLDivElement | null>(null);
  const aggregateAgentFilterRef = useRef<HTMLDetailsElement | null>(null);
  const forceClassifyOnRefreshRef = useRef(false);
  const [searchParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const [scenario, setScenario] = useState<ScenarioKey>("all");
  const [filter, setFilter] = useState<FilterKey>("all");
  const [aggregateVisibleCount, setAggregateVisibleCount] = useState(
    AGGREGATE_RENDER_PAGE_SIZE,
  );
  const [selectedEntry, setSelectedEntry] = useState<DetailEntry | null>(null);
  /** 展开中的分组 key；Agent/项目页由列表分组数量同步（仅 1 个标题时默认展开） */
  const [expandedSectionKeys, setExpandedSectionKeys] = useState<
    Set<string>
  >(() => new Set());
  const [liveInv, setLiveInv] = useState<AgentInventory | null | undefined>(
    undefined,
  );
  const [liveLoading, setLiveLoading] = useState(false);
  const [liveFailed, setLiveFailed] = useState(false);
  const [agentProjectScans, setAgentProjectScans] = useState<
    { path: string; inv: AgentInventory | null }[]
  >([]);
  const [projectInv, setProjectInv] = useState<AgentInventory | null | undefined>(
    undefined,
  );
  const [projectLoading, setProjectLoading] = useState(false);
  const [projectFailed, setProjectFailed] = useState(false);

  const projectPaths = useProjectPaths();
  const [aggregateSnapshot, setAggregateSnapshot] =
    useState<AggregateSnapshot | null>(null);
  const [detectedAgentTargets, setDetectedAgentTargets] = useState<
    PromptApplyAgentTarget[]
  >([]);
  const [aggregateLoading, setAggregateLoading] = useState(false);
  const [aiScenarioBusy, setAiScenarioBusy] = useState(false);
  const [aiBriefBusy, setAiBriefBusy] = useState(false);
  const [mineAiBusy, setMineAiBusy] = useState(false);
  const [mineScenarioMap, setMineScenarioMap] = useState<Map<string, string>>(
    () => new Map(),
  );
  const [mineBriefMap, setMineBriefMap] = useState<Map<string, string>>(
    () => new Map(),
  );
  const [cardContextMenu, setCardContextMenu] = useState<{
    x: number;
    y: number;
    row: BrowseRow;
  } | null>(null);
  const cardContextMenuRef = useRef<HTMLDivElement>(null);
  const [skillCopyTargetModalRow, setSkillCopyTargetModalRow] =
    useState<BrowseRow | null>(null);
  /** 底部/顶部操作反馈吐司；`at` 变化时重置自动消失计时 */
  const [shellToast, setShellToast] = useState<{
    at: number;
    message: string;
  } | null>(null);
  /** 递增以使数据 useEffect 重新拉取（与手动刷新配合） */
  const [refreshKey, setRefreshKey] = useState(0);
  const [userCustomAgentIds, setUserCustomAgentIds] = useState<string[]>([]);
  /** 「全部」资产页：汇总 vs 我的技能库 */
  const [aggregateAssetsTab, setAggregateAssetsTab] = useState<"all" | "mine">(
    "all",
  );
  const [aggregateAgentFilter, setAggregateAgentFilter] = useState<string[]>([]);
  /** 单个 Agent 页：Agent 原目录 vs AIControls 我的 Skills */
  const [agentAssetsTab, setAgentAssetsTab] = useState<"all" | "mine">("all");
  const [mySkillsLib, setMySkillsLib] = useState<MySkillsLibraryFile | null>(
    null,
  );
  const [promptLibrary, setPromptLibrary] = useState<PromptLibraryFile | null>(
    null,
  );
  const [mySkillsLoading, setMySkillsLoading] = useState(false);
  const [mySkillsImportBusy, setMySkillsImportBusy] = useState(false);
  const [mySkillAddPendingPaths, setMySkillAddPendingPaths] = useState<Set<string>>(
    () => new Set(),
  );
  const [aggregateMineKind, setAggregateMineKind] =
    useState<MineKindFilter>("all");

  // ── 重新分类状态 ──
  const [reclassifyMode, setReclassifyMode] = useState<"idle" | "generating" | "reviewing" | "applying">("idle");
  const [customCategories, setCustomCategories] = useState<CustomCategory[] | null>(null);
  const [customScenario, setCustomScenario] = useState<string>("all");
  const [savedSnapshotBackup, setSavedSnapshotBackup] = useState<AggregateSnapshot | null>(null);

  const onRefreshInventory = useCallback(() => {
    forceClassifyOnRefreshRef.current = dataSet === "aggregate";
    if (dataSet === "aggregate") {
      invalidateCachedAgentGlobalInventory();
      invalidateCachedProjectInventory();
      clearAggregateSnapshotCache();
    } else if (dataSet === "project" && projectRoot?.trim()) {
      invalidateCachedProjectInventory(projectRoot);
    } else if (dataSet === "skills" && ecosystem) {
      invalidateCachedAgentGlobalInventory(ecosystem);
      invalidateCachedProjectInventory();
    }
    setRefreshKey((k) => k + 1);
  }, [dataSet, ecosystem, projectRoot]);

  useEffect(() => {
    const bump = () => {
      clearAggregateSnapshotCache();
      setRefreshKey((k) => k + 1);
    };
    window.addEventListener("aicontrols-agents-changed", bump);
    return () => window.removeEventListener("aicontrols-agents-changed", bump);
  }, []);

  useEffect(() => {
    listDetectedAgents().then((agents) => {
      setUserCustomAgentIds(
        (agents ?? []).filter((a) => a.id.startsWith("useragent-")).map((a) => a.id),
      );
    });
  }, [refreshKey]);

  const toggleAggregateAgentFilter = useCallback((agentId: string) => {
    setAggregateAgentFilter((prev) =>
      prev.includes(agentId)
        ? prev.filter((id) => id !== agentId)
        : [...prev, agentId],
    );
  }, []);

  /** 自定义 Agent 从侧栏移除后，从筛选勾选状态中剔除对应 id。 */
  useEffect(() => {
    if (dataSet !== "aggregate" || !aggregateSnapshot) return;
    setAggregateAgentFilter((prev) =>
      prev.filter((id) => {
        if (!id.startsWith("useragent-")) return true;
        return aggregateSnapshot.agents.some((a) => a.id === id);
      }),
    );
  }, [aggregateSnapshot, dataSet]);

  useEffect(() => {
    const onPointerDown = (event: PointerEvent) => {
      const node = aggregateAgentFilterRef.current;
      if (!node?.open) return;
      if (event.target instanceof Node && node.contains(event.target)) return;
      node.open = false;
    };

    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, []);

  // ── 重新分类回调 ──
  const handleReclassifyStart = useCallback(async () => {
    if (!aggregateSnapshot) return;
    const cfg = await getDeepseekSettings();
    if (!cfg?.apiKeyConfigured) return;
    setReclassifyMode("generating");
    const merged = dedupeMergeInventories(
      [
        ...aggregateSnapshot.agents.map((a) => a.inv).filter((x): x is AgentInventory => !!x),
        ...aggregateSnapshot.projects.map((p) => p.inv).filter((x): x is AgentInventory => !!x),
      ],
    );
    const cats = await deepseekRegenerateCategories(merged, locale);
    if (!cats) {
      setReclassifyMode("idle");
      return;
    }
    if (!savedSnapshotBackup) {
      setSavedSnapshotBackup(aggregateSnapshot);
    }
    setCustomCategories(cats);
    setCustomScenario("all");
    setReclassifyMode("reviewing");
  }, [aggregateSnapshot, savedSnapshotBackup, locale]);

  const handleReclassifyConfirm = useCallback(async () => {
    if (!aggregateSnapshot || !customCategories) return;
    setReclassifyMode("applying");
    const merged = dedupeMergeInventories(
      [
        ...aggregateSnapshot.agents.map((a) => a.inv).filter((x): x is AgentInventory => !!x),
        ...aggregateSnapshot.projects.map((p) => p.inv).filter((x): x is AgentInventory => !!x),
      ],
    );
    const mapping = await deepseekReclassifyWithCategories(merged, customCategories);
    if (mapping) {
      // 保存成功，触发完整重新加载以从持久化存储中读取新的 scenario 数据
      setReclassifyMode("idle");
      setSavedSnapshotBackup(null);
      setCustomScenario("all");
      // customCategories 保留，因为它们已被持久化，重新加载后仍会使用
      setRefreshKey((k) => k + 1);
    } else {
      setReclassifyMode("idle");
      setCustomCategories(null);
      setCustomScenario("all");
      setSavedSnapshotBackup(null);
    }
  }, [aggregateSnapshot, customCategories]);

  const handleReclassifyCancel = useCallback(() => {
    if (savedSnapshotBackup) {
      setAggregateSnapshot(savedSnapshotBackup);
    }
    setReclassifyMode("idle");
    setCustomCategories(null);
    setCustomScenario("all");
    setSavedSnapshotBackup(null);
  }, [savedSnapshotBackup]);

  const handleResetCategories = useCallback(async () => {
    const msg = locale === "zh"
      ? "确定要重置为默认分类吗？所有 AI 分类结果将被清除。"
      : "Reset to default categories? All AI classification results will be cleared.";
    if (!window.confirm(msg)) return;
    const ok = await resetAllCategories();
    if (ok) {
      setCustomCategories(null);
      setCustomScenario("all");
      invalidateCachedAgentGlobalInventory();
      invalidateCachedProjectInventory();
      setRefreshKey((k) => k + 1);
    }
  }, [locale]);

  const refreshBusy =
    (dataSet === "skills" &&
      !!ecosystem &&
      (liveLoading || aiScenarioBusy || aiBriefBusy)) ||
    (dataSet === "project" &&
      !!projectRoot?.trim() &&
      (projectLoading || aiScenarioBusy || aiBriefBusy)) ||
    (dataSet === "aggregate" &&
      aggregateAssetsTab === "mine" &&
      (mySkillsLoading || mineAiBusy)) ||
    (dataSet === "aggregate" &&
      aggregateAssetsTab === "all" &&
      (aggregateLoading || aiScenarioBusy || aiBriefBusy));

  useEffect(() => {
    const k = searchParams.get("kind");
    if (k === "skill" || k === "mcp" || k === "rule") {
      setFilter(k);
    } else {
      setFilter("all");
    }
  }, [searchParams]);

  useEffect(() => {
    setCardContextMenu(null);
    setSkillCopyTargetModalRow(null);
    setShellToast(null);
  }, [dataSet, ecosystem, projectRoot]);

  useEffect(() => {
    if (dataSet !== "aggregate") setAggregateAssetsTab("all");
    if (dataSet !== "skills") setAgentAssetsTab("all");
  }, [dataSet]);

  useEffect(() => {
    if (!(dataSet === "aggregate" && aggregateAssetsTab === "all")) {
      setAggregateAgentFilter([]);
    }
  }, [dataSet, aggregateAssetsTab]);

  // 加载持久化的自定义分类（重新分类确认后保存的）
  useEffect(() => {
    let cancelled = false;
    loadCustomCategoriesFromStorage().then((cats) => {
      if (!cancelled && cats && cats.length > 0) {
        setCustomCategories(cats);
      }
    });
    return () => { cancelled = true; };
  }, [refreshKey]);

  useEffect(() => {
    if (locale !== "en" || !customCategories?.length) return;
    if (customCategories.every((cat) => cat.labelEn?.trim())) return;
    let cancelled = false;
    (async () => {
      const cfg = await getDeepseekSettings();
      if (cancelled || !cfg?.apiKeyConfigured) return;
      const translated = await deepseekTranslateCustomCategories(customCategories, locale);
      if (!cancelled && translated?.length) {
        setCustomCategories(translated);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [customCategories, locale]);

  useEffect(() => {
    if (
      (dataSet === "aggregate" && aggregateAssetsTab === "mine") ||
      (dataSet === "skills" && agentAssetsTab === "mine")
    ) {
      setFilter("all");
      setScenario("all");
    }
  }, [dataSet, aggregateAssetsTab, agentAssetsTab]);

  useEffect(() => {
    if (!(dataSet === "aggregate" && aggregateAssetsTab === "mine")) {
      setAggregateMineKind("all");
    }
  }, [dataSet, aggregateAssetsTab]);

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

  useEffect(() => {
    if (!skillCopyTargetModalRow) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setSkillCopyTargetModalRow(null);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [skillCopyTargetModalRow]);

  useEffect(() => {
    if (shellToast === null) return;
    const t = window.setTimeout(() => setShellToast(null), 2600);
    return () => window.clearTimeout(t);
  }, [shellToast?.at]);

  useEffect(() => {
    if (!ecosystem || dataSet !== "skills") {
      setLiveInv(undefined);
      setLiveFailed(false);
      setLiveLoading(false);
      setAgentProjectScans([]);
      return;
    }
    let cancelled = false;
    setLiveLoading(true);
    setLiveFailed(false);
    setLiveInv(undefined);
    setAgentProjectScans([]);

    (async () => {
      const globalInv = await getAgentGlobalInventoryCached(ecosystem);
      const projList = await Promise.all(
        projectPaths.map((path) => scanProjectDirectoryCached(path)),
      );
      if (cancelled) return;

      setLiveLoading(false);
      setLiveFailed(globalInv === null);
      setLiveInv(globalInv);
      setAgentProjectScans(
        projectPaths.map((path, i) => ({
          path,
          inv: projList[i],
        })),
      );

      const parts: AgentInventory[] = [];
      if (globalInv) parts.push(globalInv);
      for (const raw of projList) {
        if (raw) parts.push(filterInventoryForAgent(ecosystem, raw));
      }
      const merged = dedupeMergeInventories(parts);
      const cfg = await getDeepseekSettings();
      if (
        cancelled ||
        !cfg?.apiKeyConfigured ||
        inventoryAssetCount(merged) === 0
      ) {
        return;
      }
      setAiScenarioBusy(true);
      const classified = await deepseekClassifyInventory(merged);
      if (cancelled || !classified) {
        if (!cancelled) setAiScenarioBusy(false);
        return;
      }
      const map = scenarioMapFromInventory(classified);
      if (globalInv) setLiveInv(patchAgentInventory(globalInv, map));
      setAgentProjectScans(
        projectPaths.map((path, i) => {
          const raw = projList[i];
          if (!raw) return { path, inv: null };
          const filtered = filterInventoryForAgent(ecosystem, raw);
          return {
            path,
            inv: patchAgentInventory(filtered, map),
          };
        }),
      );
      if (!cancelled) setAiScenarioBusy(false);

      const baseForSummary = classified ?? merged;
      setAiBriefBusy(true);
      const summarized = await deepseekSummarizeInventory(baseForSummary, locale);
      if (cancelled || !summarized) {
        if (!cancelled) setAiBriefBusy(false);
        return;
      }
      const briefMap = briefMapFromInventory(summarized, locale);
      setLiveInv((prev) => (prev ? patchAgentInventoryBrief(prev, locale, briefMap) : prev));
      setAgentProjectScans((prev) =>
        prev.map((item) => ({
          ...item,
          inv: item.inv ? patchAgentInventoryBrief(item.inv, locale, briefMap) : null,
        })),
      );
      if (!cancelled) setAiBriefBusy(false);
    })();

    return () => {
      cancelled = true;
    };
  }, [ecosystem, dataSet, projectPaths, refreshKey, locale]);

  useEffect(() => {
    if (dataSet !== "project") {
      setProjectInv(undefined);
      setProjectFailed(false);
      setProjectLoading(false);
      return;
    }
    if (!projectRoot) {
      setProjectInv(undefined);
      setProjectFailed(false);
      setProjectLoading(false);
      return;
    }
    let cancelled = false;
    setProjectLoading(true);
    setProjectFailed(false);
    scanProjectDirectoryCached(projectRoot).then(async (data) => {
      if (cancelled) return;
      setProjectLoading(false);
      if (data === null) {
        setProjectInv(null);
        setProjectFailed(true);
        return;
      }
      setProjectFailed(false);
      setProjectInv(data);
      const cfg = await getDeepseekSettings();
      if (cancelled || !cfg?.apiKeyConfigured) return;
      setAiScenarioBusy(true);
      const next = await deepseekClassifyInventory(data);
      if (!cancelled && next) setProjectInv(next);
      if (!cancelled) setAiScenarioBusy(false);
      const baseForSummary = next ?? data;
      setAiBriefBusy(true);
      const summarized = await deepseekSummarizeInventory(baseForSummary, locale);
      if (!cancelled && summarized) setProjectInv(summarized);
      if (!cancelled) setAiBriefBusy(false);
    });
    return () => {
      cancelled = true;
    };
  }, [dataSet, projectRoot, refreshKey, locale]);

  useEffect(() => {
    if (dataSet !== "aggregate") {
      setAggregateLoading(false);
      setMySkillsLoading(false);
      return;
    }
    let cancelled = false;
    let frameId: number | null = null;
    let timerId: number | null = null;

    const loadAggregateData = async () => {
      if (cancelled) return;

      const cached = readAggregateSnapshotCache(projectPaths);
      if (cancelled) return;
      if (cached) {
        setAggregateSnapshot(cached.snapshot);
        setMySkillsLib(cached.mySkillsLibrary);
        setPromptLibrary(cached.promptLibrary);
        setDetectedAgentTargets(
          cached.snapshot.agents.map((agent) => ({
            id: agent.id,
            label: agent.title,
          })),
        );
        if (Date.now() - cached.savedAt < AGGREGATE_SNAPSHOT_CACHE_TTL_MS) {
          setAggregateLoading(false);
          setMySkillsLoading(false);
          return;
        }
      } else {
        setAggregateSnapshot(null);
        setMySkillsLib(null);
        setPromptLibrary(null);
      }
      setAggregateLoading(true);
      setMySkillsLoading(true);

      const librariesPromise = Promise.all([getMySkillsLibrary(), getPromptLibrary()])
        .catch((): [MySkillsLibraryFile | null, PromptLibraryFile | null] => [null, null]);
      const detected = await listDetectedAgents();
      if (!cancelled) {
        setDetectedAgentTargets(
          (detected ?? []).map((agent) => ({
            id: agent.id,
            label: AGENT_LABEL_BY_ID[agent.id] ?? agent.label ?? agent.id,
          })),
        );
      }
      const specs =
        detected && detected.length > 0
          ? detected.map((a) => ({
              id: a.id,
              title: AGENT_LABEL_BY_ID[a.id] ?? a.label ?? a.id,
            }))
          : FALLBACK_AGENT_IDS.map((id) => ({
              id,
              title: AGENT_LABEL_BY_ID[id] ?? id,
            }));

      const agentResults = await Promise.all(
        specs.map(async (spec) => ({
          id: spec.id,
          title: spec.title,
          inv: await getAgentGlobalInventoryCached(spec.id),
        })),
      );

      const projectResults = await Promise.all(
        projectPaths.map(async (path) => ({
          path,
          inv: await scanProjectDirectoryCached(path),
        })),
      );

      const [skillsLibrary, promptsLibrary] = await librariesPromise;
      if (cancelled) return;

      const anyInventoryFailed =
        agentResults.some((r) => r.inv === null) ||
        projectResults.some((r) => r.inv === null);

      const snapshot: AggregateSnapshot = {
        agents: agentResults,
        projects: projectResults,
        anyInventoryFailed,
      };
      setAggregateSnapshot(snapshot);
      setMySkillsLib(skillsLibrary);
      setPromptLibrary(promptsLibrary);
      writeAggregateSnapshotCache(projectPaths, snapshot, skillsLibrary, promptsLibrary);
      setAggregateLoading(false);
      setMySkillsLoading(false);

      const parts: AgentInventory[] = [];
      for (const a of agentResults) {
        if (a.inv) parts.push(a.inv);
      }
      for (const p of projectResults) {
        if (p.inv) parts.push(p.inv);
      }
      const merged = dedupeMergeInventories(parts);
      const cfg = await getDeepseekSettings();
      if (
        cancelled ||
        !cfg?.apiKeyConfigured ||
        inventoryAssetCount(merged) === 0
      ) {
        forceClassifyOnRefreshRef.current = false;
        return;
      }
      setAiScenarioBusy(true);
      const shouldForceCustomClassify =
        forceClassifyOnRefreshRef.current &&
        !!customCategories &&
        customCategories.length > 0;
      const classified = shouldForceCustomClassify
        ? await deepseekReclassifyWithCategories(merged, customCategories)
        : await deepseekClassifyInventory(merged);
      let baseForSummary: AgentInventory | Record<string, string> | null =
        classified;
      if (!cancelled && classified) {
        const map =
          shouldForceCustomClassify
            ? new Map(Object.entries(classified as Record<string, string>))
            : scenarioMapFromInventory(classified as AgentInventory);
        const patched = patchAggregateSnapshot(snapshot, map);
        setAggregateSnapshot(patched);
        writeAggregateSnapshotCache(projectPaths, patched, skillsLibrary, promptsLibrary);
      }
      forceClassifyOnRefreshRef.current = false;
      if (!cancelled) setAiScenarioBusy(false);

      if (shouldForceCustomClassify) baseForSummary = merged;
      const summaryInventory =
        baseForSummary && !("skills" in baseForSummary)
          ? merged
          : (baseForSummary as AgentInventory | null) ?? merged;
      setAiBriefBusy(true);
      const summarized = await deepseekSummarizeInventory(summaryInventory, locale);
      if (!cancelled && summarized) {
        const briefMap = briefMapFromInventory(summarized, locale);
        setAggregateSnapshot((prev) => {
          if (!prev) return prev;
          const patched = patchAggregateSnapshotBrief(prev, locale, briefMap);
          writeAggregateSnapshotCache(projectPaths, patched, skillsLibrary, promptsLibrary);
          return patched;
        });
      }
      if (!cancelled) setAiBriefBusy(false);
    };

    frameId = window.requestAnimationFrame(() => {
      timerId = window.setTimeout(() => {
        void loadAggregateData();
      }, 0);
    });

    return () => {
      cancelled = true;
      if (frameId !== null) window.cancelAnimationFrame(frameId);
      if (timerId !== null) window.clearTimeout(timerId);
    };
  }, [dataSet, projectPaths, refreshKey, locale, customCategories]);

  useEffect(() => {
    if (dataSet !== "aggregate") {
      setMineAiBusy(false);
      return;
    }

    const mineInv = mineLibrariesToInventory(mySkillsLib, promptLibrary);
    if (inventoryAssetCount(mineInv) === 0) {
      setMineScenarioMap(new Map());
      setMineBriefMap(new Map());
      setMineAiBusy(false);
      return;
    }

    let cancelled = false;
    setMineAiBusy(true);

    (async () => {
      const cfg = await getDeepseekSettings();
      if (cancelled || !cfg?.apiKeyConfigured) return;

      const classified = await deepseekClassifyInventory(mineInv);
      if (cancelled) return;
      if (classified) {
        setMineScenarioMap(scenarioMapFromInventory(classified));
      }

      const summarized = await deepseekSummarizeInventory(
        classified ?? mineInv,
        locale,
      );
      if (cancelled) return;
      if (summarized) {
        setMineBriefMap(briefMapFromInventory(summarized, locale));
      }
    })().finally(() => {
      if (!cancelled) setMineAiBusy(false);
    });

    return () => {
      cancelled = true;
    };
  }, [dataSet, mySkillsLib, promptLibrary, refreshKey, locale]);

  const { sections, scenarioCounts, customCategoryCounts, agentOptions } = useMemo((): {
    sections: BrowseSection[];
    scenarioCounts: Record<ScenarioKey, number>;
    customCategoryCounts: Record<string, number> | null;
    agentOptions: AgentFilterOption[];
  } => {
    const empty = zeroScenarioCounts();
    const emptyResult = {
      sections: [],
      scenarioCounts: empty,
      customCategoryCounts: null,
      agentOptions: [],
    };

    const applyKindAndQuery = (rows: BrowseRow[]): BrowseRow[] => {
      let r = rows;
      if (filter !== "all") {
        r = r.filter((row) => row.kind === filter);
      }
      const q = query.trim().toLowerCase();
      if (q) {
        r = r.filter(
          (row) =>
            row.title.toLowerCase().includes(q) ||
            row.desc.toLowerCase().includes(q) ||
            (row.csCommand?.toLowerCase().includes(q) ?? false) ||
            (row.cpCommand?.toLowerCase().includes(q) ?? false) ||
            (row.sourcePath?.toLowerCase().includes(q) ?? false),
        );
      }
      return r;
    };

    const applyScenarioFilter = (rows: BrowseRow[]): BrowseRow[] => {
      const useCustom = (reclassifyMode === "reviewing" || reclassifyMode === "generating" || reclassifyMode === "applying" || reclassifyMode === "idle")
        && customCategories && customCategories.length > 0;
      if (useCustom) {
        if (customScenario === "all") return rows;
        return rows.filter((row) => rowMatchesCustomScenario(row, customScenario));
      }
      if (scenario === "all") return rows;
      return rows.filter((row) => rowMatchesScenarioChip(row, scenario));
    };

    // ── 辅助函数：从过滤前的 rows 计算自定义分类计数 ──
    const computeCustomCategoryCounts = (rows: BrowseRow[]): Record<string, number> | null => {
      if (!customCategories || customCategories.length === 0) return null;
      if (reclassifyMode !== "reviewing" && reclassifyMode !== "generating" && reclassifyMode !== "applying" && reclassifyMode !== "idle") return null;
      const counts: Record<string, number> = { all: rows.length };
      for (const cat of customCategories) {
        counts[cat.slug] = rows.filter((row) => rowMatchesCustomScenario(row, cat.slug)).length;
      }
      return counts;
    };

    const buildMineRows = (scopeLabel?: string): BrowseRow[] =>
      (mySkillsLib?.items ?? []).map((it) => {
        const isPromptSkill = it.sourceKind === "prompt";
        const command = isPromptSkill
          ? cpsCommandForSkill(it.title, it.path)
          : csCommandForSkill(it.title, it.path);
        const brief = mineBriefMap.get(it.id)?.trim();
        return {
          id: `mine:${it.id}`,
          sourceId: it.id,
          title: it.title,
          desc: brief || it.description,
          descSource: brief ? "ai" : "source",
          kind: "skill",
          ecosystem: ecosystem ?? "cursor",
          tags: [
            locale === "zh" ? "我的技能库" : "My skills",
            ...(scopeLabel ? [scopeLabel] : []),
            command,
          ],
          active: true,
          sourcePath: it.path,
          csCommand: command,
          mineSkillSourceKind: it.sourceKind ?? null,
          scenario: mineScenarioMap.get(it.id) ?? null,
        };
      });

    const buildPublishedPromptRows = (): BrowseRow[] =>
      (promptLibrary?.items ?? []).flatMap((it) => {
        const cpCommand = cpCommandForPrompt(it);
        if (!cpCommand) return [];
        const brief = mineBriefMap.get(it.id)?.trim();
        const desc = brief || promptDescriptionForCard(it, cpCommand);
        return [{
          id: `prompt:${it.id}`,
          sourceId: it.id,
          title: it.title,
          desc,
          descSource: brief ? "ai" : "source",
          kind: "prompt",
          ecosystem: "prompt",
          tags: [locale === "zh" ? "Prompt 库" : "Prompt Library", cpCommand],
          active: true,
          cpCommand,
          promptText: it.prompt,
          promptCommandName: it.commandName ?? cpCommand.replace(/^\/cp-/, ""),
          scenario: mineScenarioMap.get(it.id) ?? null,
        }];
      });

    if (dataSet === "aggregate" && aggregateAssetsTab === "mine") {
      let mineRows = [...buildMineRows(), ...buildPublishedPromptRows()];
      if (aggregateMineKind !== "all") {
        mineRows = mineRows.filter((row) => row.kind === aggregateMineKind);
      }
      const scenarioCounts = scenarioCountsFromRows(mineRows);
      const customCatCounts = computeCustomCategoryCounts(mineRows);
      const filtered = applyScenarioFilter(applyKindAndQuery(mineRows));
      return {
        sections: [{ key: "mine-grid", title: "", rows: filtered }],
        scenarioCounts,
        customCategoryCounts: customCatCounts,
        agentOptions: [],
      };
    }

    if (dataSet === "skills" && ecosystem && agentAssetsTab === "mine") {
      if (liveLoading && liveInv === undefined) {
        return emptyResult;
      }
      const mineInv: AgentInventory = {
        skills: liveInv?.skills.filter(isCsSkillEntry) ?? [],
        mcp: [],
        rules: [],
      };
      const mineRows = inventoryToRows(
        mineInv,
        ecosystem,
        AGENT_LABEL_BY_ID[ecosystem] ?? title,
        locale,
      ).map((r) => {
        const installedCommand =
          r.kind === "prompt" ? promptCommandForInstalledPrompt(r) : csCommandForInstalledSkill(r);
        return {
          ...r,
          id: `g:${ecosystem}:mine:${r.id}`,
          tags: [
            AGENT_LABEL_BY_ID[ecosystem] ?? title,
            r.kind === "prompt"
              ? ecosystem === "codex"
                ? locale === "zh" ? "我的 /prompts" : "My /prompts"
                : locale === "zh" ? "我的 /cp" : "My /cp"
              : locale === "zh" ? "我的 /cs" : "My /cs",
            installedCommand,
          ],
          csCommand: r.kind === "prompt" ? undefined : installedCommand,
          cpCommand: r.kind === "prompt" ? installedCommand : undefined,
        };
      });
      const scenarioCounts = scenarioCountsFromRows(mineRows);
      const customCatCounts = computeCustomCategoryCounts(mineRows);
      const filtered = applyScenarioFilter(applyKindAndQuery(mineRows));
      return {
        sections: [{ key: `mine-${ecosystem}`, title: "", rows: filtered }],
        scenarioCounts,
        customCategoryCounts: customCatCounts,
        agentOptions: [],
      };
    }

    if (dataSet === "project") {
      if (!projectRoot) {
        return emptyResult;
      }
      if (projectLoading && projectInv === undefined) {
        return emptyResult;
      }
      if (projectFailed) {
        return emptyResult;
      }
      if (!projectInv) {
        return emptyResult;
      }
      const buckets = bucketInventoryByAgent(projectInv);
      const out: BrowseSection[] = [];
      for (const { agentId, inv } of buckets) {
        const agentTitle =
          agentId === "__other__"
            ? locale === "zh"
              ? "其他"
              : "Other"
            : AGENT_LABEL_BY_ID[agentId] ?? agentId;
        let rows = inventoryToRows(inv, agentId, agentTitle, locale).map((r) => ({
          ...r,
          id: `proj:${agentId}:${r.id}`,
        }));
        rows = applyKindAndQuery(rows);
        out.push({ key: agentId, title: agentTitle, rows });
      }
      out.sort((a, b) => b.rows.length - a.rows.length);
      const scenarioCounts = scenarioCountsFromRows(out.flatMap((s) => s.rows));
      const customCatCounts = computeCustomCategoryCounts(out.flatMap((s) => s.rows));
      const filtered = out
        .map((s) => ({ ...s, rows: applyScenarioFilter(s.rows) }))
        .filter((s) => s.rows.length > 0);
      return { sections: filtered, scenarioCounts, customCategoryCounts: customCatCounts, agentOptions: [] };
    }

    if (dataSet === "aggregate") {
      if (aggregateLoading || aggregateSnapshot === null) {
        return emptyResult;
      }
      const selectedAgentIds = new Set(aggregateAgentFilter);
      let rows: BrowseRow[] = [];
      for (const a of aggregateSnapshot.agents) {
        if (!a.inv) continue;
        rows.push(
          ...inventoryToRows(a.inv, a.id, a.title, locale).map((r) => ({
            ...r,
            id: `g:${a.id}:${r.id}`,
            agentId: a.id,
            tags: [a.title, locale === "zh" ? "用户全局" : "Global"],
          })),
        );
      }
      for (const p of aggregateSnapshot.projects) {
        if (!p.inv) continue;
        const bn = folderBasename(p.path);
        rows.push(
          ...inventoryToRows(p.inv, "project", bn, locale).map((r) => {
            const aid = inferAgentIdFromAssetPath(r.sourcePath ?? "");
            const agentLbl = aid
              ? agentLabelForId(aid, locale)
              : agentLabelForId("__other__", locale);
            return {
              ...r,
              id: `p:${p.path}:${r.id}`,
              agentId: aid ?? "__other__",
              tags: [agentLbl, bn],
            };
          }),
        );
      }
      rows = applyKindAndQuery(rows);
      const nextAgentOptions = agentOptionsFromRows(rows, locale);
      const optionIds = new Set(nextAgentOptions.map((option) => option.id));
      for (const a of aggregateSnapshot.agents) {
        if (!optionIds.has(a.id)) {
          nextAgentOptions.push({
            id: a.id,
            label: a.title,
            count: 0,
          });
          optionIds.add(a.id);
        }
      }
      for (const id of selectedAgentIds) {
        if (!optionIds.has(id)) {
          nextAgentOptions.push({
            id,
            label: agentLabelForId(id, locale),
            count: 0,
          });
          optionIds.add(id);
        }
      }
      nextAgentOptions.sort(
        (a, b) => b.count - a.count || a.label.localeCompare(b.label),
      );
      if (selectedAgentIds.size > 0) {
        rows = rows.filter((row) => selectedAgentIds.has(row.agentId ?? row.ecosystem));
      }
      const scenarioCounts = scenarioCountsFromRows(rows);
      const customCatCounts = computeCustomCategoryCounts(rows);
      rows = applyScenarioFilter(rows);
      return {
        sections: [{ key: "aggregate", title: "", rows }],
        scenarioCounts,
        customCategoryCounts: customCatCounts,
        agentOptions: nextAgentOptions,
      };
    }

    if (ecosystem && dataSet === "skills") {
      if (liveLoading && liveInv === undefined) {
        return emptyResult;
      }

      const globalKey = `global:${ecosystem}`;
      let globalRows: BrowseRow[] = [];
      if (liveInv) {
        globalRows = applyKindAndQuery(
          inventoryToRows(liveInv, ecosystem, title, locale).map((r) => ({
            ...r,
            id: `g:${ecosystem}:${r.id}`,
          })),
        );
      }
      const globalSection: BrowseSection = {
        key: globalKey,
        title: locale === "zh" ? "用户全局目录" : "Global user directory",
        rows: globalRows,
      };

      const projectSections: BrowseSection[] = [];
      for (const { path, inv } of agentProjectScans) {
        if (!inv) continue;
        const scoped = filterInventoryForAgent(ecosystem, inv);
        if (inventoryAssetCount(scoped) === 0) continue;
        const bn = folderBasename(path);
        let rows = inventoryToRows(scoped, "project", bn, locale).map((r) => ({
          ...r,
          id: `a:${ecosystem}:${path}:${r.id}`,
        }));
        rows = applyKindAndQuery(rows);
        projectSections.push({ key: path, title: bn, rows });
      }
      projectSections.sort((a, b) => b.rows.length - a.rows.length);
      const allForCounts = [globalSection, ...projectSections];
      const scenarioCounts = scenarioCountsFromRows(
        allForCounts.flatMap((s) => s.rows),
      );
      const customCatCounts = computeCustomCategoryCounts(
        allForCounts.flatMap((s) => s.rows),
      );
      const globalSectionFiltered: BrowseSection = {
        ...globalSection,
        rows: applyScenarioFilter(globalSection.rows),
      };
      const projectSectionsNonEmpty = projectSections
        .map((s) => ({ ...s, rows: applyScenarioFilter(s.rows) }))
        .filter((s) => s.rows.length > 0);

      return {
        sections: [globalSectionFiltered, ...projectSectionsNonEmpty],
        scenarioCounts,
        customCategoryCounts: customCatCounts,
        agentOptions: [],
      };
    }

    return emptyResult;
  }, [
    dataSet,
    ecosystem,
    filter,
    scenario,
    query,
    liveInv,
    liveLoading,
    title,
    projectRoot,
    projectInv,
    projectFailed,
    projectLoading,
    aggregateSnapshot,
    aggregateLoading,
    agentProjectScans,
    locale,
    aggregateAssetsTab,
    aggregateAgentFilter,
    agentAssetsTab,
    aggregateMineKind,
    mySkillsLib,
    promptLibrary,
    mineBriefMap,
    mineScenarioMap,
    reclassifyMode,
    customCategories,
    customScenario,
  ]);

  const aggregateAgentFilterLabel = useMemo(() => {
    if (aggregateAgentFilter.length === 0) {
      return locale === "zh" ? "Agent 筛选" : "Agent filter";
    }
    if (aggregateAgentFilter.length === 1) {
      const selected = agentOptions.find((option) => option.id === aggregateAgentFilter[0]);
      return selected?.label ?? (locale === "zh" ? "已选 1 个 Agent" : "1 agent selected");
    }
    return locale === "zh"
      ? `已选 ${aggregateAgentFilter.length} 个 Agent`
      : `${aggregateAgentFilter.length} agents selected`;
  }, [agentOptions, aggregateAgentFilter, locale]);

  const listedTotal = sections.reduce((n, s) => n + s.rows.length, 0);

  const visibleSections = useMemo(() => {
    if (dataSet !== "aggregate") return sections;
    let remaining = aggregateVisibleCount;
    const next: BrowseSection[] = [];
    for (const section of sections) {
      if (remaining <= 0) {
        next.push({ ...section, rows: [] });
        continue;
      }
      const rows = section.rows.slice(0, remaining);
      remaining -= rows.length;
      next.push({ ...section, rows });
    }
    return next.filter((section) => section.title || section.rows.length > 0);
  }, [aggregateVisibleCount, dataSet, sections]);

  const visibleAggregateTotal = visibleSections.reduce((n, s) => n + s.rows.length, 0);

  useEffect(() => {
    setAggregateVisibleCount(AGGREGATE_RENDER_PAGE_SIZE);
  }, [
    dataSet,
    aggregateAssetsTab,
    aggregateAgentFilter,
    aggregateMineKind,
    filter,
    scenario,
    query,
    customScenario,
    customCategories,
    mineBriefMap,
    mineScenarioMap,
  ]);

  useEffect(() => {
    if (dataSet !== "aggregate") return;
    if (visibleAggregateTotal >= listedTotal) return;
    const node = aggregateLoadMoreRef.current;
    if (!node) return;

    const loadMore = () => {
      setAggregateVisibleCount((count) =>
        Math.min(count + AGGREGATE_RENDER_PAGE_SIZE, listedTotal),
      );
    };

    if (typeof IntersectionObserver === "undefined") {
      const onScroll = () => {
        if (window.innerHeight + window.scrollY >= document.body.offsetHeight - 240) {
          loadMore();
        }
      };
      window.addEventListener("scroll", onScroll, { passive: true });
      onScroll();
      return () => window.removeEventListener("scroll", onScroll);
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) loadMore();
      },
      { rootMargin: "360px 0px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [dataSet, listedTotal, visibleAggregateTotal]);

  useEffect(() => {
    if (dataSet !== "skills" && dataSet !== "project") {
      setExpandedSectionKeys(new Set());
      return;
    }
    const titled = sections.filter((s) => s.title);
    const validKeys = new Set(titled.map((s) => s.key));

    setExpandedSectionKeys((prev) => {
      if (titled.length === 0) {
        const next = new Set<string>();
        return prev.size === 0 ? prev : next;
      }
      if (titled.length === 1) {
        const only = titled[0]!.key;
        return prev.size === 1 && prev.has(only)
          ? prev
          : new Set([only]);
      }
      const next = new Set<string>();
      for (const k of prev) {
        if (validKeys.has(k)) next.add(k);
      }
      return stringSetsEqual(prev, next) ? prev : next;
    });
  }, [dataSet, sections]);

  const copyMenuSections = useMemo(
    () => {
      if (skillCopyTargetModalRow?.kind === "prompt") {
        return buildPromptApplyMenuSections(detectedAgentTargets);
      }
      return buildCopySkillMenuSections({
        dataSet,
        ecosystem,
        projectRoot,
        projectPaths,
        agentProjectScanPaths: agentProjectScans.map((s) => s.path),
        userCustomAgentIds,
      });
    },
    [
      dataSet,
      ecosystem,
      projectRoot,
      projectPaths,
      agentProjectScans,
      skillCopyTargetModalRow?.kind,
      detectedAgentTargets,
      userCustomAgentIds,
    ],
  );

  const agentAllCount = useMemo(() => {
    if (dataSet !== "skills" || !ecosystem) return listedTotal;
    const globalCount = liveInv ? inventoryAssetCount(liveInv) : 0;
    const projectCount = agentProjectScans.reduce((sum, scan) => {
      if (!scan.inv) return sum;
      return sum + inventoryAssetCount(filterInventoryForAgent(ecosystem, scan.inv));
    }, 0);
    return globalCount + projectCount;
  }, [agentProjectScans, dataSet, ecosystem, listedTotal, liveInv]);
  const agentMineCount =
    dataSet === "skills" ? (liveInv?.skills.filter(isCsSkillEntry).length ?? 0) : 0;
  const publishedPromptCount =
    promptLibrary?.items.filter((item) => cpCommandForPrompt(item) !== null).length ?? 0;
  const aggregateMineCount = (mySkillsLib?.items.length ?? 0) + publishedPromptCount;

  const mySkillPathSet = useMemo(() => {
    const paths = new Set<string>();
    for (const item of mySkillsLib?.items ?? []) {
      const libraryPath = normalizeSkillPathForCompare(item.path);
      const sourcePath = normalizeSkillPathForCompare(item.sourcePath);
      if (libraryPath) paths.add(libraryPath);
      if (sourcePath) paths.add(sourcePath);
    }
    return paths;
  }, [mySkillsLib]);

  const assetsTotalBadge = useMemo(() => {
    if (dataSet !== "aggregate") return listedTotal;
    const allCount = aggregateSnapshot
      ? [...aggregateSnapshot.agents, ...aggregateSnapshot.projects].reduce(
          (n, a) => n + (a.inv ? a.inv.skills.length + a.inv.mcp.length + a.inv.rules.length : 0),
          0,
        )
      : 0;
    return allCount + aggregateMineCount;
  }, [dataSet, listedTotal, aggregateSnapshot, aggregateMineCount]);

  const toggleSectionExpanded = (sectionKey: string) => {
    setExpandedSectionKeys((prev) => {
      const next = new Set(prev);
      if (next.has(sectionKey)) next.delete(sectionKey);
      else next.add(sectionKey);
      return next;
    });
  };

  const openDetail = (item: BrowseRow) => {
    setSelectedEntry({
      id: item.id,
      kind: item.kind,
      title: item.title,
      description: item.desc,
      path: item.sourcePath,
      skillExtraFiles: item.skillExtraFiles,
    });
  };

  const addCardSkillToMine = (item: BrowseRow) => {
    const src = item.sourcePath?.trim();
    const pathKey = normalizeSkillPathForCompare(src);
    if (!src || !pathKey || mySkillAddPendingPaths.has(pathKey) || mySkillPathSet.has(pathKey)) {
      return;
    }

    setMySkillAddPendingPaths((prev) => new Set(prev).add(pathKey));
    void (async () => {
      try {
        const added = await addSkillToMyLibrary(src);
        clearAggregateSnapshotCache();
        setMySkillsLib((prev) => {
          const base: MySkillsLibraryFile = prev ?? { version: 1, items: [] };
          const addedPath = normalizeSkillPathForCompare(added.path);
          const addedSourcePath = normalizeSkillPathForCompare(added.sourcePath);
          const exists = base.items.some((existing) => {
            return (
              existing.id === added.id ||
              (!!addedPath && normalizeSkillPathForCompare(existing.path) === addedPath) ||
              (!!addedSourcePath &&
                normalizeSkillPathForCompare(existing.sourcePath) === addedSourcePath)
            );
          });
          if (exists) {
            return {
              ...base,
              items: base.items.map((existing) =>
                existing.id === added.id ? added : existing,
              ),
            };
          }
          return { ...base, items: [...base.items, added] };
        });
        setShellToast({
          at: Date.now(),
          message: locale === "zh" ? "已添加到「我的」" : "Added to My skills",
        });
      } catch (err) {
        window.alert(
          `${locale === "zh" ? "添加到我的失败" : "Add to My skills failed"}: ${String(err)}`,
        );
      } finally {
        setMySkillAddPendingPaths((prev) => {
          const next = new Set(prev);
          next.delete(pathKey);
          return next;
        });
      }
    })();
  };

  const findMySkillAppliedLocations = (row: BrowseRow): string[] => {
    const out: string[] = [];
    const seen = new Set<string>();
    const pushLocation = (label: string, path: string) => {
      const key = normalizeSkillPathForCompare(path) || label;
      if (seen.has(key)) return;
      seen.add(key);
      out.push(label);
    };
    const push = (label: string, asset: AssetEntry) => {
      if (!rowMatchesAppliedSkill(row, asset)) return;
      pushLocation(label, asset.path);
    };

    for (const location of readMySkillAppliedLocations()[row.sourceId] ?? []) {
      pushLocation(location.label, location.path);
    }

    for (const agent of aggregateSnapshot?.agents ?? []) {
      for (const asset of agent.inv?.skills ?? []) {
        push(
          locale === "zh"
            ? `${agent.title} · 用户全局 · ${normalizedPathBasename(asset.path)}`
            : `${agent.title} · Global · ${normalizedPathBasename(asset.path)}`,
          asset,
        );
      }
    }

    for (const project of aggregateSnapshot?.projects ?? []) {
      const projectName = folderBasename(project.path);
      for (const asset of project.inv?.skills ?? []) {
        const agentId = inferAgentIdFromAssetPath(asset.path) ?? "__other__";
        push(
          `${projectName} · ${agentLabelForId(agentId, locale)} · ${normalizedPathBasename(asset.path)}`,
          asset,
        );
      }
    }

    if (ecosystem && liveInv) {
      for (const asset of liveInv.skills) {
        push(
          locale === "zh"
            ? `${agentLabelForId(ecosystem, locale)} · 用户全局 · ${normalizedPathBasename(asset.path)}`
            : `${agentLabelForId(ecosystem, locale)} · Global · ${normalizedPathBasename(asset.path)}`,
          asset,
        );
      }
    }

    if (ecosystem) {
      for (const scan of agentProjectScans) {
        const inv = scan.inv ? filterInventoryForAgent(ecosystem, scan.inv) : null;
        for (const asset of inv?.skills ?? []) {
          push(
            `${folderBasename(scan.path)} · ${agentLabelForId(ecosystem, locale)} · ${normalizedPathBasename(asset.path)}`,
            asset,
          );
        }
      }
    }

    if (projectRoot && projectInv) {
      for (const asset of projectInv.skills) {
        const agentId = inferAgentIdFromAssetPath(asset.path) ?? "__other__";
        push(
          `${folderBasename(projectRoot)} · ${agentLabelForId(agentId, locale)} · ${normalizedPathBasename(asset.path)}`,
          asset,
        );
      }
    }

    return out;
  };

  const onCardContextMenu = (e: MouseEvent, item: BrowseRow) => {
    const p = item.sourcePath?.trim();
    if (!p && item.kind !== "prompt") return;
    e.preventDefault();
    e.stopPropagation();
    const pad = 8;
    const approxW = 240;
    const skillPath = item.sourcePath?.trim() ?? "";
    const isMineRow = item.id.startsWith("mine:");
    const skillHasDelete =
      item.kind === "skill" &&
      !!skillPath &&
      skillBrowsePathIsDeletableFolder(skillPath) &&
      !isMineRow;
    const approxH =
      item.kind === "prompt"
        ? 80
        : item.kind === "skill"
        ? isMineRow
          ? 132
          : skillHasDelete
            ? 176
            : 132
        : 48;
    const vw = typeof window !== "undefined" ? window.innerWidth : e.clientX;
    const vh = typeof window !== "undefined" ? window.innerHeight : e.clientY;
    const x = Math.min(Math.max(pad, e.clientX), Math.max(pad, vw - approxW - pad));
    const y = Math.min(Math.max(pad, e.clientY), Math.max(pad, vh - approxH - pad));
    setCardContextMenu({ x, y, row: item });
  };

  function cardHoverTitle(item: BrowseRow): string | undefined {
    const d = item.desc.trim();
    return d.length > 0 ? d : undefined;
  }

  function renderBrowseCard(item: BrowseRow) {
    const skillPathKey = normalizeSkillPathForCompare(item.sourcePath);
    const canAddToMine =
      item.kind === "skill" &&
      !!skillPathKey &&
      !item.id.startsWith("mine:") &&
      !(dataSet === "aggregate" && aggregateAssetsTab === "mine");
    const addToMinePending = canAddToMine && mySkillAddPendingPaths.has(skillPathKey);
    const addedToMine = canAddToMine && mySkillPathSet.has(skillPathKey);
    return (
      <article
        key={item.id}
        className="skill-card"
        title={cardHoverTitle(item)}
        onClick={() => openDetail(item)}
        onContextMenu={
          item.sourcePath?.trim() || item.kind === "prompt"
            ? (e) => onCardContextMenu(e, item)
            : undefined
        }
        style={{ cursor: "pointer" }}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            openDetail(item);
          }
        }}
      >
        <div className="skill-card__title-row">
          <span
            className={`skill-card__radio skill-card__radio--${item.descSource}`}
            aria-hidden
            title={item.descSource === "ai" ? (locale === "zh" ? "AI 缩略介绍" : "AI brief") : (locale === "zh" ? "原始描述" : "Source description")}
          />
          <span className="skill-card__title">{item.title}</span>
          {item.csCommand || item.cpCommand ? (
            <span className="skill-card__command">{item.csCommand ?? item.cpCommand}</span>
          ) : null}
          <span
            className={`skill-card__kind skill-card__kind--${item.kind}`}
            aria-label={`${locale === "zh" ? "类型" : "Type"}: ${
              item.kind === "prompt" ? "Prompt" : FILTER_LABEL[item.kind]
            }`}
          >
            {item.kind === "prompt" ? "Prompt" : FILTER_LABEL[item.kind]}
          </span>
        </div>
        <p className="skill-card__desc">{item.desc}</p>
        {(dataSet === "aggregate" && item.tags.length > 0) || canAddToMine ? (
          <div className="skill-card__footer">
            {dataSet === "aggregate" && item.tags.length > 0 ? (
              <div className="skill-card__tags" aria-label={locale === "zh" ? "来源标签" : "Source tags"}>
                {item.tags.map((t, i) => (
                  <span key={`${item.id}-tag-${i}`} className="skill-card__tag">
                    {t}
                  </span>
                ))}
              </div>
            ) : null}
            {canAddToMine ? (
              <div className="skill-card__actions">
                <button
                  type="button"
                  className={`skill-card__mine-button${
                    addedToMine ? " skill-card__mine-button--added" : ""
                  }`}
                  disabled={addToMinePending || addedToMine}
                  aria-pressed={addedToMine}
                  aria-label={
                    addedToMine
                      ? locale === "zh"
                        ? "已添加到我的"
                        : "Added to My skills"
                      : addToMinePending
                        ? locale === "zh"
                          ? "正在添加到我的"
                          : "Adding to My skills"
                        : locale === "zh"
                          ? "添加到我的"
                          : "Add to My skills"
                  }
                  title={
                    addedToMine
                      ? locale === "zh"
                        ? "已添加到我的"
                        : "Added to My skills"
                      : locale === "zh"
                        ? "添加到我的"
                        : "Add to My skills"
                  }
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    addCardSkillToMine(item);
                  }}
                >
                  <span aria-hidden>{addedToMine ? "✓" : addToMinePending ? "…" : "+"}</span>
                </button>
              </div>
            ) : null}
          </div>
        ) : null}
      </article>
    );
  }

  return (
    <>
      <div className="page-header">
        <div className="page-header__title-bar">
          <div className="page-title__row">
            <h2>{title}</h2>
            {dataSet === "aggregate" ? (
              <>
                <span className="count-badge">{assetsTotalBadge}</span>
                <div
                  className="seg page-header__assets-seg"
                  role="tablist"
                  aria-label={
                    locale === "zh" ? "资产范围" : "Asset scope"
                  }
                >
                  <button
                    type="button"
                    role="tab"
                    aria-selected={aggregateAssetsTab === "all"}
                    className={`seg__item${aggregateAssetsTab === "all" ? " active" : ""}`}
                    onClick={() => setAggregateAssetsTab("all")}
                  >
                    {locale === "zh" ? "全部" : "All"}
                  </button>
                  <button
                    type="button"
                    role="tab"
                    aria-selected={aggregateAssetsTab === "mine"}
                    className={`seg__item${aggregateAssetsTab === "mine" ? " active" : ""}`}
                    onClick={() => setAggregateAssetsTab("mine")}
                  >
                    {locale === "zh" ? "我的" : "Mine"}
                    <span className="skill-copy-dialog__tab-badge">
                      {aggregateMineCount}
                    </span>
                  </button>
                </div>
              </>
            ) : dataSet === "skills" ? (
              <>
                <span className="count-badge">{agentAllCount}</span>
                <div
                  className="seg page-header__assets-seg"
                  role="tablist"
                  aria-label={locale === "zh" ? "Agent 资产范围" : "Agent asset scope"}
                >
                  <button
                    type="button"
                    role="tab"
                    aria-selected={agentAssetsTab === "all"}
                    className={`seg__item${agentAssetsTab === "all" ? " active" : ""}`}
                    onClick={() => setAgentAssetsTab("all")}
                  >
                    {locale === "zh" ? "全部" : "All"}
                  </button>
                  <button
                    type="button"
                    role="tab"
                    aria-selected={agentAssetsTab === "mine"}
                    className={`seg__item${agentAssetsTab === "mine" ? " active" : ""}`}
                    onClick={() => setAgentAssetsTab("mine")}
                  >
                    {locale === "zh" ? "我的" : "Mine"}
                    <span className="skill-copy-dialog__tab-badge">
                      {agentMineCount}
                    </span>
                  </button>
                </div>
              </>
            ) : (
              <span className="count-badge">{listedTotal}</span>
            )}
            {dataSet === "aggregate" && aggregateAssetsTab === "mine" ? (
              <button
                type="button"
                className="page-header__primary-action"
                disabled={mySkillsImportBusy || refreshBusy}
                onClick={() => {
                  void (async () => {
                    setMySkillsImportBusy(true);
                    try {
                      const picked = await open({
                        directory: true,
                        multiple: false,
                      });
                      const dir =
                        typeof picked === "string"
                          ? picked
                          : Array.isArray(picked)
                            ? picked[0] ?? null
                            : null;
                      if (!dir?.trim()) return;
                      await addSkillToMyLibrary(dir.trim());
                      setRefreshKey((k) => k + 1);
                      setShellToast({
                        at: Date.now(),
                        message:
                          locale === "zh"
                            ? "已导入到「我的」"
                            : "Imported to My skills",
                      });
                    } catch (e) {
                      window.alert(
                        `${locale === "zh" ? "导入失败" : "Import failed"}: ${String(e)}`,
                      );
                    } finally {
                      setMySkillsImportBusy(false);
                    }
                  })();
                }}
              >
                <span className="page-header__primary-action-icon" aria-hidden>
                  <svg viewBox="0 0 24 24" width="15" height="15" fill="none">
                    <path d="M12 5v14M5 12h14" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                  </svg>
                </span>
                <span>{locale === "zh" ? "导入技能文件夹" : "Import Folder"}</span>
              </button>
            ) : null}
          </div>
          <PageRefreshButton
            onClick={onRefreshInventory}
            disabled={refreshBusy}
            spinning={refreshBusy}
            label={locale === "zh" ? "重新扫描并加载" : "Rescan and reload"}
          />
        </div>
        {subtitle ? (
          <p className="muted" style={{ margin: "0.35rem 0 0", fontSize: "0.88rem" }}>
            {subtitle}
          </p>
        ) : null}
        {ecosystem && dataSet === "skills" && agentAssetsTab === "all" && (liveLoading || liveFailed) ? (
          <p className="muted" style={{ margin: "0.35rem 0 0", fontSize: "0.85rem" }}>
            {liveLoading
              ? locale === "zh"
                ? "正在读取该 Agent 的用户级全局目录与侧栏已添加项目…"
                : "Loading this agent's global directory and added projects…"
              : locale === "zh"
                ? "无法读取用户级全局目录：仍可查看侧栏项目中归属该 Agent 的配置；请在桌面端运行或检查权限。"
                : "Failed to read global directory. You can still view project-scoped entries."}
          </p>
        ) : null}
        {ecosystem && dataSet === "skills" && agentAssetsTab === "mine" ? (
          <p className="muted" style={{ margin: "0.35rem 0 0", fontSize: "0.85rem" }}>
            {locale === "zh"
              ? "这里筛选当前 Agent 用户全局目录中已安装的 /cs-* Skills。需要新增时，先在「全部」页的「我的」中应用到 Agent。"
              : "Filters /cs-* Skills already installed in this agent's global directory. To add one, apply it from Mine in the All page."}
          </p>
        ) : null}
        {dataSet === "project" &&
        projectRoot &&
        (projectLoading || projectFailed) ? (
          <p className="muted" style={{ margin: "0.35rem 0 0", fontSize: "0.85rem" }}>
            {projectLoading
              ? locale === "zh"
                ? "正在扫描所选目录下各 Agent skills 目录、MCP（JSON）与规则文件…"
                : "Scanning agent skills, MCP JSON and rules in selected directory…"
              : locale === "zh"
                ? "无法扫描该目录：请在 AIControls 桌面端运行，或检查路径与权限。"
                : "Failed to scan this directory. Check desktop runtime and permissions."}
          </p>
        ) : null}
        {dataSet === "aggregate" ? (
          <p className="muted" style={{ margin: "0.35rem 0 0", fontSize: "0.85rem" }}>
            {aggregateAssetsTab === "mine"
              ? locale === "zh"
                ? "这里包含 AIControls「我的」Skills，以及 Prompt 库中已发布为 /cp-* 的 Prompts。"
                : "Includes AIControls My Skills and prompts published as /cp-* from Prompt Library."
              : aggregateLoading || aggregateSnapshot === null
                ? locale === "zh"
                  ? "正在汇总各 Agent 用户级全局目录与侧栏已添加项目…"
                  : "Aggregating global assets and added projects…"
                : aggregateSnapshot.anyInventoryFailed
                  ? locale === "zh"
                    ? "部分目录读取失败，已展示可用结果。"
                    : "Some directories failed to load; showing available results."
                  : locale === "zh"
                    ? "包含所有已识别 Agent 的全局 Skills、MCP、Rules，以及「全部项目」中各目录的扫描结果。"
                    : "Includes global assets from detected agents and scanned results from all projects."}
          </p>
        ) : null}
      </div>

      <div className="toolbar">
        <div className="toolbar__stack">
          <div className="toolbar__row">
            <label className="search" htmlFor={searchFieldId}>
              <span className="search__icon" aria-hidden>
                ⌕
              </span>
              <input
                id={searchFieldId}
                className="search__input"
                type="search"
                placeholder={
                  locale === "zh"
                    ? "搜索标题、描述或路径…"
                    : "Search title, description, or path…"
                }
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                autoComplete="off"
              />
            </label>
            {dataSet === "aggregate" && aggregateAssetsTab === "all" ? (
              <details ref={aggregateAgentFilterRef} className="agent-filter">
                <summary
                  className={`agent-filter__button${
                    aggregateAgentFilter.length > 0 ? " active" : ""
                  }${agentOptions.length === 0 ? " disabled" : ""}`}
                  aria-disabled={agentOptions.length === 0}
                  onClick={(event) => {
                    if (agentOptions.length === 0) event.preventDefault();
                  }}
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden
                  >
                    <path d="M3 5h18" />
                    <path d="M7 12h10" />
                    <path d="M10 19h4" />
                  </svg>
                  <span>{aggregateAgentFilterLabel}</span>
                  {aggregateAgentFilter.length > 0 ? (
                    <span className="agent-filter__badge">{aggregateAgentFilter.length}</span>
                  ) : null}
                </summary>
                <div className="agent-filter__menu" role="group" aria-label={locale === "zh" ? "Agent 多选筛选" : "Agent multi-select filter"}>
                  <div className="agent-filter__menu-head">
                    <span>{locale === "zh" ? "选择 Agent" : "Select agents"}</span>
                    {aggregateAgentFilter.length > 0 ? (
                      <button
                        type="button"
                        className="agent-filter__clear"
                        onClick={() => setAggregateAgentFilter([])}
                      >
                        {locale === "zh" ? "清空" : "Clear"}
                      </button>
                    ) : null}
                  </div>
                  <div className="agent-filter__options">
                    {agentOptions.map((option) => {
                      const checked = aggregateAgentFilter.includes(option.id);
                      return (
                        <label key={option.id} className="agent-filter__option">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={() => toggleAggregateAgentFilter(option.id)}
                          />
                          <span className="agent-filter__option-label">{option.label}</span>
                          <span className="agent-filter__option-count">{option.count}</span>
                        </label>
                      );
                    })}
                  </div>
                </div>
              </details>
            ) : null}
            {dataSet === "aggregate" && aggregateAssetsTab === "mine" ? (
              <div
                className="seg"
                role="tablist"
                aria-label={locale === "zh" ? "我的资产类型筛选" : "Mine asset type filter"}
              >
                {(["all", "skill", "prompt"] as MineKindFilter[]).map((k) => (
                  <button
                    key={k}
                    type="button"
                    role="tab"
                    aria-selected={aggregateMineKind === k}
                    className={`seg__item${aggregateMineKind === k ? " active" : ""}`}
                    onClick={() => setAggregateMineKind(k)}
                  >
                    {k === "all" ? (locale === "zh" ? "全部" : "All") : MINE_KIND_LABEL[k]}
                  </button>
                ))}
              </div>
            ) : dataSet === "skills" && agentAssetsTab === "mine" ? (
              <span className="muted toolbar__mine-kind-hint">
                {locale === "zh" ? "仅 Skill" : "Skills only"}
              </span>
            ) : (
              <div
                className="seg"
                role="tablist"
                aria-label={locale === "zh" ? "类型筛选" : "Type filter"}
              >
                {SEGMENT_KEYS.map((k) => (
                  <button
                    key={k}
                    type="button"
                    role="tab"
                    aria-selected={filter === k}
                    className={`seg__item${filter === k ? " active" : ""}`}
                    onClick={() => setFilter(k)}
                  >
                    {FILTER_LABEL[k]}
                  </button>
                ))}
              </div>
            )}
          </div>
          <div
            className="scenario-strip"
            role="tablist"
            aria-label={locale === "zh" ? "场景分类" : "Scenario filter"}
          >
            {((reclassifyMode === "reviewing" || reclassifyMode === "generating" || reclassifyMode === "applying") || (reclassifyMode === "idle" && customCategories && customCategories.length > 0)) && customCategoryCounts ? (
              <>
                <button
                  type="button"
                  role="tab"
                  aria-selected={customScenario === "all"}
                  title={locale === "zh" ? "展示全部" : "Show all"}
                  className={`scenario-chip${customScenario === "all" ? " active" : ""}`}
                  onClick={() => setCustomScenario("all")}
                >
                  {locale === "zh" ? "全部" : "All"} ({customCategoryCounts.all})
                </button>
                {customCategories!.map((cat) => {
                  const label = customCategoryLabel(cat, locale);
                  return (
                    <button
                      key={cat.slug}
                      type="button"
                      role="tab"
                      aria-selected={customScenario === cat.slug}
                      title={label}
                      className={`scenario-chip scenario-chip--custom${customScenario === cat.slug ? " active" : ""}`}
                      onClick={() => setCustomScenario(cat.slug)}
                    >
                      {label} ({customCategoryCounts[cat.slug] ?? 0})
                    </button>
                  );
                })}
              </>
            ) : (
              <>
                <button
                  type="button"
                  role="tab"
                  aria-selected={scenario === "all"}
                  title={locale === "zh" ? "展示全部 Skill、MCP 与 Rules" : "Show all Skills, MCP and Rules"}
                  className={`scenario-chip${scenario === "all" ? " active" : ""}`}
                  onClick={() => setScenario("all")}
                >
                  {getScenarioLabel(locale, "all")} ({scenarioCounts.all})
                </button>
                {SCENARIO_ORDER.map((key) => (
                  <button
                    key={key}
                    type="button"
                    role="tab"
                    aria-selected={scenario === key}
                    title={getScenarioHint(locale, key)}
                    className={`scenario-chip${scenario === key ? " active" : ""}`}
                    onClick={() => setScenario(key)}
                  >
                    {getScenarioLabel(locale, key)} ({scenarioCounts[key]})
                  </button>
                ))}
              </>
            )}
            {dataSet === "aggregate" && aggregateAssetsTab === "all" && (
              <>
                {reclassifyMode === "idle" && (
                  <>
                    <button
                      type="button"
                      className="scenario-chip scenario-chip--action"
                      title={locale === "zh" ? "重新分类" : "Reclassify"}
                      disabled={aiScenarioBusy || aiBriefBusy || aggregateLoading}
                      onClick={handleReclassifyStart}
                    >
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                        <path d="M16 3h5v5" /><path d="M4 20L21 3" /><path d="M21 16v5h-5" /><path d="M15 15l6 6" /><path d="M4 4l5 5" />
                      </svg>
                    </button>
                    {customCategories && customCategories.length > 0 && (
                      <button
                        type="button"
                        className="scenario-chip scenario-chip--action"
                        title={locale === "zh" ? "重置为默认分类" : "Reset to default categories"}
                        onClick={handleResetCategories}
                      >
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                          <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /><path d="M3 3v5h5" />
                        </svg>
                      </button>
                    )}
                  </>
                )}
                {reclassifyMode === "generating" && (
                  <span className="scenario-chip scenario-chip--status">
                    <span className="scenario-chip--status__pulse" />
                    {locale === "zh" ? "生成中…" : "Generating…"}
                  </span>
                )}
                {reclassifyMode === "reviewing" && (
                  <>
                    <button
                      type="button"
                      className="scenario-chip scenario-chip--action"
                      title={locale === "zh" ? "重新生成" : "Regenerate"}
                      disabled={aiScenarioBusy || aiBriefBusy || aggregateLoading}
                      onClick={handleReclassifyStart}
                    >
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                        <path d="M16 3h5v5" /><path d="M4 20L21 3" /><path d="M21 16v5h-5" /><path d="M15 15l6 6" /><path d="M4 4l5 5" />
                      </svg>
                    </button>
                    <button
                      type="button"
                      className="scenario-chip scenario-chip--confirm"
                      onClick={handleReclassifyConfirm}
                    >
                      {locale === "zh" ? "确定" : "Confirm"}
                    </button>
                    <button
                      type="button"
                      className="scenario-chip scenario-chip--cancel"
                      onClick={handleReclassifyCancel}
                    >
                      {locale === "zh" ? "取消" : "Cancel"}
                    </button>
                  </>
                )}
                {reclassifyMode === "applying" && (
                  <span className="scenario-chip scenario-chip--status">
                    <span className="scenario-chip--status__pulse" />
                    {locale === "zh" ? "应用中…" : "Applying…"}
                  </span>
                )}
              </>
            )}
          </div>
          {dataSet === "aggregate" && aggregateAssetsTab === "all" ? null : null}
          {aiScenarioBusy || aiBriefBusy || mineAiBusy ? (
            <p
              className="muted toolbar__deepseek-status"
              role="status"
              aria-live="polite"
            >
              {mineAiBusy
                ? locale === "zh"
                  ? "DeepSeek 正在为「我的」资产补全场景分类与卡片简介，请稍候…"
                  : "DeepSeek is classifying and summarizing Mine assets…"
                : aiScenarioBusy
                ? locale === "zh"
                  ? "DeepSeek 正在为尚未写入本地缓存的条目补全场景分类，请稍候…"
                  : "DeepSeek is classifying uncached entries…"
                : locale === "zh"
                  ? "DeepSeek 正在逐条生成中文缩略介绍（100字以内），请稍候…"
                  : "DeepSeek is generating English briefs (<=100 chars) …"}
            </p>
          ) : null}
        </div>
      </div>

      {visibleSections.map((sec) => {
        if (!sec.title) {
          return (
            <div key={sec.key} className="skill-grid">
              {sec.rows.map((item) => renderBrowseCard(item))}
            </div>
          );
        }

        const expanded = expandedSectionKeys.has(sec.key);
        const frag = sectionIdSafeFragment(sec.key);
        const headerId = `${browseSectionDomPrefix}-h-${frag}`;
        const panelId = `${browseSectionDomPrefix}-p-${frag}`;

        return (
          <section key={sec.key} className="browse-section">
            <div className="browse-section__header">
              <button
                type="button"
                id={headerId}
                className="browse-section__toggle"
                aria-expanded={expanded}
                aria-controls={panelId}
                onClick={() => toggleSectionExpanded(sec.key)}
              >
                <span className="browse-section__chevron" aria-hidden>
                  ▾
                </span>
                <span className="browse-section__title">{sec.title}</span>
                <span className="count-badge">{sec.rows.length}</span>
              </button>
            </div>
            {expanded ? (
              <div
                id={panelId}
                className="skill-grid"
                role="region"
                aria-labelledby={headerId}
              >
                {sec.rows.map((item) => renderBrowseCard(item))}
              </div>
            ) : null}
          </section>
        );
      })}

      {dataSet === "aggregate" && visibleAggregateTotal < listedTotal ? (
        <div
          ref={aggregateLoadMoreRef}
          className="muted"
          style={{ padding: "1rem 0 1.5rem", textAlign: "center" }}
          role="status"
        >
          {locale === "zh"
            ? `继续向下滚动加载更多（${visibleAggregateTotal}/${listedTotal}）`
            : `Scroll down to load more (${visibleAggregateTotal}/${listedTotal})`}
        </div>
      ) : null}

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
              aria-label={locale === "zh" ? "卡片操作" : "Card actions"}
            >
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                onClick={() => {
                  const p = cardContextMenu.row.sourcePath?.trim();
                  if (p) void revealPathInFolder(p);
                  setCardContextMenu(null);
                }}
              >
                {locale === "zh" ? "在所在目录中显示" : "Show in folder"}
              </button>
              {!cardContextMenu.row.id.startsWith("mine:") ? (
              <button
                type="button"
                role="menuitem"
                className="card-context-menu__item"
                onClick={() => {
                  const row = cardContextMenu.row;
                  setCardContextMenu(null);
                  void (async () => {
                    const brief = await deepseekResummarizeAsset(
                      {
                        id: row.sourceId,
                        kind: row.kind,
                        title: row.title,
                        description: row.desc,
                        path: row.sourcePath ?? "",
                        active: row.active,
                      },
                      locale,
                    );
                    if (!brief) {
                      window.alert(locale === "zh" ? "重新生成简介失败" : "Failed to regenerate brief");
                      return;
                    }
                    const srcId = row.sourceId;
                    setLiveInv((prev) =>
                      prev ? patchEntryBriefInInventory(prev, srcId, locale, brief) : prev,
                    );
                    setAgentProjectScans((prev) =>
                      prev.map((item) => ({
                        ...item,
                        inv: item.inv
                          ? patchEntryBriefInInventory(item.inv, srcId, locale, brief)
                          : null,
                      })),
                    );
                    setProjectInv((prev) =>
                      prev ? patchEntryBriefInInventory(prev, srcId, locale, brief) : prev,
                    );
                    setAggregateSnapshot((prev) =>
                      prev
                        ? {
                            ...prev,
                            agents: prev.agents.map((a) => ({
                              ...a,
                              inv: a.inv
                                ? patchEntryBriefInInventory(a.inv, srcId, locale, brief)
                                : null,
                            })),
                            projects: prev.projects.map((p) => ({
                              ...p,
                              inv: p.inv
                                ? patchEntryBriefInInventory(p.inv, srcId, locale, brief)
                                : null,
                            })),
                          }
                        : prev,
                    );
                    setShellToast({
                      at: Date.now(),
                      message: locale === "zh" ? "已重新生成简介" : "Brief regenerated",
                    });
                  })();
                }}
              >
                {locale === "zh" ? "重新生成简介" : "Regenerate brief"}
              </button>
              ) : null}
              {cardContextMenu.row.kind === "skill" &&
              cardContextMenu.row.sourcePath?.trim() ? (
                <>
                  <button
                    type="button"
                    role="menuitem"
                    className="card-context-menu__item"
                    onClick={() => {
                      setSkillCopyTargetModalRow(cardContextMenu.row);
                      setCardContextMenu(null);
                    }}
                  >
                    {cardContextMenu.row.id.startsWith("mine:")
                      ? locale === "zh"
                        ? "应用到…"
                        : "Apply to…"
                      : locale === "zh"
                        ? "复制到…"
                        : "Copy to…"}
                  </button>
                  {cardContextMenu.row.id.startsWith("mine:") ? (
                    <button
                      type="button"
                      role="menuitem"
                      className="card-context-menu__item card-context-menu__item--danger"
                      onClick={() => {
                        const row = cardContextMenu.row;
                        const rawId = row.sourceId?.trim();
                        setCardContextMenu(null);
                        if (!rawId) return;
                        const appliedLocations = findMySkillAppliedLocations(row);
                        const locationPreview = appliedLocations
                          .slice(0, 6)
                          .map((location) => `• ${location}`)
                          .join("\n");
                        const overflowCount = appliedLocations.length - 6;
                        const ok = window.confirm(
                          appliedLocations.length > 0
                            ? locale === "zh"
                              ? `「${row.title}」已应用到以下位置：\n\n${locationPreview}${
                                  overflowCount > 0 ? `\n等 ${appliedLocations.length} 处` : ""
                                }\n\n从「我的」移除只会删除「我的」里的本地副本，不会删除这些已应用副本。继续移除？`
                              : `"${row.title}" has been applied to:\n\n${locationPreview}${
                                  overflowCount > 0 ? `\nand ${overflowCount} more` : ""
                                }\n\nRemoving it from My skills only deletes the local My copy. Applied copies will remain. Continue?`
                            : locale === "zh"
                              ? `从「我的」移除「${row.title}」？将删除本地副本。`
                              : `Remove "${row.title}" from My skills? The local copy will be deleted.`,
                        );
                        if (!ok) return;
                        void (async () => {
                          try {
                            await removeMySkill(rawId);
                            forgetMySkillAppliedLocations(rawId);
                            setRefreshKey((k) => k + 1);
                            setSelectedEntry((cur) =>
                              cur?.path?.trim() === row.sourcePath?.trim()
                                ? null
                                : cur,
                            );
                            setShellToast({
                              at: Date.now(),
                              message:
                                locale === "zh" ? "已从「我的」移除" : "Removed from My skills",
                            });
                          } catch (err) {
                            window.alert(
                              `${locale === "zh" ? "移除失败" : "Remove failed"}: ${String(err)}`,
                            );
                          }
                        })();
                      }}
                    >
                      {locale === "zh" ? "从「我的」移除…" : "Remove from My skills…"}
                    </button>
                  ) : skillBrowsePathIsDeletableFolder(
                      cardContextMenu.row.sourcePath?.trim() ?? "",
                    ) ? (
                    <button
                      type="button"
                      role="menuitem"
                      className="card-context-menu__item card-context-menu__item--danger"
                      onClick={() => {
                        const row = cardContextMenu.row;
                        const p = row.sourcePath?.trim();
                        if (!p) return;
                        const ok = window.confirm(
                          locale === "zh"
                            ? `确定要删除技能文件夹「${row.title}」吗？将删除整个文件夹及其中的文件，且无法撤销。`
                            : `Delete skill folder "${row.title}"? This removes all files and cannot be undone.`,
                        );
                        setCardContextMenu(null);
                        if (!ok) return;
                        void (async () => {
                          const r = await deleteSkillAtPath(p);
                          if ("error" in r) {
                            window.alert(`${locale === "zh" ? "删除失败" : "Delete failed"}: ${r.error}`);
                            return;
                          }
                          setSelectedEntry((cur) =>
                            cur?.path?.trim() === p ? null : cur,
                          );
                          setShellToast({ at: Date.now(), message: locale === "zh" ? "已删除" : "Deleted" });
                          onRefreshInventory();
                        })();
                      }}
                    >
                      {locale === "zh" ? "删除…" : "Delete…"}
                    </button>
                  ) : null}
                </>
              ) : null}
              {cardContextMenu.row.kind === "prompt" && cardContextMenu.row.cpCommand ? (
                <button
                  type="button"
                  role="menuitem"
                  className="card-context-menu__item"
                  onClick={() => {
                    setSkillCopyTargetModalRow(cardContextMenu.row);
                    setCardContextMenu(null);
                  }}
                >
                  {locale === "zh" ? "应用到 Agent…" : "Apply to Agent…"}
                </button>
              ) : null}
            </div>,
            document.body,
          )
        : null}

      {skillCopyTargetModalRow
        ? createPortal(
            <SkillCopyDestinationDialog
              row={skillCopyTargetModalRow}
              sections={copyMenuSections}
              dialogTitle={
                skillCopyTargetModalRow.kind === "prompt"
                  ? locale === "zh"
                    ? "应用到 Agent…"
                    : "Apply to Agent…"
                  : skillCopyTargetModalRow.id.startsWith("mine:")
                  ? locale === "zh"
                    ? "应用到…"
                    : "Apply to…"
                  : undefined
              }
              onClose={() => setSkillCopyTargetModalRow(null)}
              onChoose={(payload) => {
                const row = skillCopyTargetModalRow;
                if (!row) return;
                if (row.kind === "prompt") {
                  const segment = normalizePromptApplyCommandSegment(
                    row.promptCommandName ?? row.cpCommand ?? row.title ?? "",
                  );
                  if (!isValidAgentCommandSegmentInput(segment)) {
                    window.alert(agentCommandSegmentInvalidMessage(locale));
                    return;
                  }
                }
                const src = row.sourcePath?.trim();
                void (async () => {
                  setSkillCopyTargetModalRow(null);
                  if (row.kind === "prompt") {
                    if (payload.destKind !== "promptGlobal") return;
                    try {
                      await applyPromptCommandToAgent({
                        agentId: payload.agentId,
                        title: row.title,
                        prompt: row.promptText ?? row.desc,
                        commandName: row.promptCommandName ?? row.cpCommand ?? row.title,
                      });
                      setShellToast({
                        at: Date.now(),
                        message: locale === "zh" ? "已应用到 Agent" : "Applied to agent",
                      });
                    } catch (err) {
                      window.alert(
                        `${locale === "zh" ? "应用失败" : "Apply failed"}: ${String(err)}`,
                      );
                    }
                    return;
                  }
                  if (!src) return;
                  if (payload.destKind === "myLibrary") {
                    try {
                      await addSkillToMyLibrary(src);
                      setRefreshKey((k) => k + 1);
                      setShellToast({
                        at: Date.now(),
                        message:
                          locale === "zh"
                            ? "已复制到「我的」"
                            : "Copied to My skills",
                      });
                    } catch (err) {
                      window.alert(
                        `${locale === "zh" ? "复制到我的失败" : "Copy to My skills failed"}: ${String(err)}`,
                      );
                    }
                    return;
                  }
                  if (payload.destKind === "promptGlobal") return;

                  const r = await copySkillPackage({
                    sourcePath: src,
                    destKind: payload.destKind,
                    agentId: payload.agentId,
                    bucketIndex: payload.bucketIndex,
                    projectRoot:
                      payload.destKind === "project"
                        ? payload.projectRoot
                        : undefined,
                    onConflict: "suffix",
                    folderNamePrefix: row.id.startsWith("mine:")
                      ? row.mineSkillSourceKind === "prompt"
                        ? "cps-"
                        : "cs-"
                      : null,
                  });
                  if ("error" in r) {
                    window.alert(`${locale === "zh" ? "复制失败" : "Copy failed"}: ${r.error}`);
                    return;
                  }
                  const applied = row.id.startsWith("mine:");
                  if (applied) {
                    const appliedPath = normalizeSkillPathForCompare(r.path);
                    const appliedLabel =
                      payload.destKind === "project"
                        ? `${folderBasename(payload.projectRoot)} · ${agentLabelForId(payload.agentId, locale)} · ${normalizedPathBasename(appliedPath)}`
                        : locale === "zh"
                          ? `${agentLabelForId(payload.agentId, locale)} · 用户全局 · ${normalizedPathBasename(appliedPath)}`
                          : `${agentLabelForId(payload.agentId, locale)} · Global · ${normalizedPathBasename(appliedPath)}`;
                    rememberMySkillAppliedLocation(row.sourceId, {
                      label: appliedLabel,
                      path: appliedPath,
                    });
                    clearAggregateSnapshotCache();
                  }
                  setShellToast({
                    at: Date.now(),
                    message: applied
                      ? locale === "zh"
                        ? "已应用"
                        : "Applied"
                      : locale === "zh"
                        ? "复制成功"
                        : "Copied",
                  });
                })();
              }}
            />,
            document.body,
          )
        : null}

      {shellToast
        ? createPortal(
            <div className="toast-stack" role="status" aria-live="polite">
              <div className="toast toast--success">
                <span className="toast__symbol" aria-hidden>
                  ✓
                </span>
                <span className="toast__text">{shellToast.message}</span>
              </div>
            </div>,
            document.body,
          )
        : null}

      {/* Skill 详情面板 */}
      <SkillDetailPanel
        entry={selectedEntry}
        onClose={() => setSelectedEntry(null)}
      />

      {listedTotal === 0 &&
      !(ecosystem && dataSet === "skills" && agentAssetsTab === "all" && (liveLoading || liveFailed)) &&
      !(ecosystem && dataSet === "skills" && agentAssetsTab === "mine" && liveLoading) &&
      !(dataSet === "project" && projectLoading) &&
      !(
        dataSet === "aggregate" &&
        aggregateAssetsTab === "all" &&
        (aggregateLoading || aggregateSnapshot === null)
      ) &&
      !(dataSet === "aggregate" && aggregateAssetsTab === "mine" && mySkillsLoading) ? (
        <p className="muted" style={{ marginTop: "1rem" }}>
          {dataSet === "project"
            ? !projectRoot
              ? locale === "zh"
                ? "请先通过侧栏「添加项目」选择文件夹。"
                : "Please add a project folder from the sidebar first."
              : projectFailed
                ? null
                : locale === "zh"
                  ? "所选目录下未发现条目，或没有符合当前筛选的结果。"
                  : "No entries found in selected directory, or no matches for current filters."
            : dataSet === "aggregate"
              ? aggregateAssetsTab === "mine"
                ? locale === "zh"
                  ? "「我的」中暂无匹配资产。可导入 Skill，或在 Prompt 库中发布 /cp Prompt。"
                  : 'No matching assets in Mine. Import a Skill, or publish a /cp prompt from Prompt Library.'
                : locale === "zh"
                  ? "未发现任何条目，或没有符合当前筛选的结果。"
                  : "No entries found, or no matches for current filters."
              : ecosystem && dataSet === "skills"
                ? agentAssetsTab === "mine"
                  ? locale === "zh"
                    ? "当前 Agent 的用户全局目录中还没有 /cs-* Skills。请先从「全部」页的「我的」中应用到该 Agent。"
                    : "No /cs-* Skills are installed in this agent's global directory yet. Apply one from Mine in the All page first."
                  : locale === "zh"
                    ? "没有符合条件的全局条目。"
                    : "No matching global entries."
                : locale === "zh"
                  ? "没有符合当前筛选条件的条目。"
                  : "No entries match current filters."}
        </p>
      ) : null}
    </>
  );
}
