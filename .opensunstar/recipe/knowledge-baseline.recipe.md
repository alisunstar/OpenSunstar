---
schemaVersion: 1
name: "knowledge-baseline"
description: "Knowledge 五层目录约定脚手架（A2 分发载体）：main/applications/candidate/personal/template 五层 + ROUTING.md 渐进式加载路由 + KNOWLEDGE-RULES.md 治理规则。安装到项目 knowledge/，safe-install 不覆盖。"
presetId: "rd-loop"
projectType: "backend"
modules: []
stages: []
artifacts: []
rules:
  - name: "knowledge_is_context_not_fact"
    value: "code-is-truth"
    description: "KB 提供稳定上下文，当前代码仍然是实现事实；错误知识比没有知识更危险。"
  - name: "candidate_gate"
    value: "owner-review-required"
    description: "candidate/ 与 personal/ 内容未经验收不进正式区，不作为契约引用。"
projectTemplates:
  - path: "knowledge/ROUTING.md"
    content: "# ROUTING — {{project_name}} 知识渐进式加载路由\n\n> 生成时间：{{date}}\n> 本文件是知识库的入口：通过关键词/业务身份/Topic/接口/状态/模型定位候选域→候选应用→知识入口→仓库路径，实现渐进式加载。\n> 核心原则：KB 提供稳定上下文，当前代码仍然是实现事实；错误知识比没有知识更危险。\n\n## 路由规则\n\n按以下顺序匹配，命中即加载对应知识，未命中则继续向下：\n\n### 1. 业务身份路由\n\n| 业务身份 | 候选域 | 候选应用 | 知识入口 |\n|---------|--------|---------|---------|\n| {{business_identity}} | {{domain}} | {{application}} | applications/{{application}}/INDEX.md |\n\n### 2. 关键词路由\n\n| 关键词 | 候选域 | 知识入口 |\n|--------|--------|---------|\n| {{keyword}} | {{domain}} | main/{{domain}}.md |\n\n### 3. 接口/Topic 路由\n\n| 接口/Topic | 候选应用 | 知识入口 |\n|-----------|---------|---------|\n| {{topic}} | {{application}} | applications/{{application}}/domain/{{topic}}.md |\n\n### 4. 状态路由\n\n| 状态 | 加载策略 |\n|------|---------|\n| 新需求 | 先读 main/ 全局语境 → ROUTING 定位应用 → 应用 INDEX → domain/tech |\n| 变更需求 | 先读 applications/{{app}}/requirement.md → 对照变更映射 |\n| 故障排查 | 先读 applications/{{app}}/runbooks/ |\n\n## 渐进式加载层级\n\n1. **应用职责** — applications/{{app}}/INDEX.md（这个应用管什么）\n2. **product 主干** — applications/{{app}}/domain/product/（业务主干）\n3. **solution 差异** — applications/{{app}}/domain/solution/（方案差异）\n4. **base 索引** — applications/{{app}}/domain/base/（基础索引）\n5. **tech 规范** — applications/{{app}}/tech/（技术规范）\n6. **回当前代码核对** — 知识提供上下文，代码是事实\n\n## 候选区加载\n\n- candidate/ 下的知识标 source/evidence/confidence，未经验证不进正式区\n- personal/ 下的个人经验仅作参考，不作为契约\n\n---\n*此模板由 knowledge-baseline recipe 生成，落盘到 knowledge/ROUTING.md*\n"
  - path: "knowledge/KNOWLEDGE-RULES.md"
    content: "# Knowledge Rules — {{project_name}}\n\n> 生成时间：{{date}}\n> 知识库治理规则：生命周期、质量要求、回补约定。\n\n## 五层目录\n\n| 层 | 用途 | 准入 |\n|----|------|------|\n| `main/` | 业务域公共语境 | owner review 后可写入 |\n| `applications/` | 应用范围知识（product/solution/base/tech） | owner review 后可写入 |\n| `candidate/` | 候选暂存 | 任何人可写，标来源/证据/可信度 |\n| `personal/` | 个人经验 | 个人维护，不作为契约 |\n| `template/` | 强约束写作模板 | 变更需评审 |\n\n## 知识生命周期\n\n```\npersonal → candidate（标来源/证据/可信度）\n  → owner review → official（main/applications）\n    → 被引用 → 代码变化后 update/deprecated\n```\n\n## 模板 YAML frontmatter 约定\n\n每篇正式知识含以下元数据：\n\n```yaml\n---\nid: {{knowledge_id}}\ntype: {{product|solution|base|tech|runbook}}\ndomain: {{business_domain}}\napplication: {{app_name}}\nappType: {{backend|frontend|cli}}\nstatus: {{draft|official|deprecated}}\nsourceType: {{manual|backfill|imported}}\nowner: {{owner}}\nversion: {{version}}\nconfidence: {{high|medium|low}}\nstability: {{stable|evolving|volatile}}\nevidence: {{commit|pr|test|doc}}\ntags: [{{tags}}]\nanchors: [{{file_path:block_range}}]\n---\n```\n\n## 质量要求\n\n- 错误知识比没有知识更危险——不确定的标 `confidence: low`，放 candidate/\n- `anchors` 必须指向真实代码路径，代码变化时触发漂移检测\n- 正式区知识变更需经 owner review\n\n## 回补约定\n\n- rd-loop preset 的 backfill 阶段产 KNOWLEDGE-BACKFILL.md\n- 回补候选写入 candidate/，标 sourceType: backfill\n- GUI 验收导入后进 official，建立基线\n- 下一需求 ROUTING 命中已回补知识\n\n---\n*此模板由 knowledge-baseline recipe 生成，落盘到 knowledge/KNOWLEDGE-RULES.md*\n"
  - path: "knowledge/INDEX.md"
    content: "# Knowledge Index — {{project_name}}\n\n> 生成时间：{{date}}\n> 知识库总入口。先读 ROUTING.md 定位，再按层级加载。\n\n## 入口文件\n\n- [ROUTING.md](./ROUTING.md) — 渐进式加载路由\n- [KNOWLEDGE-RULES.md](./KNOWLEDGE-RULES.md) — 治理规则\n- [main/](./main/) — 业务域公共语境\n- [applications/](./applications/) — 应用范围知识\n- [candidate/](./candidate/) — 候选暂存\n- [personal/](./personal/) — 个人经验\n- [template/](./template/) — 强约束写作模板\n\n## 快速开始\n\n1. 新需求：读 ROUTING.md → 定位应用 → 读应用 INDEX\n2. 写知识：用 template/ 下的模板，填 YAML frontmatter\n3. 回补：rd-loop backfill 产物写入 candidate/，验收后导入\n\n---\n*由 knowledge-baseline recipe 生成*\n"
  - path: "knowledge/main/INDEX.md"
    content: "# Main — 业务域公共语境\n\n> 全局业务域通用知识，跨应用共享。\n\n## 目录\n\n| 文件 | 内容 |\n|------|------|\n| {{domain}}.md | {{domain}} 业务域定义 |\n\n---\n*由 knowledge-baseline recipe 生成*\n"
  - path: "knowledge/applications/INDEX.md"
    content: "# Applications — 应用范围知识\n\n> 每个应用一个子目录，含 INDEX + domain/(product/solution/base) + tech/。\n\n## 应用清单\n\n| 应用 | 路径 | 职责 |\n|------|------|------|\n| {{app_name}} | applications/{{app_name}}/ | {{app_scope}} |\n\n## 应用内读取路径\n\n应用职责 → product 主干 → solution 差异 → base 索引 → tech 规范 → 回当前代码核对\n\n---\n*由 knowledge-baseline recipe 生成*\n"
  - path: "knowledge/candidate/README.md"
    content: "# Candidate — 候选暂存区\n\n> 未经验证的知识暂存于此，标来源/证据/可信度。owner review 后可导入正式区。\n\n## 候选要求\n\n每篇候选含 YAML frontmatter：\n- `sourceType`: manual / backfill / imported\n- `evidence`: commit / pr / test / doc\n- `confidence`: high / medium / low\n\n## 回补候选\n\nrd-loop preset 的 backfill 阶段产生的候选写入本目录，经 GUI 验收导入后进正式区。\n\n---\n*由 knowledge-baseline recipe 生成*\n"
  - path: "knowledge/personal/README.md"
    content: "# Personal — 个人经验\n\n> 个人经验沉淀，仅作参考，不作为开发契约。正式知识请写入 main/ 或 applications/。\n\n---\n*由 knowledge-baseline recipe 生成*\n"
  - path: "knowledge/template/application.md"
    content: "---\nid: \"{{app_name}}-application\"\ntype: \"product\"\ndomain: \"{{business_domain}}\"\napplication: \"{{app_name}}\"\nappType: \"{{backend|frontend|cli}}\"\nstatus: \"draft\"\nsourceType: \"manual\"\nowner: \"{{owner}}\"\nversion: \"1.0.0\"\nconfidence: \"medium\"\nstability: \"evolving\"\nevidence: \"doc\"\ntags: []\nanchors: []\n---\n\n# {{app_name}} — 应用知识\n\n> 应用职责：{{app_scope}}\n\n## 业务主干（product）\n\n{{product_overview}}\n\n## 方案差异（solution）\n\n{{solution_diff}}\n\n## 基础索引（base）\n\n{{base_index}}\n\n## 技术规范（tech）\n\n{{tech_spec}}\n\n---\n*此模板由 knowledge-baseline recipe 生成，复制后填写具体内容*\n*落盘到 knowledge/applications/{{app_name}}/application.md*\n"
