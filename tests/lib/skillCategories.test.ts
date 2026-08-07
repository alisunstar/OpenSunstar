import { describe, expect, it } from "vitest";
import {
  classifySkill,
  classifySkills,
  SKILL_CATEGORIES,
} from "@/lib/skillCategories";

describe("skillCategories", () => {
  describe("classifySkill", () => {
    it("classifies writing skills by Chinese keywords", () => {
      expect(classifySkill("公众号写作助手", "帮助用户撰写公众号文章")).toBe("writing");
    });

    it("classifies writing skills by English keywords", () => {
      expect(classifySkill("blog-writer", "Write blog articles with SEO optimization")).toBe("writing");
    });

    it("classifies translation as writing", () => {
      expect(classifySkill("translate-pro", "Professional translation tool")).toBe("writing");
    });

    it("classifies coding skills", () => {
      expect(classifySkill("code-reviewer", "Review code for best practices")).toBe("coding");
      expect(classifySkill("git-commit", "Generate git commit messages")).toBe("coding");
    });

    it("classifies data skills", () => {
      expect(classifySkill("data-analysis", "全链路数据分析；支持 CSV/Excel")).toBe("data");
      expect(classifySkill("sql-query", "Generate SQL queries from natural language")).toBe("data");
    });

    it("classifies docs skills", () => {
      expect(classifySkill("pdf", "Use this skill for PDF file operations")).toBe("docs");
      expect(classifySkill("docx", "Create and edit Word documents")).toBe("docs");
    });

    it("classifies design skills", () => {
      expect(classifySkill("canvas-design", "Create visual art and illustrations")).toBe("design");
      expect(classifySkill("frontend-design", "Guidance for distinctive visual design")).toBe("design");
    });

    it("classifies media skills", () => {
      expect(classifySkill("ffmpeg-usage", "基于 ffmpeg 的音视频处理")).toBe("media");
      expect(classifySkill("podcast-maker", "Create podcast episodes with TTS")).toBe("media");
    });

    it("classifies slides skills", () => {
      expect(classifySkill("pptx", "Create PowerPoint presentations")).toBe("slides");
      expect(classifySkill("slide-deck", "Generate presentation slides")).toBe("slides");
    });

    it("classifies product skills", () => {
      expect(classifySkill("PRD生成", "输入功能需求描述，生成结构化PRD文档")).toBe("product");
      expect(classifySkill("user-story", "Decompose user stories with acceptance criteria")).toBe("product");
    });

    it("classifies collab skills", () => {
      expect(classifySkill("feishu-bot", "飞书审批流程自动化")).toBe("collab");
      expect(classifySkill("notion-sync", "Sync to Notion workspace")).toBe("collab");
    });

    it("classifies info skills", () => {
      expect(classifySkill("rss-reader", "抓取 RSS feed 并汇总")).toBe("info");
      expect(classifySkill("web-scraper", "Scrape and search web content")).toBe("info");
    });

    it("classifies devops skills", () => {
      expect(classifySkill("deploy-to-vercel", "Deploy projects to Vercel")).toBe("devops");
      expect(classifySkill("docker-compose", "Generate docker-compose.yml")).toBe("devops");
    });

    it("classifies AI/Agent skills", () => {
      expect(classifySkill("prompt-engineer", "Optimize LLM prompts")).toBe("ai");
      expect(classifySkill("rag-builder", "Build RAG pipelines with embeddings")).toBe("ai");
    });

    it("falls back to 'other' for unrecognized skills", () => {
      expect(classifySkill("my-custom-widget", "A very specific niche purpose")).toBe("other");
    });

    it("handles missing description gracefully", () => {
      expect(classifySkill("some-skill")).toBe("other");
      expect(classifySkill("pdf-tools")).toBe("docs");
    });

    it("prioritizes first matching rule", () => {
      // "code" in "coding" matches "coding" before "ai"
      expect(classifySkill("code-assistant", "Help with code and debugging")).toBe("coding");
    });
  });

  describe("classifySkills", () => {
    it("returns correct counts for a mixed batch", () => {
      const skills = [
        { name: "pdf", description: "PDF file operations" },
        { name: "docx", description: "Word documents" },
        { name: "pptx", description: "PowerPoint presentations" },
        { name: "web-scraper", description: "Scrape web content" },
        { name: "mystery-tool", description: "Unknown purpose" },
      ];
      const counts = classifySkills(skills);
      expect(counts.get("docs")).toBe(2);
      expect(counts.get("slides")).toBe(1);
      expect(counts.get("info")).toBe(1);
      expect(counts.get("other")).toBe(1);
    });

    it("returns empty map for empty input", () => {
      expect(classifySkills([]).size).toBe(0);
    });
  });

  describe("SKILL_CATEGORIES", () => {
    it("includes 'other' as the last category", () => {
      const last = SKILL_CATEGORIES[SKILL_CATEGORIES.length - 1];
      expect(last.key).toBe("other");
    });

    it("has unique keys", () => {
      const keys = new Set(SKILL_CATEGORIES.map((c) => c.key));
      expect(keys.size).toBe(SKILL_CATEGORIES.length);
    });

    it("every category has required fields", () => {
      for (const cat of SKILL_CATEGORIES) {
        expect(cat.key).toBeTruthy();
        expect(cat.i18nKey).toMatch(/^skills\.category\./);
        expect(cat.fallbackLabel).toBeTruthy();
        expect(cat.pillClass).toBeTruthy();
        expect(cat.pillActiveClass).toBeTruthy();
      }
    });
  });
});
