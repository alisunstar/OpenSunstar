/**
 * Project Wiki Baseline 类型定义
 *
 * 与后端 src-tauri/src/services/project_wiki.rs 的数据结构保持一致。
 */

export interface WikiScanResult {
  projectId: string;
  wikiRoot: string;
  exists: boolean;
  baseStatus: "missing" | "scaffolded" | "effective" | "drifted" | "invalid";
  qualityLevel: "N/A" | "L1" | "L2" | "L3";
  pageCount: number;
  corePageCoverage: WikiCorePageCoverage;
  sourceRefCount: number;
  questionCount: number;
  latestMtime: number | null;
  contentSha256: string | null;
  lastLintPassed: boolean | null;
  lastLintAt: number | null;
  checkedAt: number;
}

export interface WikiCorePageCoverage {
  hasIndex: boolean;
  hasOverview: boolean;
  hasSourceMap: boolean;
  hasLog: boolean;
  hasSchema: boolean;
  componentPages: number;
  flowPages: number;
  apiPages: number;
  runbookPages: number;
}

export interface WikiPageMeta {
  path: string;
  title: string;
  pageType: string;
  status: string;
  sourceFiles: string[];
  lastVerified: string | null;
  lastVerifiedCommit: string | null;
  tags: string[];
  mtime: number;
  sizeBytes: number;
}

export interface WikiInventory {
  projectId: string;
  pages: WikiPageMeta[];
  summary: WikiInventorySummary;
  generatedAt: number;
}

export interface WikiInventorySummary {
  totalPages: number;
  byType: Record<string, number>;
  byStatus: Record<string, number>;
  effectiveSourcePages: number;
}

export interface WikiLintIssue {
  ruleId: string;
  file: string;
  line: number | null;
  message: string;
  severity: "error" | "warning";
}

export interface WikiLintResult {
  projectId: string;
  wikiRoot: string;
  checkedAt: number;
  qualityMode: boolean;
  errors: WikiLintIssue[];
  warnings: WikiLintIssue[];
  summary: {
    totalFiles: number;
    errorCount: number;
    warningCount: number;
    passed: boolean;
    qualityLevel: string;
  };
}

export interface WikiInitFilePlan {
  targetPath: string;
  sourceTemplate: string;
  willCreate: boolean;
  alreadyExists: boolean;
}

export interface WikiInitPlan {
  projectId: string;
  files: WikiInitFilePlan[];
  audit: {
    blocked: boolean;
    existingWikiFiles: number;
    warnings: string[];
  };
}

export interface WikiInitResult {
  projectId: string;
  filesCreated: string[];
  filesSkipped: string[];
  profilePath: string;
}

export interface WikiChangedFilesResult {
  coldStart: boolean;
  effectiveSourcePages: number;
  threshold: number;
  changedFiles: string[];
  affectedPages: string[];
  unmappedChangedFiles: string[];
  guidance: string | null;
}
