# OpenSunstar 国际化（i18n）指南

本目录记录 UI 文案国际化的约定、术语表与 key 对齐基线，供后续扩展韩语（ko）、越南语（vi）等语种时使用。

## 技术栈

- **运行时**：[`i18next`](https://www.i18next.com/) + [`react-i18next`](https://react.i18next.com/)
- **资源文件**：`src/i18n/locales/*.json`（嵌套 JSON，dot-path key）
- **回退语言**：`en`（见 `src/i18n/index.ts` 中 `fallbackLng: "en"`）

## 源语言（Source of Truth）

**以 `en.json` 为唯一源语言。**

新增或修改 UI 文案时：

1. **先改** `src/i18n/locales/en.json`
2. 再同步到其他 locale（`zh`、`zh-TW`、`ja`，以及未来的 `ko`、`vi`）
3. 提交前运行 `pnpm i18n:check`，确保未引入 key 漂移

> **当前已知偏差（2026-07-08）**
>
> | Locale | 相对 `en` 缺失 | 额外 key | 说明 |
> | ------ | -------------- | -------- | ---- |
> | `zh`   | 0              | 66       | 含未回写至 `en` 的历史 key，需逐步合并 |
> | `ja`   | 507            | 34       | 待补全 |
> | `zh-TW`| 612            | 36       | 待补全，运行时缺失项回退英文 |
>
> 基线见 [`baseline.json`](./baseline.json)。CI 会在缺失 key **数量增加**时失败，防止回归。

## 校验命令

```bash
# 默认：打印报告 + 对比 baseline 防回归
pnpm i18n:check

# 列出缺失/多余 key 样例（最多 20 条）
pnpm i18n:check:list

# 补全翻译后刷新基线
pnpm i18n:baseline

# 严格模式：任一 locale 有 missing/extra 即失败（补全后可启用）
pnpm i18n:check:strict
```

## 占位符与格式

- 保留 i18next 插值：`{{count}}`、`{{name}}`、`{{error}}` 等，**翻译时不得删除或改名**
- 保留 HTML / Markdown 片段（如 `<strong>`、换行）的结构
- 专有名词见 [`glossary.md`](./glossary.md)，各语种应保持一致

## 应用内语言 vs 文档语言

| 类型 | 位置 | 当前语种 |
| ---- | ---- | -------- |
| **应用 UI** | `src/i18n/locales/` | `zh`、`zh-TW`、`en`、`ja` |
| **README** | 仓库根目录 `README*.md` | `en`、`zh`、`ja`、`de`；繁体 README 筹备中 |
| **用户手册** | `docs/user-manual/` | `en`、`zh`、`zh-TW`、`ja`、`de` |

GitHub 默认 README 语言导航已加入繁体链接（暂指向用户手册，完整 `README_ZH_TW.md` 列入后续阶段）。

## 扩展新语种 checklist

以韩语（`ko`）为例：

- [ ] 在 `src/i18n/locales/` 新增 `ko.json`（从 `en.json` 复制结构）
- [ ] 更新 `src/i18n/index.ts`：`Language` 类型、`resources`、`getInitialLanguage()`
- [ ] 更新设置页语言选择器
- [ ] 运行 `pnpm i18n:check:list` 确认 key 对齐
- [ ] 更新 [`glossary.md`](./glossary.md) 韩语列
- [ ] （可选）接入 Crowdin / Weblate 做社区校对

## 相关文件

| 文件 | 用途 |
| ---- | ---- |
| [`glossary.md`](./glossary.md) | 核心术语表 |
| [`baseline.json`](./baseline.json) | key 对齐基线（CI 防回归） |
| [`scripts/i18n-check.mjs`](../../scripts/i18n-check.mjs) | 校验脚本 |
