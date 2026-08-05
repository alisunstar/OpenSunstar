---
title: RD 闭环 P1-0 经验（A2/B2/治理声明）
type: runbook
status: draft
source_files:
  - src-tauri/src/services/recipe_composer.rs
  - scripts/build-knowledge-recipe.mjs
  - scripts/package-rd-protocol.mjs
  - src-tauri/assets/workflow/presets/rd-loop.json
last_verified_commit: bdce254c7fea9f5b4c6dd495ef89f78205283847
last_verified: 2026-08-05
tags:
  - rd-loop
  - recipe
  - skill-pack
---

# RD 闭环 P1-0 经验

> CHG-p05-rd-loop-patch 回补 · 证据见 .specs/CHG-p05-rd-loop-patch/

## 1. A2：projectTemplates 是外部模板的 recipe 分发标准通道

recipe frontmatter 的 `projectTemplates: [{path, content}]` 由 recipe_composer 纯搬运落盘：safe-install 不覆盖、路径安全校验（拒绝绝对路径/`..`）、写入前纳入审计扫描。knowledge 五层与 ROUTING 经 `os recipe install --name knowledge-baseline` 到达用户项目。

## 2. B2：rd-protocol 分发三件套

`node scripts/package-rd-protocol.mjs` 产 `distrib/rd-protocol-<ver>.zip`（SKILL.md 位于 zip 根）→ 经 Skills 界面「从 ZIP 安装」/仓库安装/手工放入 `~/.agents/skills/rd-protocol/` → skills SSOT 同步到各目标 CLI。

## 3. rd-loop 治理声明

rd-loop 采用业务语义阶段命名，不适用 standard preset 的 S1—S5 语义规则；门禁强制以工件必选性为准；独立语义规则集于 P1 定义。

## 使用前必须核对

- recipe_composer.rs 的 projectTemplates 行为以源码与单测为准；
- 本页为 draft，经验收导入后转 active。
