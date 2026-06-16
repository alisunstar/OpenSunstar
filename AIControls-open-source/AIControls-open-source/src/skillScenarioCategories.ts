import type { Locale } from "./i18n/types";

/** 场景分类：用于在无结构化 taxonomy 时按文案与路径做启发式筛选 */

export type ScenarioKey =
  | "all"
  | "dev"
  | "office"
  | "creative"
  | "data"
  | "network"
  | "ops"
  | "collab";

export const SCENARIO_ORDER: Exclude<ScenarioKey, "all">[] = [
  "dev",
  "office",
  "creative",
  "data",
  "network",
  "ops",
  "collab",
];

const SCENARIO_LABELS: Record<Locale, Record<ScenarioKey, string>> = {
  zh: {
    all: "全部",
    dev: "开发",
    office: "办公",
    creative: "创作",
    data: "数据",
    network: "网络",
    ops: "运维",
    collab: "协作",
  },
  en: {
    all: "All",
    dev: "Development",
    office: "Office",
    creative: "Creative",
    data: "Data",
    network: "Network",
    ops: "Ops",
    collab: "Collab",
  },
};

/** 按钮 title，便于悬停查看说明 */
const SCENARIO_HINTS: Record<Locale, Record<Exclude<ScenarioKey, "all">, string>> = {
  zh: {
    dev: "适配各类编码、前后端搭建相关 Skills 与代码仓库、API 调用类 MCP，覆盖开发全流程。",
    office:
      "对应办公自动化类 Skills 与日程、文档、邮件相关 MCP，提升日常办公效率。",
    creative:
      "涵盖文案、音视频、设计类 Skills 与图像、音视频处理类 MCP，支撑各类创意产出。",
    data: "适配数据获取、分析、存储类 Skills 与向量库、数据库相关 MCP，实现数据处理与洞察。",
    network:
      "对应浏览器自动化、网页抓取、搜索类 MCP 与网络相关 Skills，实现网络交互与信息获取。",
    ops: "适配容器、云资源、监控类 MCP 与部署、故障排查类 Skills，保障系统稳定运行。",
    collab:
      "涵盖团队沟通、项目管理类 MCP 与会议、任务协同类 Skills，助力团队高效配合。",
  },
  en: {
    dev: "Coding, frontend/backend, repositories, and API/MCP integrations across development workflows.",
    office:
      "Office automation, calendars, docs, and email-oriented skills and MCP services.",
    creative:
      "Copywriting, audio/video, design, image generation, and multimedia production workflows.",
    data: "Data collection, analysis, storage, vector DB, and database-related workflows.",
    network:
      "Browser automation, web scraping, online search, and HTTP data retrieval workflows.",
    ops: "Container, cloud, monitoring, deployment, and troubleshooting workflows.",
    collab:
      "Team communication, project management, meetings, and task collaboration workflows.",
  },
};

export function getScenarioLabel(locale: Locale, key: ScenarioKey): string {
  return SCENARIO_LABELS[locale][key];
}

export function getScenarioHint(locale: Locale, key: Exclude<ScenarioKey, "all">): string {
  return SCENARIO_HINTS[locale][key];
}

