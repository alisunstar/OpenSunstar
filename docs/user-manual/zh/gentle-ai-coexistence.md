# 与 Gentle-AI 等第三方配置器共存指南

> OpenSunstar P0-3 · 阶段 1 文档

## 1. 职责划分

| 工具 | 主要职责 | 典型写入位置 |
|------|----------|--------------|
| **OpenSunstar** | Provider / 代理、MCP 列表、Skills 安装与同步、项目组合治理、Readiness 评分 | `settings.json`、`~/.claude.json` MCP、`CLAUDE.md` 托管段、`.opensunstar/` |
| **Gentle-AI** | SDD 工作流、Engram 记忆、Persona、Trigger Rules、编排器 Prompt | `<!-- gentle-ai:* -->` 段落、`opencode.json` 编排段、`.atl/skill-registry.md` |

两者**不是竞品**，而是同一用户旅程上的互补：OpenSunstar 侧重「用好」（连接与治理），Gentle-AI 侧重「装好」（工作流与行为塑造）。

## 2. 推荐安装顺序

1. 安装并配置 **OpenSunstar**（Quick Start 或专家模式完成 Provider、代理、基础 MCP/Skills）。
2. 确认 Claude Code / Codex 等 CLI 能通过 OpenSunstar 正常访问 API。
3. 再运行 **gentle-ai install**，选择需要的 SDD / Engram / Persona 组件。
4. 避免在两边同时「全量覆盖」同一 Prompt 文件；OpenSunstar 仅写入 `<!-- opensunstar:managed-prompt -->` 段落。

## 3. OpenSunstar 托管标记

启用 Prompt 同步时，OpenSunstar 只更新以下段落，**不会删除**文件中的其他内容（含 gentle-ai 段落）：

```markdown
<!-- opensunstar:managed-prompt -->
（OpenSunstar 管理的 Prompt 正文）
<!-- /opensunstar:managed-prompt -->
```

## 4. 项目级 Agent 工件

OpenSunstar 会在项目根目录生成（供终端 Agent 读取）：

| 文件 | 用途 |
|------|------|
| `.opensunstar/agent-context-hints.md` | Readiness 缺口与建议（刷新评分时更新） |
| `.opensunstar/skill-registry.md` | 项目技能索引（路径列表，非摘要） |
| `.atl/skill-registry.md` | 与 gentle-ai 兼容的同名索引副本 |

## 5. 冲突排查

| 现象 | 可能原因 | 处理 |
|------|----------|------|
| SDD / Persona 段落消失 | OpenSunstar 旧版整文件覆盖 Prompt | 升级至支持 marker 的版本；从 gentle-ai 备份或 `gentle-ai sync` 恢复 |
| Provider 被改回官方 | gentle-ai GGA 与 OpenSunstar Provider 切换冲突 | Provider 以 OpenSunstar 为准；gentle-ai 侧跳过 Provider 组件 |
| MCP 重复或缺失 | 两边各写一套 MCP | 以 OpenSunstar MCP 面板为 SSOT，gentle-ai 仅补 Engram/Context7 |
| Skill 索引不一致 | 仅一方刷新 registry | 在 OpenSunstar 项目组合中刷新 Readiness，或关联项目 Skills 后自动更新 |

## 6. FAQ

**Q：必须同时安装 gentle-ai 吗？**  
否。OpenSunstar 可独立使用；本指南面向选择叠加工作流的用户。

**Q：Readiness 导出会调用 LLM 吗？**  
`agent-context-hints.md` 基于规则评分生成；仅当你在 UI 中配置了 AI 洞察 Provider 且存在缺项时，才可能附加 LLM 建议段落。

**Q：`.atl/skill-registry.md` 会覆盖 gentle-ai 生成的文件吗？**  
OpenSunstar 写入的是**索引表**，与 gentle-ai 格式兼容；若同时使用，建议在关联 Skills 或刷新 Readiness 后运行 `gentle-ai skill-registry refresh --force` 合并最新状态。

---

[返回用户手册目录](README.md)
