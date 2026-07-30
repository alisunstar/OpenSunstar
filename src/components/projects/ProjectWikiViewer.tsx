import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { BookOpen, FileText, Loader2, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import type { WikiDocument, WikiPageContent } from "@/types/projectWiki";

interface ProjectWikiViewerProps {
  open: boolean;
  title: string;
  document: WikiDocument | null;
  loading: boolean;
  error: string | null;
  onOpenChange: (open: boolean) => void;
}

/** 正式 Wiki 与隔离候选共用的只读阅读器。 */
export function ProjectWikiViewer({
  open,
  title,
  document,
  loading,
  error,
  onOpenChange,
}: ProjectWikiViewerProps) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const contentRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    setSelectedPath(document?.pages[0]?.path ?? null);
  }, [document]);

  const selectedPage = useMemo(
    () =>
      document?.pages.find((page) => page.path === selectedPath) ??
      document?.pages[0] ??
      null,
    [document, selectedPath],
  );

  const pagePaths = useMemo(
    () => document?.pages.map((page) => page.path) ?? [],
    [document],
  );

  const navigateToPage = useCallback((path: string, anchor?: string) => {
    setSelectedPath(path);
    window.requestAnimationFrame(() => {
      const content = contentRef.current;
      if (!content) return;
      if (anchor) {
        const decodedAnchor = decodeWikiAnchor(anchor);
        const heading = Array.from(
          content.querySelectorAll<HTMLElement>("[id]"),
        ).find((element) => element.id === decodedAnchor);
        heading?.scrollIntoView({ block: "start" });
        return;
      }
      if (typeof content.scrollTo === "function") {
        content.scrollTo({ top: 0 });
      } else {
        content.scrollTop = 0;
      }
    });
  }, []);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="h-[86vh] w-[94vw] max-w-6xl overflow-hidden p-0">
        <DialogHeader className="flex-row items-start justify-between space-y-0 px-5 py-4">
          <div className="min-w-0">
            <DialogTitle className="flex items-center gap-2 text-base">
              <BookOpen className="h-4 w-4 text-primary" />
              {title}
            </DialogTitle>
            <DialogDescription className="mt-1 text-xs">
              {document?.candidateId
                ? `隔离候选 ${document.candidateId} · 导入前预览`
                : "正式 Wiki · 验收前请逐页核对内容与源码依据"}
            </DialogDescription>
          </div>
          <Button
            size="icon"
            variant="ghost"
            className="h-7 w-7 shrink-0"
            onClick={() => onOpenChange(false)}
            aria-label="关闭 Wiki 阅读器"
          >
            <X className="h-4 w-4" />
          </Button>
        </DialogHeader>

        <div className="grid min-h-0 flex-1 grid-cols-[230px_minmax(0,1fr)]">
          <aside className="min-h-0 overflow-y-auto border-r border-border/60 bg-muted/10 p-3">
            <p className="mb-2 px-2 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              页面目录 · {document?.pages.length ?? 0}
            </p>
            <div className="space-y-1">
              {document?.pages.map((page) => (
                <button
                  key={page.path}
                  type="button"
                  className={cn(
                    "w-full rounded-md px-2.5 py-2 text-left transition-colors",
                    selectedPage?.path === page.path
                      ? "bg-primary/10 text-primary"
                      : "text-muted-foreground hover:bg-muted/50 hover:text-foreground",
                  )}
                  onClick={() => navigateToPage(page.path)}
                >
                  <span className="block truncate text-xs font-medium">
                    {page.title}
                  </span>
                  <span className="mt-0.5 block truncate font-mono text-[9px] opacity-70">
                    {page.path}
                  </span>
                </button>
              ))}
            </div>
          </aside>

          <main
            ref={contentRef}
            className="min-h-0 overflow-y-auto bg-background px-7 py-6"
          >
            {loading && (
              <div className="flex h-full items-center justify-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                正在读取 Wiki 正文…
              </div>
            )}
            {!loading && error && (
              <div className="rounded-lg border border-red-500/20 bg-red-500/5 p-4 text-sm text-red-600 dark:text-red-400">
                {error}
              </div>
            )}
            {!loading && !error && selectedPage && (
              <WikiPageView
                page={selectedPage}
                pagePaths={pagePaths}
                onNavigate={navigateToPage}
              />
            )}
            {!loading && !error && !selectedPage && (
              <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                当前文档没有可阅读页面
              </div>
            )}
          </main>
        </div>
      </DialogContent>
    </Dialog>
  );
}