notes: "由 scripts/build-knowledge-recipe.mjs 从 assets/recipes/knowledge/*.tmpl 生成；修改模板后重新运行脚本。用户将本文件复制到 <project>/.opensunstar/recipe/ 后执行 os recipe install --name knowledge-baseline。"
generatedAt: "2026-08-05T14:49:44.221Z"
opensunstarVersion: "1.2.2"
---

# knowledge-baseline

> Knowledge 五层目录约定脚手架（A2 分发载体）：main/applications/candidate/personal/template 五层 + ROUTING.md 渐进式加载路由 + KNOWLEDGE-RULES.md 治理规则。安装到项目 knowledge/，safe-install 不覆盖。

## Quick Reference

| Key | Value |
|---|---|
| Preset | `rd-loop` |
| Project Type | `backend` |
| Carried Templates | 8 |
| Schema | v1 |

## Carried Templates

- `knowledge/ROUTING.md`
- `knowledge/KNOWLEDGE-RULES.md`
- `knowledge/INDEX.md`
- `knowledge/main/INDEX.md`
- `knowledge/applications/INDEX.md`
- `knowledge/candidate/README.md`
- `knowledge/personal/README.md`
- `knowledge/template/application.md`

## 安装方式（A2 外插拔）

1. 将本文件复制到 `<project>/.opensunstar/recipe/knowledge-baseline.recipe.md`；
2. 执行 `os recipe install --project-path . --name knowledge-baseline --change-id <CHG-xxx> --yes`；
3. 安装器按 projectTemplates 落盘 knowledge/ 五层与 ROUTING，已存在文件一律跳过。

Parse the YAML frontmatter between `---` delimiters for structured data.
