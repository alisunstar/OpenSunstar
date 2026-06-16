import {
  normalizeProjectPath,
  pathsReferToSameDir,
} from "./projectPathsStorage";

const AGENT_ORDER = [
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

const AGENT_UI_NAME: Record<string, string> = {
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

/** Relative skill roots; index = `bucketIndex` passed to the desktop command. */
const SKILL_BUCKET_REL: Record<string, readonly string[]> = {
  cursor: [".cursor/skills-cursor", ".cursor/skills"],
  claude: [".claude/skills"],
  codex: [".codex/skills"],
  hermes: [".hermes/skills"],
  openclaw: [".openclaw/skills"],
  trae: [".trae/skills"],
  qoder: [".qoder/skills", ".qoderwork/skills"],
  kiro: [".kiro/skills"],
  opencode: [".opencode/skills"],
};

/** 「复制到…」目标：Agent 目录或应用内「我的」技能库。 */
export type CopySkillTargetPayload =
  | {
      destKind: "global";
      agentId: string;
      bucketIndex: number;
    }
  | {
      destKind: "project";
      agentId: string;
      bucketIndex: number;
      projectRoot: string;
    }
  | {
      destKind: "myLibrary";
    }
  | {
      destKind: "promptGlobal";
      agentId: string;
    };

export type CopySkillMenuSection = {
  key: string;
  title: string;
  items: { id: string; label: string; payload: CopySkillTargetPayload }[];
};

function mineLibrarySection(_copyVerb: "复制" | "导入"): CopySkillMenuSection {
  return {
    key: "mine-library",
    title: "我的Skills",
    items: [
      {
        id: "mine-library-sink",
        label: "我的Skills",
        payload: { destKind: "myLibrary" },
      },
    ],
  };
}

function folderBasename(path: string): string {
  return path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "项目";
}

/** 侧栏项目 + 当前路由项目（URL ?path=）合并去重，避免只打开单一项目页时缺少条目。 */
function mergeProjectRootsForCopy(
  projectPaths: readonly string[],
  projectRoot?: string,
): string[] {
  const out: string[] = [];
  const push = (raw: string) => {
    const t = normalizeProjectPath(raw);
    if (!t) return;
    if (out.some((p) => pathsReferToSameDir(p, t))) return;
    out.push(t);
  };
  for (const p of projectPaths) push(p);
  if (projectRoot?.trim()) push(projectRoot);
  return out;
}

function globalItems(agentId: string): CopySkillMenuSection["items"] {
  if (agentId.startsWith("useragent-")) {
    return [
      {
        id: `g:${agentId}:0`,
        label: "自定义 · skills/",
        payload: {
          destKind: "global" as const,
          agentId,
          bucketIndex: 0,
        },
      },
    ];
  }
  const rels = SKILL_BUCKET_REL[agentId];
  if (!rels) return [];
  const agentName = AGENT_UI_NAME[agentId] ?? agentId;
  return rels.map((rel, bucketIndex) => ({
    id: `g:${agentId}:${bucketIndex}`,
    label: `${agentName} · ${rel}`,
    payload: {
      destKind: "global" as const,
      agentId,
      bucketIndex,
    },
  }));
}

export type PromptApplyAgentTarget = {
  id: string;
  label: string;
};

function promptGlobalItems(
  agents: readonly PromptApplyAgentTarget[],
): CopySkillMenuSection["items"] {
  return agents.map(({ id: agentId, label }) => {
    const agentName = AGENT_UI_NAME[agentId] ?? agentId;
    return {
      id: `prompt:${agentId}`,
      label: label || agentName,
      payload: {
        destKind: "promptGlobal" as const,
        agentId,
      },
    };
  });
}

export function buildPromptApplyMenuSections(
  agents: readonly PromptApplyAgentTarget[],
): CopySkillMenuSection[] {
  return [
    {
      key: "global-prompt-agents",
      title: "应用到 · Agent",
      items: promptGlobalItems(agents),
    },
  ];
}

function projectItems(
  projectRoot: string,
  agentId: string,
): CopySkillMenuSection["items"] {
  if (agentId.startsWith("useragent-")) {
    return [];
  }
  const rels = SKILL_BUCKET_REL[agentId];
  if (!rels) return [];
  const agentName = AGENT_UI_NAME[agentId] ?? agentId;
  return rels.map((rel, bucketIndex) => ({
    id: `p:${projectRoot}:${agentId}:${bucketIndex}`,
    label: `${agentName} · ${rel}`,
    payload: {
      destKind: "project" as const,
      agentId,
      bucketIndex,
      projectRoot,
    },
  }));
}

/** Destinations for「复制 skill」：含 AIControls「我的」技能库 + Agent 全局/项目 skills 根。 */
export function buildCopySkillMenuSections(params: {
  dataSet: "skills" | "project" | "aggregate";
  ecosystem?: string;
  projectRoot?: string;
  projectPaths: readonly string[];
  /** Agent 页侧栏已扫过的项目路径（与全局同一生态合并展示时的项目列表） */
  agentProjectScanPaths: readonly string[];
  /** 用户添加的 `useragent-*` id，用于「复制到 · 用户全局」额外目标 */
  userCustomAgentIds?: readonly string[];
  /** 首页导入对话框等处设为 `"导入"`，以便分段标题写「导入到」 */
  copyVerb?: "复制" | "导入";
  /** 普通复制默认不进入「我的」；仅明确导入/收藏到我的技能库时开启。 */
  includeMyLibrary?: boolean;
}): CopySkillMenuSection[] {
  const {
    dataSet,
    ecosystem,
    projectRoot,
    projectPaths,
    agentProjectScanPaths,
    userCustomAgentIds = [],
    copyVerb = "复制",
    includeMyLibrary = false,
  } = params;
  const sections: CopySkillMenuSection[] = [];
  const maybePrependMine = (list: CopySkillMenuSection[]) =>
    includeMyLibrary ? [mineLibrarySection(copyVerb), ...list] : list;

  if (dataSet === "skills" && ecosystem) {
    sections.push({
      key: "global-one",
      title: "复制到 · 用户全局",
      items: globalItems(ecosystem),
    });
    for (const p of agentProjectScanPaths) {
      const pi = projectItems(p, ecosystem);
      if (pi.length === 0) continue;
      sections.push({
        key: `proj:${p}`,
        title: `复制到 · 项目「${folderBasename(p)}」`,
        items: pi,
      });
    }
    return maybePrependMine(sections);
  }

  const globalAllItems = [
    ...AGENT_ORDER.flatMap((id) => globalItems(id)),
    ...userCustomAgentIds.flatMap((id) => globalItems(id)),
  ];

  if (dataSet === "project") {
    sections.push({
      key: "global-all",
      title: "复制到 · 用户全局",
      items: globalAllItems,
    });
    const merged = mergeProjectRootsForCopy(projectPaths, projectRoot);
    const cur = projectRoot?.trim()
      ? normalizeProjectPath(projectRoot)
      : "";
    for (const pt of merged) {
      const isCurrent = cur.length > 0 && pathsReferToSameDir(cur, pt);
      sections.push({
        key: `proj:${pt}`,
        title: isCurrent
          ? `复制到 · 当前项目「${folderBasename(pt)}」`
          : `复制到 · 项目「${folderBasename(pt)}」`,
        items: AGENT_ORDER.flatMap((id) => projectItems(pt, id)),
      });
    }
    return maybePrependMine(sections);
  }

  if (dataSet === "aggregate") {
    sections.push({
      key: "global-all",
      title: "复制到 · 用户全局",
      items: globalAllItems,
    });
    for (const p of projectPaths) {
      const pt = p.trim();
      if (!pt) continue;
      sections.push({
        key: `proj:${pt}`,
        title: `复制到 · 项目「${folderBasename(pt)}」`,
        items: AGENT_ORDER.flatMap((id) => projectItems(pt, id)),
      });
    }
    return maybePrependMine(sections);
  }

  return maybePrependMine(sections);
}
