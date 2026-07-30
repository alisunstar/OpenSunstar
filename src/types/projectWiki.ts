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
  sourceBaseline: WikiSourceBaselineStatus;
  lifecycle: WikiLifecycle;
  checkedAt: number;
}

/** 当前项目的源码同步基线；没有 Git 提交时使用内容快照。 */
export interface WikiSourceBaselineStatus {
  hasGitCommit: boolean;
  snapshotSha256: string | null;
  snapshotFileCount: number | null;
  snapshotRecordedAt: number | null;
}

/** 由 OpenSunstar 控制面持久化的 Wiki 生命周期，不由生成器自行决定。 */
export interface WikiLifecycle {
  phase:
    | "uninitialized"
    | "pendingGeneration"
    | "generating"
    | "pendingAcceptance"
    | "syncedToCommit"
    | "syncedToSnapshot"
    | "changesDetected"
    | "pendingSync"
    | "syncing"
    | "updated"
    | "failed";
  baselineCommit: string | null;
  baselineContentSha256: string | null;
  engine: string | null;
  updatedAt: number;
  lastError: string | null;
}

/** 生成器写入隔离目录、尚未进入正式 wiki/ 的候选产物。 */
export interface WikiCandidate {
  id: string;
  engine: string;
  createdAt: number;
  pageCount: number;
  hasIndex: boolean;
  path: string;
  sourceCommit: string | null;
  model: string | null;
  generationSeconds: number | null;
}

/** 控制面安全导入候选后的结果。 */
export interface WikiCandidateImportResult {
  candidate: WikiCandidate;
  backupPath: string;
  filesWritten: number;
  lifecycle: WikiLifecycle;
}

export interface WikiCandidateQuality {
  candidateId: string;
  engine: string;
  sourceCommit: string | null;
  model: string | null;
  pageCount: number;
  qualityLevel: string;
  sourceRefCount: number;
  invalidSourceRefCount: number;
  questionCount: number;
  corePageCoverage: WikiCorePageCoverage;
  generationSeconds: number | null;
}

export interface WikiComparisonReport {
  baseCommit: string | null;
  model: string | null;
  generatedAt: number;
  comparable: boolean;
  blockers: string[];
  results: WikiCandidateQuality[];
}

export interface WikiGeneratorRunResult {
  candidate: WikiCandidate;
  durationSeconds: number;
  summary: string;
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

/** Wiki 阅读器使用的页面正文。frontmatter 已由后端解析并从正文中移除。 */
export interface WikiPageContent {
  path: string;
  title: string;
  pageType: string;
  status: string;
  sourceFiles: string[];
  content: string;
}

/** 正式 Wiki 或指定隔离候选的只读文档。 */
export interface WikiDocument {
  candidateId: string | null;
  pages: WikiPageContent[];
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