interface WikiPageViewProps {
  page: WikiPageContent;
  pagePaths: string[];
  onNavigate: (path: string, anchor?: string) => void;
}

function WikiPageView({ page, pagePaths, onNavigate }: WikiPageViewProps) {
  return (
    <article className="mx-auto max-w-4xl">
      <div className="mb-5 border-b border-border/60 pb-4">
        <div className="flex items-start gap-3">
          <FileText className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
          <div className="min-w-0">
            <h2 className="text-xl font-semibold tracking-tight">
              {page.title}
            </h2>
            <p className="mt-1 font-mono text-[10px] text-muted-foreground">
              {page.path} · {page.pageType || "page"} · {page.status || "draft"}
            </p>
          </div>
        </div>
        {page.sourceFiles.length > 0 && (
          <div className="mt-3 flex flex-wrap gap-1.5">
            {page.sourceFiles.map((source) => (
              <code
                key={source}
                className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground"
              >
                {source}
              </code>
            ))}
          </div>
        )}
      </div>
      <WikiMarkdown
        content={page.content}
        currentPath={page.path}
        pagePaths={pagePaths}
        onNavigate={onNavigate}
      />
    </article>
  );
}

/**
 * 只读、安全的轻量 Markdown 展示。这里不执行 HTML，也不加载远程资源；
 * Wiki 的工程标题、列表、代码块与正文可以直接阅读。
 */
interface WikiMarkdownProps {
  content: string;
  currentPath: string;
  pagePaths: string[];
  onNavigate: (path: string, anchor?: string) => void;
}

function WikiMarkdown({
  content,
  currentPath,
  pagePaths,
  onNavigate,
}: WikiMarkdownProps) {
  const nodes: ReactNode[] = [];
  const inlineOptions: InlineRenderOptions = {
    currentPath,
    pagePaths,
    onNavigate,
  };
  const lines = content.replace(/\r\n/g, "\n").split("\n");
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (line.trim().startsWith("```")) {
      const language = line.trim().slice(3).trim();
      const code: string[] = [];
      index += 1;
      while (index < lines.length && !lines[index].trim().startsWith("```")) {
        code.push(lines[index]);
        index += 1;
      }
      nodes.push(
        <pre
          key={`code-${index}`}
          className="my-4 overflow-x-auto rounded-lg border border-border/60 bg-muted/30 p-4 text-xs leading-5"
        >
          {language && (
            <span className="mb-2 block text-[9px] uppercase tracking-wider text-muted-foreground">
              {language}
            </span>
          )}
          <code>{code.join("\n")}</code>
        </pre>,
      );
      index += 1;
      continue;
    }
    const heading = /^(#{1,4})\s+(.+)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const className = cn(
        "font-semibold tracking-tight text-foreground",
        level === 1 && "mb-3 mt-1 text-2xl",
        level === 2 && "mb-2 mt-7 text-lg",
        level === 3 && "mb-2 mt-5 text-base",
        level === 4 && "mb-1.5 mt-4 text-sm",
      );
      nodes.push(
        <div
          key={`heading-${index}`}
          id={wikiHeadingId(heading[2])}
          className={cn(className, "scroll-mt-4")}
        >
          {renderInline(heading[2], inlineOptions)}
        </div>,
      );
    } else if (/^[-*]\s+/.test(line)) {
      nodes.push(
        <div
          key={`bullet-${index}`}
          className="my-1 flex gap-2 text-sm leading-6"
        >
          <span className="text-primary">•</span>
          <span>
            {renderInline(line.replace(/^[-*]\s+/, ""), inlineOptions)}
          </span>
        </div>,
      );
    } else if (/^\d+\.\s+/.test(line)) {
      const match = /^(\d+)\.\s+(.+)$/.exec(line);
      nodes.push(
        <div
          key={`ordered-${index}`}
          className="my-1 flex gap-2 text-sm leading-6"
        >
          <span className="min-w-5 text-right text-primary">{match?.[1]}.</span>
          <span>{renderInline(match?.[2] ?? line, inlineOptions)}</span>
        </div>,
      );
    } else if (line.trim().startsWith(">")) {
      nodes.push(
        <blockquote
          key={`quote-${index}`}
          className="my-3 border-l-2 border-primary/40 pl-4 text-sm italic leading-6 text-muted-foreground"
        >
          {renderInline(line.trim().replace(/^>\s?/, ""), inlineOptions)}
        </blockquote>,
      );
    } else if (line.trim()) {
      nodes.push(
        <p
          key={`paragraph-${index}`}
          className="my-2 text-sm leading-7 text-foreground/90"
        >
          {renderInline(line, inlineOptions)}
        </p>,
      );
    } else {
      nodes.push(<div key={`space-${index}`} className="h-1" />);
    }
    index += 1;
  }
  return <div>{nodes}</div>;
}

