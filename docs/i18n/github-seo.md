# GitHub 仓库 About / SEO 设置说明

GitHub 仓库主页的**浏览器标签标题**（`<title>`）由 GitHub 生成，README 无法直接写入 HTML `<title>`。

## 浏览器标题如何组成

| 是否填写 Description | 典型标题格式 |
| -------------------- | ------------ |
| 未填写 | `GitHub - alisunstar/OpenSunstar` |
| 已填写 | `GitHub - alisunstar/OpenSunstar: {Description 前若干字}` |

搜索引擎主要抓取：

1. **仓库 Description**（About 区短描述）
2. **Topics**（主题标签）
3. **默认 README.md** 首屏标题与段落（已加入中英文主副标题）
4. 各语言 README 链接（`README_ZH.md` 等）

## 推荐手动设置（仓库 Settings → General → About）

**Description（建议复制）：**

```text
一站式统一管理你的 AI 编程工作流工程化配置平台 · 跨多项目 AI 就绪度驾驶舱 · 方法论与工作流编排 · 跨工具跨设备 Agent 配置双向同步
```

**Topics（建议）：**

```text
ai, agent, mcp, claude-code, codex, gemini-cli, tauri, workflow, i18n, desktop-app
```

**Website（可选）：**

```text
https://alisunstar.github.io/OpenSunstar/
```

（若已启用 GitHub Pages 指向 `website/`）

## 已在本仓库 README 中实现的 SEO 内容

- `README.md`（GitHub 默认首页）：英文标题 + **中文主副标题**（便于中文检索命中）
- `README_ZH.md`：完整中文主副标题
- 语言导航含 **繁體中文** → `docs/user-manual/zh-TW/README.md`

## 无法通过 README 实现的部分

- 自定义浏览器 `<title>` 全文（需 GitHub 平台规则 + Description）
- 独立 `meta description` 标签（GitHub 不使用 README 内 HTML meta）
