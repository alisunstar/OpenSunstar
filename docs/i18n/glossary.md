# OpenSunstar 国际化术语表（Glossary）

翻译 UI 文案、README 与用户手册时请遵循下表。  
**原则**：产品名、协议名、CLI 工具名保留英文；概念性词汇各语种应统一。

## 产品与定位

| English | 简体中文 | 繁體中文 | 日本語 | Deutsch | 备注 |
| ------- | -------- | -------- | ------ | ------- | ---- |
| OpenSunstar | OpenSunstar | OpenSunstar | OpenSunstar | OpenSunstar | 产品名不译 |
| AI coding workflow engineering | AI 编程工作流工程化 | AI 程式工作流工程化 | AI コーディングワークフロー工程 | AI-Coding-Workflow-Engineering | 战略定位用语 |
| AI readiness cockpit | AI 就绪度驾驶舱 | AI 就緒度駕駛艙 | AI レディネスコックピット | AI-Readiness-Cockpit | |
| Methodology & Orchestration | 方法论与编排 | 方法論與編排 | 方法論とオーケストレーション | Methodik & Orchestrierung | 侧栏模块名 |
| Quick Connect | 快速接入 | 快速接入 | クイック接続 | Schnellverbindung | 原 Quick Start 演进 |

## Agent 与资产

| English | 简体中文 | 繁體中文 | 日本語 | 备注 |
| ------- | -------- | -------- | ------ | ---- |
| Agent | Agent | Agent | Agent | 不译 |
| MCP | MCP | MCP | MCP | Model Context Protocol |
| Skills | Skills | Skills | Skills | |
| Prompts | Prompts | Prompts | プロンプト | |
| Commands | Commands | Commands | コマンド | |
| Hooks | Hooks | Hooks | フック | |
| Subagents | Subagents | Subagents | サブエージェント | |
| Permissions | Permissions | Permissions | 権限 | |
| Agent Readiness | Agent 就绪度 | Agent 就緒度 | Agent レディネス | |

## 工作流与方法论

| English | 简体中文 | 繁體中文 | 日本語 | 备注 |
| ------- | -------- | -------- | ------ | ---- |
| Recipe | Recipe | Recipe | Recipe | 自定义编排产物名 |
| Workflow profile | 工作流配置 | 工作流設定 | ワークフロープロファイル | `workflow.profile.json` |
| Design Contract | 设计契约 | 設計契約 | デザイン契約 | 生成 DESIGN.md |
| Preset Orchestration | 预设编排 | 預設編排 | プリセットオーケストレーション | |
| Custom Orchestration | 自定义编排 | 自訂編排 | カスタムオーケストレーション | |
| Methodology Framework | 方法论框架 | 方法論框架 | 方法論フレームワーク | |
| Stage graph | 阶段图 | 階段圖 | ステージグラフ | Recipe Composer |

## CLI 与供应商

| English | 简体中文 | 繁體中文 | 日本語 | 备注 |
| ------- | -------- | -------- | ------ | ---- |
| Claude Code | Claude Code | Claude Code | Claude Code | |
| Codex | Codex | Codex | Codex | |
| Gemini CLI | Gemini CLI | Gemini CLI | Gemini CLI | |
| Provider | 供应商 | 供應商 | プロバイダー | |
| Preset | 预设 | 預設 | プリセット | |
| API Key | API Key | API Key | API Key | |

## 工作区与治理

| English | 简体中文 | 繁體中文 | 日本語 | 备注 |
| ------- | -------- | -------- | ------ | ---- |
| Workspace | 工作区 | 工作區 | ワークスペース | 侧栏一级菜单；曾名「跨项目工作区」 |
| Today | 今日工作台 | 今日工作臺 | 今日のワークスペース | |
| Project board | 项目看板 | 項目看板 | プロジェクトボード | |
| AI assets | AI 资产总览 | AI 資產總覽 | AI アセット一覧 | 工作区第三个 Tab；项目级关联与就绪状态 |
| Agent Config | Agent 配置 | Agent 配置 | Agent 設定 | 侧栏一级菜单；全局资产 CRUD。曾名「跨Agent配置」，去掉了「跨」字前缀 |
| Workflow & Methodology | 流程与方法论 | 流程與方法論 | ワークフローと方法論 | 曾名「项目治理」 |
| Config effectiveness | 配置生效率 | 配置生效率 | 設定有効化率 | `GovernanceDashboard`；曾名「治理总览」。挂在「AI 资产总览」Tab 下，**不在**「项目看板」 |
| Repair drift | 修复漂移 | 修復漂移 | ドリフトを修復 | `health.action.repair`；曾名「查看修复」 |
| Project · AI Config | 项目 · AI 配置 | 專案 · AI 設定 | プロジェクト · AI 設定 | |

> 表中「—」= `ja.json` 尚缺该键，组件回落到中文兜底文案，待补。

> **「Agent 配置」与「AI 资产总览」是同一批实体（8 类）的两个作用域**，不是「可写 vs 只读」：
> 前者是全局库的增删改，后者看这些资产在各项目里的关联与生效情况。审查报告 §2.5 曾建议
> 改名成「资产库 / 项目落地」把作用域直接写进名字，产品决定保留沿用已久的旧名 —— 因此
> 这层区别目前只活在本表、`docs/kanban.md` 和界面上的 subtitle 里，属于已知欠账。翻译
> 这两个词时不要按字面理解成「库 vs 视图」。

## 同步与备份

| English | 简体中文 | 繁體中文 | 日本語 | 备注 |
| ------- | -------- | -------- | ------ | ---- |
| Sync & Backup | 同步备份 | 同步備份 | 同期とバックアップ | |
| Deep Link | Deep Link | Deep Link | Deep Link | |
| WebDAV | WebDAV | WebDAV | WebDAV | |

## 计划扩展语种（占位）

| English | 한국어 (ko) | Tiếng Việt (vi) | 备注 |
| ------- | ----------- | --------------- | ---- |
| Methodology & Orchestration | 방법론 및 오케스트레이션 | Phương pháp & Điều phối | 首稿待填 |
| Quick Connect | 빠른 연결 | Kết nối nhanh | |
| AI readiness cockpit | AI 준비 상태 cockpit | Bảng điều khiển AI readiness | |

> 扩展 `ko` / `vi` 时请先在本表补全术语，再批量翻译 JSON，避免同一概念多种译法。
