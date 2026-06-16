import type { AgentInventory, AssetEntry } from "./api/agents";

export function inventoryAssetCount(inv: AgentInventory): number {
  return inv.skills.length + inv.mcp.length + inv.rules.length;
}

/** Normalize separators for stable substring checks */
function normPath(p: string): string {
  return p.replace(/\\/g, "/").toLowerCase();
}

/** Infer which agent conventions a project-path asset belongs to (paths from desktop scanner). */
export function inferAgentIdFromAssetPath(path: string): string | null {
  const n = normPath(path);

  if (n.includes("/.claude/")) return "claude";
  if (/(?:^|[\\/])claude\.md$/i.test(path)) return "claude";

  if (n.includes("/.codex/")) return "codex";
  if (/(?:^|[\\/])agents\.md$/i.test(path)) return "codex";

  if (n.includes("/.hermes/")) return "hermes";
  if (/(?:^|[\\/])hermes\.md$/i.test(path)) return "hermes";

  if (n.includes("/.openclaw/")) return "openclaw";
  if (/(?:^|[\\/])openclaw\.md$/i.test(path)) return "openclaw";

  if (n.includes("/.trae/")) return "trae";
  if (/(?:^|[\\/])trae\.config\.jsonc?$/i.test(path)) return "trae";

  if (n.includes("/.qoderwork/")) return "qoder";
  if (n.includes("/.qoder/")) return "qoder";

  if (n.includes("/.kiro/")) return "kiro";

  if (n.includes("/.opencode/")) return "opencode";
  if (n.includes("/.config/opencode/")) return "opencode";

  if (n.includes("/.cursor/")) return "cursor";
  if (/(?:^|[\\/])\.cursorrules$/i.test(path)) return "cursor";
  if (n.includes("/.vscode/")) return "cursor";

  return null;
}

export function assetPathMatchesAgent(agentId: string, path: string): boolean {
  return inferAgentIdFromAssetPath(path) === agentId;
}

export function filterInventoryForAgent(
  agentId: string,
  inv: AgentInventory,
): AgentInventory {
  const ok = (e: AssetEntry) => assetPathMatchesAgent(agentId, e.path);
  return {
    skills: inv.skills.filter(ok),
    mcp: inv.mcp.filter(ok),
    rules: inv.rules.filter(ok),
  };
}

type Bucket = { skills: AssetEntry[]; mcp: AssetEntry[]; rules: AssetEntry[] };

/** Split merged project scan into per-agent inventories; sort by total count descending. */
export function bucketInventoryByAgent(
  inv: AgentInventory,
): { agentId: string; inv: AgentInventory }[] {
  const map = new Map<string, Bucket>();

  const push = (e: AssetEntry, key: keyof Bucket) => {
    const id = inferAgentIdFromAssetPath(e.path) ?? "__other__";
    let b = map.get(id);
    if (!b) {
      b = { skills: [], mcp: [], rules: [] };
      map.set(id, b);
    }
    b[key].push(e);
  };

  for (const e of inv.skills) push(e, "skills");
  for (const e of inv.mcp) push(e, "mcp");
  for (const e of inv.rules) push(e, "rules");

  const out = [...map.entries()].map(([agentId, b]) => ({
    agentId,
    inv: {
      skills: b.skills,
      mcp: b.mcp,
      rules: b.rules,
    },
  }));

  out.sort(
    (a, b) => inventoryAssetCount(b.inv) - inventoryAssetCount(a.inv),
  );
  return out;
}
