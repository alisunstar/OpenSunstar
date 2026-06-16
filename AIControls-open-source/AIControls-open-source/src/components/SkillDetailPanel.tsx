import { useEffect, useRef, useState } from "react";
import { getSkillDocument } from "../api/agents";
import { DetailSheet } from "./DetailSheet";
import { SkillMarkdown } from "./SkillMarkdown";

/** 轻量接口，兼容 BrowseRow 和 AssetEntry 两种数据源 */
export interface DetailEntry {
  id: string;
  kind: string;
  title: string;
  description?: string;
  path?: string;
  /** 技能目录内除主 SKILL.md 外的文件（扫描端填充） */
  skillExtraFiles?: string[];
}

interface SkillDetailPanelProps {
  entry: DetailEntry | null;
  onClose: () => void;
}

type ContentState =
  | { status: "loading" }
  | { status: "loaded"; filename: string; content: string; fmDescription?: string }
  | { status: "error"; message: string }
  | { status: "nosupport" };

/** ATX H1：`# 标题`，排除 `##`。 */
function isAtxH1Line(line: string): boolean {
  const t = line.trimStart();
  return t.startsWith("#") && !t.startsWith("##");
}

function yamlBlockScalarStarts(rest: string): boolean {
  const t = rest.trim();
  return t.startsWith("|") || t.startsWith(">");
}

/** Short `key:` / `key: token` siblings in YAML frontmatter（避免误判 `Note: long prose`）。 */
function lineLooksLikeYamlMapKey(line: string): boolean {
  const t = line.trimStart();
  if (!t || isAtxH1Line(line)) return false;
  const colon = t.indexOf(":");
  if (colon <= 0) return false;
  const key = t.slice(0, colon);
  if (!/^[a-zA-Z_][a-zA-Z0-9_-]*$/.test(key)) return false;
  const after = t.slice(colon + 1).trim();
  if (!after) return true;
  if (after.startsWith('"') || after.startsWith("'")) return true;
  return after.split(/\s+/).length === 1;
}

function collectDescriptionBlockScalar(
  lines: string[],
  start: number,
  stopOnYamlMapKey: boolean
): string | undefined {
  const buf: string[] = [];
  for (let i = start; i < lines.length; i++) {
    const line = lines[i];
    if (isAtxH1Line(line)) break;
    if (stopOnYamlMapKey && lineLooksLikeYamlMapKey(line)) break;
    buf.push(line);
  }
  while (buf.length && buf[buf.length - 1].trim() === "") buf.pop();
  const folded = buf
    .map((l) => l.trim())
    .filter((l) => l.length > 0)
    .join(" ");
  return folded || undefined;
}

function unquoteYamlScalar(s: string): string {
  const t = s.trim();
  if (t.length >= 2 && t.startsWith('"') && t.endsWith('"')) {
    return t.slice(1, -1).replace(/\\"/g, '"');
  }
  if (t.length >= 2 && t.startsWith("'") && t.endsWith("'")) {
    return t.slice(1, -1);
  }
  return t;
}

/** `description:` 行内标量，或块标量（`>-`、`|` 等），与 scan.rs 一致。 */
function extractDescriptionFromYamlLike(
  text: string,
  stopOnYamlMapKey: boolean
): string | undefined {
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trimStart();
    if (!trimmed.startsWith("description:")) continue;
    const rest = trimmed.slice("description:".length).trimStart();
    if (yamlBlockScalarStarts(rest)) {
      return collectDescriptionBlockScalar(lines, i + 1, stopOnYamlMapKey);
    }
    const val = rest.trim();
    if (val) return unquoteYamlScalar(val);
  }
  return undefined;
}

/** `* * *` + `## name:` + `description:` 等，出现在首个一级标题之前（与 scan.rs 一致）。 */
function extractDescriptionFromPseudoHeader(text: string): string | undefined {
  const MAX = 80;
  const lines = text.split("\n");
  const header: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (isAtxH1Line(line)) break;
    header.push(line);
    if (header.length >= MAX) break;
  }
  return extractDescriptionFromYamlLike(header.join("\n"), false);
}

/** 从 `---` YAML 或非标准头里提取 description，并返回去掉标准 frontmatter 的 body */
function parseFrontmatter(content: string): { description?: string; body: string } {
  const trimmed = content.trimStart();
  let body = content;
  let description: string | undefined;

  if (trimmed.startsWith("---")) {
    const afterFirst = trimmed.slice(3);
    const endIdx = afterFirst.indexOf("\n---");
    if (endIdx !== -1) {
      const frontmatter = afterFirst.slice(0, endIdx);
      body = afterFirst.slice(endIdx + 4).trimStart();
      description = extractDescriptionFromYamlLike(frontmatter, true);
    }
  }

  if (!description) {
    const pseudo = extractDescriptionFromPseudoHeader(trimmed);
    if (pseudo) description = pseudo;
  }

  return { description, body };
}

