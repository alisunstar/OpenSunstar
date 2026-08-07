/**
 * Skill 自动分类规则
 *
 * 基于 name + description 关键词匹配的轻量分类引擎（纯前端、零 API 调用）。
 * 规则按优先级排列，first-match 语义；未命中的 Skill 归入 "其它" 兜底分类。
 *
 * 设计参考：SkillDeck (skilldeck-main by zephyrwang6) 的 CATEGORY_RULES 方案，
 * 针对 OpenSunstar 国际化用户群做了扩展（新增文档/DevOps/AI-Agent 分类）。
 */

/** 分类标识（内部 key，用于匹配和过滤） */
export type SkillCategory =
  | "writing"
  | "coding"
  | "data"
  | "docs"
  | "design"
  | "media"
  | "slides"
  | "product"
  | "collab"
  | "info"
  | "devops"
  | "ai"
  | "other";

/** 分类规则定义：[categoryKey, regex] */
const SKILL_CATEGORY_RULES: [SkillCategory, RegExp][] = [
  ["writing", /写作|文章|改写|润色|文案|口播|翻译|\bwrit|article|blog|copywriting|translat|humaniz/i],
  ["slides", /ppt|slide|幻灯|演示|deck|presentation|keynote/i],
  ["media", /视频|音频|录音|字幕|播客|video|audio|ffmpeg|tts|asr|remotion|podcast|gif/i],
  ["coding", /代码|编程|coding|code\b|debug|lint|\bspec\b|mcp|skill.*creat|extension|plugin|git\b|refactor|review/i],
  ["data", /excel|表格|xlsx|csv|sql|database|analy|chart|plot|visualization|pandas|numpy/i],
  ["docs", /pdf|docx|word\b|document|markdown|report|简历|resume|readme/i],
  ["design", /图片|设计|海报|logo|绘图|image|design|draw|poster|figma|ui\s|ux\s|visual|canvas|art/i],
  ["product", /产品|prd|原型|需求|roadmap|竞品|okr|prototype|user[\s-]*stor|brainstorm|文档/i],
  ["collab", /飞书|lark|slack|notion|协作|collab|calendar|审批|知识库|wiki|team|团队/i],
  ["info", /rss|抓取|爬|采集|搜索|热点|scrap|search|news|trend|browser|fetch|web/i],
  ["devops", /部署|deploy|docker|k8s|kubernetes|monitor|日志|log\b|server|infra|ci\/cd|vercel|netlify/i],
  ["ai", /agent|llm|prompt|rag|embedding|fine.?tun|模型|model\b|ai\s|gpt|claude|gemini|codex/i],
];

/** 分类的显示信息（label + 颜色） */
export interface SkillCategoryInfo {
  key: SkillCategory;
  /** i18n key（供 t() 调用） */
  i18nKey: string;
  /** 兜底显示名（i18n 未配置时使用） */
  fallbackLabel: string;
  /** Pill 样式 */
  pillClass: string;
  /** 激活态 Pill 样式 */
  pillActiveClass: string;
}