interface InlineRenderOptions {
  currentPath: string;
  pagePaths: string[];
  onNavigate: (path: string, anchor?: string) => void;
}

function renderInline(
  value: string,
  options: InlineRenderOptions,
): ReactNode[] {
  const nodes: ReactNode[] = [];
  const tokenPattern = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = tokenPattern.exec(value)) !== null) {
    if (match.index > cursor) {
      nodes.push(value.slice(cursor, match.index));
    }
    const token = match[0];
    const key = `${match.index}-${token}`;
    if (token.startsWith("`") && token.endsWith("`")) {
      nodes.push(
        <code key={key} className="rounded bg-muted px-1 py-0.5 text-[0.9em]">
          {token.slice(1, -1)}
        </code>,
      );
    } else if (token.startsWith("**") && token.endsWith("**")) {
      nodes.push(<strong key={key}>{token.slice(2, -2)}</strong>);
    } else {
      const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(token);
      const label = link?.[1] ?? token;
      const href = link?.[2].trim() ?? "";
      const target = resolveWikiLinkTarget(
        options.currentPath,
        href,
        options.pagePaths,
      );
      if (target) {
        nodes.push(
          <a
            key={key}
            href={href}
            className="font-medium text-primary underline decoration-primary/35 underline-offset-4 transition-colors hover:decoration-primary"
            onClick={(event) => {
              event.preventDefault();
              options.onNavigate(target.path, target.anchor);
            }}
          >
            {label}
          </a>,
        );
      } else {
        nodes.push(
          <span
            key={key}
            className="text-muted-foreground underline decoration-dotted underline-offset-4"
            title={`未找到 Wiki 页面：${href}`}
          >
            {label}
          </span>,
        );
      }
    }
    cursor = match.index + token.length;
  }

  if (cursor < value.length) {
    nodes.push(value.slice(cursor));
  }
  return nodes;
}

export interface WikiLinkTarget {
  path: string;
  anchor?: string;
}

/**
 * 将正文里的相对链接限制并解析到当前 Wiki 文档。
 * 支持文件、目录（进入目录首个页面）、../、根路径以及 #anchor。
 */
export function resolveWikiLinkTarget(
  currentPath: string,
  rawHref: string,
  pagePaths: string[],
): WikiLinkTarget | null {
  const href = rawHref.replace(/^<|>$/g, "").trim();
  if (!href || /^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(href)) return null;

  const hashIndex = href.indexOf("#");
  const rawPath = (hashIndex >= 0 ? href.slice(0, hashIndex) : href).split(
    "?",
    1,
  )[0];
  const anchor = hashIndex >= 0 ? href.slice(hashIndex + 1) : undefined;
  if (!rawPath) {
    return pagePaths.includes(currentPath)
      ? { path: currentPath, anchor }
      : null;
  }

  let decodedPath: string;
  try {
    decodedPath = decodeURIComponent(rawPath);
  } catch {
    decodedPath = rawPath;
  }
  decodedPath = decodedPath.replace(/\\/g, "/");
  const segments = decodedPath.startsWith("/")
    ? []
    : currentPath.split("/").slice(0, -1);
  for (const segment of decodedPath.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (segments.length === 0) return null;
      segments.pop();
      continue;
    }
    segments.push(segment);
  }

  let normalized = segments.join("/");
  if (normalized.startsWith("wiki/")) normalized = normalized.slice(5);
  const directCandidates = [
    normalized,
    `${normalized}.md`,
    `${normalized.replace(/\/$/, "")}/index.md`,
  ];
  const direct = directCandidates.find((candidate) =>
    pagePaths.includes(candidate),
  );
  if (direct) return { path: direct, anchor };

  const directory = `${normalized.replace(/\/$/, "")}/`;
  const firstPageInDirectory = pagePaths.find((path) =>
    path.startsWith(directory),
  );
  return firstPageInDirectory ? { path: firstPageInDirectory, anchor } : null;
}

function decodeWikiAnchor(anchor: string): string {
  try {
    return decodeURIComponent(anchor);
  } catch {
    return anchor;
  }
}

function wikiHeadingId(value: string): string {
  return value
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/[`*_~]/g, "")
    .trim()
    .toLocaleLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^\p{Letter}\p{Number}\-_\u4e00-\u9fff]/gu, "");
}