export function SkillDetailPanel({ entry, onClose }: SkillDetailPanelProps) {
  if (!entry) return null;

  return <SkillDetailPanelContent key={entry.id} entry={entry} onClose={onClose} />;
}

const KIND_TAGS: Record<string, { label: string; color: string }> = {
  skill: { label: "Skill", color: "var(--accent)" },
  rule: { label: "Rule", color: "#22c55e" },
  mcp: { label: "MCP", color: "#e879f9" },
};

function SkillDetailPanelContent({
  entry,
  onClose,
}: {
  entry: DetailEntry;
  onClose: () => void;
}) {
  const [docState, setDocState] = useState<ContentState>({ status: "loading" });
  const requestIdRef = useRef(0);

  useEffect(() => {
    requestIdRef.current += 1;
    const reqId = requestIdRef.current;
    setDocState({ status: "loading" });

    // For MCP entries that don't have a local file, show an unsupported message
    if (entry.kind === "mcp") {
      setDocState({ status: "nosupport" });
      return;
    }

    const path = entry.path;
    if (!path) {
      setDocState({ status: "error", message: "无文件路径" });
      return;
    }

    getSkillDocument(path).then((doc) => {
      if (reqId !== requestIdRef.current) return;
      if (doc === null) {
        setDocState({
          status: "error",
          message: "无法读取文档文件",
        });
      } else {
        const { description: fmDescription } = parseFrontmatter(doc.content);
        setDocState({
          status: "loaded",
          filename: doc.filename,
          content: doc.content,
          fmDescription,
        });
      }
    });
  }, [entry.id, entry.path, entry.kind]);

  const kindTag = KIND_TAGS[entry.kind] ?? { label: entry.kind, color: "var(--muted)" };

  const meta = (
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: 8,
        fontSize: "12.5px",
      }}
    >
      {/* 类型标签 */}
      <span
        style={{
          display: "inline-flex",
          alignItems: "center",
          padding: "2px 10px",
          borderRadius: 999,
          fontSize: 11,
          fontWeight: 600,
          border: `1px solid ${kindTag.color}33`,
          color: kindTag.color,
          backgroundColor: `${kindTag.color}11`,
        }}
      >
        {kindTag.label}
      </span>

      {/* 文件路径 */}
      {entry.path && (
        <span
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 4,
            color: "var(--muted)",
            fontSize: 12,
            fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            maxWidth: "100%",
          }}
          title={entry.path}
        >
          📁 {entry.path}
        </span>
      )}
    </div>
  );

  const sheetDescriptionText =
    docState.status === "loaded"
      ? entry.description?.trim() || docState.fmDescription?.trim()
      : entry.description?.trim();

  return (
    <DetailSheet
      open={true}
      title={entry.title}
      description={
        sheetDescriptionText ? (
          <p style={{ margin: 0, lineClamp: 3 } as React.CSSProperties}>{sheetDescriptionText}</p>
        ) : undefined
      }
      meta={meta}
      onClose={onClose}
    >
      {docState.status === "loading" && (
        <div
          style={{
            marginTop: 48,
            textAlign: "center",
            fontSize: 13,
            color: "var(--muted)",
          }}
        >
          加载中…
        </div>
      )}

      {docState.status === "loaded" && (
        <>
          {entry.kind === "skill" &&
            entry.skillExtraFiles &&
            entry.skillExtraFiles.length > 0 && (
              <div
                style={{
                  marginBottom: 20,
                  padding: "12px 14px",
                  borderRadius: 10,
                  border: "1px solid var(--border)",
                  background: "var(--surface-2)",
                }}
              >
                <div
                  style={{
                    fontSize: 11,
                    fontWeight: 600,
                    letterSpacing: "0.04em",
                    textTransform: "uppercase",
                    color: "var(--muted)",
                    marginBottom: 8,
                  }}
                >
                  包内其他文件
                </div>
                <ul
                  style={{
                    margin: 0,
                    paddingLeft: 18,
                    fontSize: 12.5,
                    fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                    color: "var(--foreground)",
                    lineHeight: 1.5,
                  }}
                >
                  {entry.skillExtraFiles.map((name) => (
                    <li key={name}>{name}</li>
                  ))}
                </ul>
              </div>
            )}
          <SkillMarkdown content={docState.content} />
        </>
      )}

      {docState.status === "error" && (
        <div
          style={{
            marginTop: 48,
            textAlign: "center",
            fontSize: 13,
            color: "var(--danger)",
          }}
        >
          {docState.message}
        </div>
      )}

      {docState.status === "nosupport" && (
        <div
          style={{
            marginTop: 48,
            textAlign: "center",
            fontSize: 13,
            color: "var(--muted)",
          }}
        >
          <p>MCP 配置暂无可视化预览。</p>
          <p style={{ fontSize: 12, marginTop: 8, color: "var(--muted)" }}>
            如需编辑 MCP 服务，请直接修改对应的 JSON 配置文件。
          </p>
        </div>
      )}
    </DetailSheet>
  );
}