/** 分类显示配置表（顺序即 Pill 渲染顺序） */
export const SKILL_CATEGORIES: SkillCategoryInfo[] = [
  {
    key: "coding",
    i18nKey: "skills.category.coding",
    fallbackLabel: "编程/Code",
    pillClass: "bg-green-500/10 text-green-700 dark:text-green-300",
    pillActiveClass: "bg-green-500/20 text-green-800 dark:text-green-200 ring-1 ring-green-500/40",
  },
  {
    key: "writing",
    i18nKey: "skills.category.writing",
    fallbackLabel: "写作/Writing",
    pillClass: "bg-amber-500/10 text-amber-700 dark:text-amber-300",
    pillActiveClass: "bg-amber-500/20 text-amber-800 dark:text-amber-200 ring-1 ring-amber-500/40",
  },
  {
    key: "data",
    i18nKey: "skills.category.data",
    fallbackLabel: "数据/Data",
    pillClass: "bg-blue-500/10 text-blue-700 dark:text-blue-300",
    pillActiveClass: "bg-blue-500/20 text-blue-800 dark:text-blue-200 ring-1 ring-blue-500/40",
  },
  {
    key: "docs",
    i18nKey: "skills.category.docs",
    fallbackLabel: "文档/Docs",
    pillClass: "bg-orange-500/10 text-orange-700 dark:text-orange-300",
    pillActiveClass: "bg-orange-500/20 text-orange-800 dark:text-orange-200 ring-1 ring-orange-500/40",
  },
  {
    key: "design",
    i18nKey: "skills.category.design",
    fallbackLabel: "图片/Design",
    pillClass: "bg-pink-500/10 text-pink-700 dark:text-pink-300",
    pillActiveClass: "bg-pink-500/20 text-pink-800 dark:text-pink-200 ring-1 ring-pink-500/40",
  },
  {
    key: "media",
    i18nKey: "skills.category.media",
    fallbackLabel: "视频/音视频",
    pillClass: "bg-purple-500/10 text-purple-700 dark:text-purple-300",
    pillActiveClass: "bg-purple-500/20 text-purple-800 dark:text-purple-200 ring-1 ring-purple-500/40",
  },
  {
    key: "slides",
    i18nKey: "skills.category.slides",
    fallbackLabel: "PPT/演示",
    pillClass: "bg-red-500/10 text-red-700 dark:text-red-300",
    pillActiveClass: "bg-red-500/20 text-red-800 dark:text-red-200 ring-1 ring-red-500/40",
  },
  {
    key: "product",
    i18nKey: "skills.category.product",
    fallbackLabel: "产品/PM",
    pillClass: "bg-teal-500/10 text-teal-700 dark:text-teal-300",
    pillActiveClass: "bg-teal-500/20 text-teal-800 dark:text-teal-200 ring-1 ring-teal-500/40",
  },
  {
    key: "collab",
    i18nKey: "skills.category.collab",
    fallbackLabel: "协作/Collab",
    pillClass: "bg-cyan-500/10 text-cyan-700 dark:text-cyan-300",
    pillActiveClass: "bg-cyan-500/20 text-cyan-800 dark:text-cyan-200 ring-1 ring-cyan-500/40",
  },
  {
    key: "info",
    i18nKey: "skills.category.info",
    fallbackLabel: "信息/Info",
    pillClass: "bg-yellow-500/10 text-yellow-700 dark:text-yellow-300",
    pillActiveClass: "bg-yellow-500/20 text-yellow-800 dark:text-yellow-200 ring-1 ring-yellow-500/40",
  },
  {
    key: "devops",
    i18nKey: "skills.category.devops",
    fallbackLabel: "DevOps",
    pillClass: "bg-slate-500/10 text-slate-700 dark:text-slate-300",
    pillActiveClass: "bg-slate-500/20 text-slate-800 dark:text-slate-200 ring-1 ring-slate-500/40",
  },
  {
    key: "ai",
    i18nKey: "skills.category.ai",
    fallbackLabel: "AI/Agent",
    pillClass: "bg-violet-500/10 text-violet-700 dark:text-violet-300",
    pillActiveClass: "bg-violet-500/20 text-violet-800 dark:text-violet-200 ring-1 ring-violet-500/40",
  },
  {
    key: "other",
    i18nKey: "skills.category.other",
    fallbackLabel: "其它/Other",
    pillClass: "bg-gray-500/10 text-gray-700 dark:text-gray-300",
    pillActiveClass: "bg-gray-500/20 text-gray-800 dark:text-gray-200 ring-1 ring-gray-500/40",
  },
];

/**
 * 根据 Skill 的 name + description 自动分类。
 * 纯前端运算，不调用任何 API。
 */
export function classifySkill(name: string, description?: string): SkillCategory {
  const text = `${name} ${description ?? ""}`;
  for (const [category, regex] of SKILL_CATEGORY_RULES) {
    if (regex.test(text)) return category;
  }
  return "other";
}

/**
 * 批量分类一组 Skill，返回 Map<categoryId, count>。
 */
export function classifySkills(
  skills: { name: string; description?: string }[]
): Map<SkillCategory, number> {
  const counts = new Map<SkillCategory, number>();
  for (const skill of skills) {
    const cat = classifySkill(skill.name, skill.description);
    counts.set(cat, (counts.get(cat) ?? 0) + 1);
  }
  return counts;
}
