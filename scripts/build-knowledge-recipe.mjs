// 从 assets/recipes/knowledge/ 模板 SSOT 生成 knowledge-baseline.recipe.md（A2 分发载体）。
// 用法: node scripts/build-knowledge-recipe.mjs
// 产物: src-tauri/assets/recipes/knowledge/knowledge-baseline.recipe.md
// 说明: recipe frontmatter 使用 JSON 引号标量发射 YAML（serde_yaml 兼容），
//       projectTemplates 由 recipe_composer 的携带扩展在安装时落盘（safe-install）。
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const srcDir = path.join(root, "src-tauri", "assets", "recipes", "knowledge");
const outFile = path.join(srcDir, "knowledge-baseline.recipe.md");
const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));

const manifest = JSON.parse(fs.readFileSync(path.join(srcDir, "recipe.json"), "utf8"));

const q = (s) => JSON.stringify(s); // JSON 双引号标量 == 合法 YAML 双引号标量

const projectTemplates = manifest.templates.map((t) => ({
  path: t.target.replace(/\\/g, "/"),
  content: fs.readFileSync(path.join(srcDir, t.source), "utf8").replace(/\r\n/g, "\n"),
}));

const recipe = {
  schemaVersion: 1,
  name: "knowledge-baseline",
  description:
    "Knowledge 五层目录约定脚手架（A2 分发载体）：main/applications/candidate/personal/template 五层 + ROUTING.md 渐进式加载路由 + KNOWLEDGE-RULES.md 治理规则。安装到项目 knowledge/，safe-install 不覆盖。",
  presetId: "rd-loop",
  projectType: "backend",
  modules: [],
  stages: [],
  artifacts: [],
  rules: [
    {
      name: "knowledge_is_context_not_fact",
      value: "code-is-truth",
      description: "KB 提供稳定上下文，当前代码仍然是实现事实；错误知识比没有知识更危险。",
    },
    {
      name: "candidate_gate",
      value: "owner-review-required",
      description: "candidate/ 与 personal/ 内容未经验收不进正式区，不作为契约引用。",
    },
  ],
  projectTemplates,
  notes:
    "由 scripts/build-knowledge-recipe.mjs 从 assets/recipes/knowledge/*.tmpl 生成；修改模板后重新运行脚本。用户将本文件复制到 <project>/.opensunstar/recipe/ 后执行 os recipe install --name knowledge-baseline。",
  generatedAt: new Date().toISOString(),
  opensunstarVersion: pkg.version,
};

const yamlLines = [];
yamlLines.push(`schemaVersion: ${recipe.schemaVersion}`);
yamlLines.push(`name: ${q(recipe.name)}`);
yamlLines.push(`description: ${q(recipe.description)}`);
yamlLines.push(`presetId: ${q(recipe.presetId)}`);
yamlLines.push(`projectType: ${q(recipe.projectType)}`);
yamlLines.push(`modules: []`);
yamlLines.push(`stages: []`);
yamlLines.push(`artifacts: []`);
yamlLines.push(`rules:`);
for (const r of recipe.rules) {
  yamlLines.push(`  - name: ${q(r.name)}`);
  yamlLines.push(`    value: ${q(r.value)}`);
  yamlLines.push(`    description: ${q(r.description)}`);
}
yamlLines.push(`projectTemplates:`);
for (const t of projectTemplates) {
  yamlLines.push(`  - path: ${q(t.path)}`);
  yamlLines.push(`    content: ${q(t.content)}`);
}
yamlLines.push(`notes: ${q(recipe.notes)}`);
yamlLines.push(`generatedAt: ${q(recipe.generatedAt)}`);
yamlLines.push(`opensunstarVersion: ${q(recipe.opensunstarVersion)}`);

const body = [
  `# ${recipe.name}`,
  ``,
  `> ${recipe.description}`,
  ``,
  `## Quick Reference`,
  ``,
  `| Key | Value |`,
  `|---|---|`,
  `| Preset | \`rd-loop\` |`,
  `| Project Type | \`backend\` |`,
  `| Carried Templates | ${projectTemplates.length} |`,
  `| Schema | v1 |`,
  ``,
  `## Carried Templates`,
  ``,
  ...projectTemplates.map((t) => `- \`${t.path}\``),
  ``,
  `## 安装方式（A2 外插拔）`,
  ``,
  `1. 将本文件复制到 \`<project>/.opensunstar/recipe/knowledge-baseline.recipe.md\`；`,
  `2. 执行 \`os recipe install --project-path . --name knowledge-baseline --change-id <CHG-xxx> --yes\`；`,
  `3. 安装器按 projectTemplates 落盘 knowledge/ 五层与 ROUTING，已存在文件一律跳过。`,
  ``,
  `Parse the YAML frontmatter between \`---\` delimiters for structured data.`,
  ``,
].join("\n");

const out = `---\n${yamlLines.join("\n")}\n---\n\n${body}`;
fs.writeFileSync(outFile, out, "utf8");
console.log("OK ->", path.relative(root, outFile), out.length, "bytes;", projectTemplates.length, "templates");