const SCENARIO_KEYWORDS: Record<Exclude<ScenarioKey, "all">, readonly string[]> =
  {
    dev: [
      "typescript",
      "javascript",
      "python",
      "rust",
      "golang",
      "kotlin",
      "swift",
      "scala",
      "ruby",
      "php",
      "csharp",
      "c++",
      "frontend",
      "backend",
      "fullstack",
      "full-stack",
      "react",
      "vue",
      "angular",
      "next.js",
      "nextjs",
      "svelte",
      "vite",
      "webpack",
      "rollup",
      "tailwind",
      "graphql",
      "grpc",
      "openapi",
      "swagger",
      "websocket",
      "sdk",
      "vscode",
      "eslint",
      "prettier",
      "jest",
      "pytest",
      "mocha",
      "junit",
      "maven",
      "gradle",
      "npm",
      "yarn",
      "pnpm",
      "cargo.",
      "composer.json",
      "gemfile",
      "refactor",
      "linter",
      "compiler",
      "codegen",
      "monorepo",
      "microservice",
      "rest api",
      "api client",
      "react-native",
      "expo",
      "android",
      "ios",
      "xcode",
      "spring boot",
      "django",
      "flask",
      "fastapi",
      "express",
      "nestjs",
      "laravel",
      "rails",
      "prisma",
      "drizzle",
      "sqlalchemy",
      "mongoose",
      "tauri",
      "electron",
      "skill creator",
      "create-hook",
      "cursor sdk",
      "repository",
      "codebase",
      "编程",
      "开发",
      "编码",
      "前后端",
      "前端",
      "后端",
      "代码仓库",
      "调用接口",
      "接口调用",
      "构建工具",
      "编译",
      "调试",
      "脚手架",
    ],
    office: [
      "excel",
      "spreadsheet",
      "outlook",
      "word doc",
      "docx",
      "office 365",
      "microsoft office",
      "gmail",
      "imap",
      "smtp",
      "mail merge",
      "calendar sync",
      "pdf extract",
      "notion export",
      "zapier",
      "make.com",
      "integromat",
      "办公",
      "日程",
      "邮件",
      "文档",
      "表格",
      "自动化办公",
      "办公软件",
    ],
    creative: [
      "image gen",
      "text-to-image",
      "stable diffusion",
      "midjourney",
      "dall-e",
      "dashscope",
      "replicate",
      "video",
      "remotion",
      "ffmpeg",
      "audio",
      "podcast",
      "subtitle",
      "transcription",
      "whisper",
      "figma",
      "mastergo",
      "canvas",
      "design system",
      "copywriting",
      "prompt engineering",
      "文案",
      "创作",
      "音视频",
      "设计",
      "图像",
      "绘图",
      "视频生成",
      "配音",
    ],
    data: [
      "database",
      "postgres",
      "postgresql",
      "mysql",
      "sqlite",
      "mongodb",
      "redis",
      "vector db",
      "vector database",
      "embedding",
      "embeddings",
      "chroma",
      "pinecone",
      "milvus",
      "weaviate",
      "qdrant",
      "rag ",
      "rag.",
      "retrieval",
      "analytics",
      "snowflake",
      "bigquery",
      "databricks",
      "pandas",
      "polars",
      "duckdb",
      "warehouse",
      "etl",
      "dbt",
      "数据",
      "数据库",
      "向量库",
      "向量",
      "分析",
      "存储",
      "洞察",
      "数据仓库",
    ],
    network: [
      "puppeteer",
      "playwright",
      "selenium",
      "headless browser",
      "browser automation",
      "web scraping",
      "scraping",
      "scraper",
      "crawler",
      "crawling",
      "cheerio",
      "jsdom",
      "http client",
      "urllib",
      "axios",
      "fetch api",
      "libcurl",
      "wget",
      "github-search",
      "web search",
      "serp",
      "浏览器",
      "网页抓取",
      "爬虫",
      "抓取网页",
      "搜索api",
      "站点抓取",
    ],
    ops: [
      "docker",
      "kubernetes",
      "k8s",
      "helm",
      "terraform",
      "ansible",
      "pulumi",
      "aws",
      "gcp",
      "azure",
      "cloudflare",
      "nginx",
      "prometheus",
      "grafana",
      "datadog",
      "new relic",
      "sentry deploy",
      "ci/cd",
      "github actions",
      "gitlab ci",
      "jenkins",
      "argocd",
      "kubectl",
      "container",
      "deployment",
      "运维",
      "容器",
      "监控",
      "部署",
      "云资源",
      "故障排查",
      "告警",
    ],
    collab: [
      "slack",
      "discord",
      "microsoft teams",
      "teams meeting",
      "linear.app",
      "linear issue",
      "jira",
      "asana",
      "trello",
      "monday.com",
      "clickup",
      "notion api",
      "zoom meeting",
      "meet.google",
      "calendar invite",
      "协作",
      "团队沟通",
      "项目管理",
      "会议",
      "任务协同",
      "工单",
    ],
  };

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function keywordMatches(haystack: string, kw: string): boolean {
  const t = kw.trim();
  if (!t) return false;
  const lower = haystack.toLowerCase();
  const kl = t.toLowerCase();
  if (/[^\x00-\x7F]/.test(t)) {
    return lower.includes(kl);
  }
  const needsSubstring =
    /[\s./\\:+@-]/.test(t) || kl.length >= 16;
  if (!needsSubstring && /^[a-z0-9][a-z0-9_-]*$/i.test(t)) {
    return new RegExp(`\\b${escapeRegExp(kl)}\\b`, "i").test(haystack);
  }
  return lower.includes(kl);
}

export function rowMatchesScenario(
  haystack: string,
  key: Exclude<ScenarioKey, "all">,
): boolean {
  return SCENARIO_KEYWORDS[key].some((kw) => keywordMatches(haystack, kw));
}

export function browseRowHaystack(row: {
  title: string;
  desc: string;
  sourcePath?: string;
}): string {
  return `${row.title}\n${row.desc}\n${row.sourcePath ?? ""}`;
}

export function isAiScenarioSlug(
  s: string | null | undefined,
): s is Exclude<ScenarioKey, "all"> {
  return !!s && s.trim() !== "";
}

/** 优先使用 AI 写入的 `scenario`，否则沿用关键词启发式。 */
export function rowMatchesScenarioChip(
  row: {
    title: string;
    desc: string;
    sourcePath?: string;
    scenario?: string | null;
  },
  scenario: ScenarioKey,
): boolean {
  if (scenario === "all") return true;
  if (isAiScenarioSlug(row.scenario)) {
    return row.scenario === scenario;
  }
  return rowMatchesScenario(browseRowHaystack(row), scenario);
}

/** 使用自定义分类 slug 进行匹配（重新分类后使用）。 */
export function rowMatchesCustomScenario(
  row: {
    scenario?: string | null;
  },
  slug: string,
): boolean {
  return row.scenario === slug;
}
