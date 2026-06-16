import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface SkillMarkdownProps {
  content: string;
  className?: string;
}

/** Strip YAML frontmatter (--- ... ---) from markdown content */
function stripMarkdownFrontmatter(content: string): string {
  const trimmed = content.trimStart();
  if (!trimmed.startsWith("---")) return content;

  const afterFirst = trimmed.slice(3);
  const endIdx = afterFirst.indexOf("\n---");
  if (endIdx === -1) return content;

  return afterFirst.slice(endIdx + 4).trimStart();
}

/** Simple card-style wrapper for inline code snippets */
const codeStyle: React.CSSProperties = {
  backgroundColor: "var(--nav-item-hover)",
  borderRadius: 4,
  padding: "1px 6px",
  fontFamily: "var(--font-mono)",
  fontSize: "0.88em",
  color: "var(--accent)",
};

const preStyle: React.CSSProperties = {
  marginBottom: 16,
  overflowX: "auto",
  borderRadius: 12,
  border: "1px solid var(--border)",
  backgroundColor: "var(--surface)",
  padding: "14px 16px",
};

const blockquoteStyle: React.CSSProperties = {
  marginBottom: 16,
  borderLeft: "3px solid var(--accent)",
  backgroundColor: "var(--surface)",
  padding: "10px 16px",
  color: "var(--muted)",
  fontStyle: "italic",
  borderRadius: "0 8px 8px 0",
};

const tableWrapStyle: React.CSSProperties = {
  marginBottom: 16,
  overflowX: "auto",
  borderRadius: 12,
  border: "1px solid var(--border)",
};

const tableStyle: React.CSSProperties = {
  minWidth: "100%",
  borderCollapse: "collapse",
  fontSize: 13,
};

const thStyle: React.CSSProperties = {
  borderBottom: "1px solid var(--border)",
  padding: "8px 12px",
  fontWeight: 600,
  textAlign: "left",
  backgroundColor: "var(--nav-item-hover)",
};

const tdStyle: React.CSSProperties = {
  borderBottom: "1px solid var(--border)",
  padding: "8px 12px",
  color: "var(--muted)",
};

export function SkillMarkdown({ content, className }: SkillMarkdownProps) {
  const markdown = stripMarkdownFrontmatter(content);

  return (
    <article
      className={className}
      style={{
        margin: "0 auto",
        maxWidth: 1240,
        fontSize: 13,
        lineHeight: 1.7,
        color: "var(--muted)",
      }}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children, ...props }) => (
            <h1
              style={{
                marginBottom: 16,
                fontSize: 28,
                fontWeight: 600,
                lineHeight: 1.2,
                color: "var(--text)",
              }}
              {...props}
            >
              {children}
            </h1>
          ),
          h2: ({ children, ...props }) => (
            <h2
              style={{
                marginBottom: 12,
                marginTop: 32,
                fontSize: 20,
                fontWeight: 600,
                lineHeight: 1.3,
                color: "var(--text)",
              }}
              {...props}
            >
              {children}
            </h2>
          ),
          h3: ({ children, ...props }) => (
            <h3
              style={{
                marginBottom: 8,
                marginTop: 24,
                fontSize: 16,
                fontWeight: 600,
                lineHeight: 1.3,
                color: "var(--text)",
              }}
              {...props}
            >
              {children}
            </h3>
          ),
          p: ({ children, ...props }) => (
            <p
              style={{
                marginBottom: 16,
                fontSize: 13,
                lineHeight: 1.7,
                color: "var(--muted)",
              }}
              {...props}
            >
              {children}
            </p>
          ),
          a: ({ href, children, ...props }) => {
            const isDangerous = /^(javascript|vbscript|data):/i.test(
              href?.trim() ?? "",
            );
            const safeHref = isDangerous ? undefined : href;
            return (
              <a
                href={safeHref}
                target="_blank"
                rel="noreferrer"
                style={{
                  color: "var(--accent)",
                  textDecoration: "underline",
                  textUnderlineOffset: 4,
                }}
                {...props}
              >
                {children}
              </a>
            );
          },
          ul: ({ children, ...props }) => (
            <ul
              style={{
                marginBottom: 16,
                listStyle: "disc",
                paddingLeft: 20,
                color: "var(--muted)",
              }}
              {...props}
            >
              {children}
            </ul>
          ),
          ol: ({ children, ...props }) => (
            <ol
              style={{
                marginBottom: 16,
                listStyle: "decimal",
                paddingLeft: 20,
                color: "var(--muted)",
              }}
              {...props}
            >
              {children}
            </ol>
          ),
          li: ({ children, ...props }) => (
            <li style={{ paddingLeft: 4 }} {...props}>
              {children}
            </li>
          ),
          blockquote: ({ children, ...props }) => (
            <blockquote style={blockquoteStyle} {...props}>
              {children}
            </blockquote>
          ),
          hr: ({ ...props }) => (
            <hr
              style={{
                margin: "24px 0",
                border: "none",
                borderTop: "1px solid var(--border)",
              }}
              {...props}
            />
          ),
          code: ({ className, children, ...props }) => {
            const isBlock = String(className || "").includes("language-");
            if (isBlock) {
              return (
                <code
                  style={{
                    display: "block",
                    fontSize: 13,
                    lineHeight: 1.6,
                    color: "var(--muted)",
                  }}
                  {...props}
                >
                  {children}
                </code>
              );
            }
            return (
              <code style={codeStyle} {...props}>
                {children}
              </code>
            );
          },
          pre: ({ children, ...props }) => (
            <pre style={preStyle} {...props}>
              {children}
            </pre>
          ),
          table: ({ children, ...props }) => (
            <div style={tableWrapStyle}>
              <table style={tableStyle} {...props}>
                {children}
              </table>
            </div>
          ),
          thead: ({ children, ...props }) => (
            <thead {...props}>{children}</thead>
          ),
          th: ({ children, ...props }) => (
            <th style={thStyle} {...props}>
              {children}
            </th>
          ),
          td: ({ children, ...props }) => (
            <td style={tdStyle} {...props}>
              {children}
            </td>
          ),
        }}
      >
        {markdown}
      </ReactMarkdown>
    </article>
  );
}
