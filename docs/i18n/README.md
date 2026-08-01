# OpenSunstar 国际化（i18n）指南

本目录记录 UI 文案国际化的约定、术语表与 key 对齐基线，供后续扩展韩语（ko）、越南语（vi）等语种时使用。

## 产品定位（源码侧边栏基线）

**主标题：** 本地优先，一站式统一管理你的 AI 编程工作流工程化配置平台

**副标题：** 跨多项目组合矩阵以AI驱动的项目驾驶舱，一站式帮你基于项目的AI资产配置&工作流编排和跨工具跨设备Agent扩展配置同步

当前 README 按真实侧边栏理解 OpenSunstar：项目驾驶舱、我的项目、项目配置（AI资产配置 / 工作流编排）、Agent 配置（MCP / Skills / Prompt & Rules / Commands / Hooks / Ignore / Permissions / Subagents / Convert）、AI模型（快速接入 / Context / AI Tokens）、同步与协作、设置。快速接入文案统一为：**预设22+供应商，支持用户自定义配置更多供应商（含聚合/中转站）**。

## 技术栈

- **运行时**：[`i18next`](https://www.i18next.com/) + [`react-i18next`](https://react.i18next.com/)
- **资源文件**：`src/i18n/locales/*.json`（嵌套 JSON，dot-path key）
- **回退链**：见 `src/i18n/index.ts` 的 `FALLBACK_CHAINS`，**每条链末端都是 `zh`**

## 源语言（Source of Truth）

**中文是源语言，而中文的源头是源码里的 `defaultValue`。**

写新文案只写一次中文，写在 `t()` 调用里：

```tsx
t("kanban.governance.title", { defaultValue: "配置生效率" })
```

然后 `pnpm i18n:sync` 把它落进 `zh.json`。**不要手写 `zh.json`。**

翻译走另一条路：`zh.json` → `en` / `zh-TW` / `ja`，由 `pnpm i18n:check` 的棘轮盯着。

```
源码 defaultValue ──i18n:sync──▶ zh.json ──人工/机器翻译──▶ en / zh-TW / ja
                   （自动）                  （i18n:check 盯缺口）
```

> **为什么不再以 `en.json` 为源语言**
>
> 这份文档从前写着「以 `en.json` 为唯一源语言，先改 en 再同步其他 locale」——
> 那条流程从来没有真正执行过。实测 `en` 是 `zh` 的**严格子集**，几百个只有中文
> 才有的 key 就是这么来的。以 en 为源时 `i18n:check` 长期报「zh 缺 0 个」，
> 而真实缺口全在比对范围之外。与其维护一条没人走的流程，不如把规矩改成实际在
> 走的那条。

### 为什么需要 `i18n:sync`

`i18n:check` 只比对 locale JSON 之间的 key 对齐，**看不见源码**。于是有一整类
漂移它抓不到：同一句中文写了两份（源码 `defaultValue` 一份、`zh.json` 一份），
改文案时漏掉任意一份都不会有人报错 —— 而且两份的可见范围还不一样：

| | 单元测试里看到的 | 真实应用里看到的 |
| --- | --- | --- |
| 文案来源 | `defaultValue` | `zh.json` |
| 原因 | `tests/setupTests.ts` 用**空 resources** 初始化 i18n | 正常加载 locale |

两边漂了，测试照样全绿，界面是另一套文案。`methodology.sidebar` 就这么漂过
（`defaultValue` 写「工作流与治理」，`zh.json` 写「项目治理」）。

### `i18n:sync` 不会做的事

- **不覆盖已有文案。** 只在 `zh.json` 缺这个 key 时写入。两边不一致时只报告，
  要覆盖得显式 `--adopt-drift`。这条是硬需求：历史上大量 `defaultValue` 当年
  是按「英文兜底」写的（`designContract.*` 整块、`common.clear` …），无条件让
  源码获胜等于把上百条中文界面文案改回英文。
- **不删 key。** 源码里有大量 `` t(`a.b.${x}`) `` 动态 key，静态扫描看不见，
  按「没扫到就是没用到」删除会删掉真在用的文案。
- **不碰 `en` / `ja` / `zh-TW`。** 那三份不该被中文兜底文案覆盖。

历史欠账（漂移 / 同 key 两套 defaultValue / 运行时表达式 defaultValue）记在
[`sync-baseline.json`](./sync-baseline.json) 里做棘轮，**只许降不许升**。

## 校验命令

```bash
# —— 源码 → zh.json ——
pnpm i18n:sync           # 把源码 defaultValue 里的新 key 补进 zh.json
pnpm i18n:sync:check     # 只检查（CI 用）：有未落盘的新 key 即失败
pnpm i18n:sync:list      # 同上 + 打印全部明细
pnpm i18n:sync:baseline  # 补完历史欠账后收紧棘轮

# —— zh.json → 其余语言 ——
pnpm i18n:check          # 打印报告 + 对比 baseline 防回归
pnpm i18n:check:list     # 列出缺失/多余 key 样例
pnpm i18n:baseline       # 补全翻译后刷新基线
pnpm i18n:check:strict   # 严格模式：任一 locale 有 missing/extra 即失败
```

两条命令都在 CI 里（`.github/workflows/ci.yml`），方向不同，缺一不可。

## 回退链

`src/i18n/index.ts` 的 `FALLBACK_CHAINS` 按「哪份最全」排，而不是一律回落英文：

| 当前语言 | 回退顺序 |
| -------- | -------- |
| `zh-TW`  | `zh` → `en` |
| `ja`     | `en` → `zh` |
| `en`     | `zh` |
| 其他     | `zh` → `en` |

**每条链末端必须是 `zh`** —— 它是唯一保证有值的那份。原来一律 `fallbackLng: "en"`
的后果是：`ja` 缺的键去问 `en`，`en` 也没有 → i18next 直接把 **key 字符串**渲染到
界面上（形如 `kanban.governance.title`）。全仓约 1300 处 `t()` 没写 `defaultValue`，
这些位置一个兜底都没有。宁可让日语用户看到一句中文，也好过看到一个变量名。

这条不变量由 `src/i18n/coverage.test.ts` 钉死。

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

- [ ] 在 `src/i18n/locales/` 新增 `ko.json`（**从 `zh.json` 复制结构** —— 它才是最全的那份）
- [ ] 更新 `src/i18n/index.ts`：`Language` 类型、`resources`、`getInitialLanguage()`
- [ ] 在 `FALLBACK_CHAINS` 里加一条，**末端必须是 `zh`**（否则 `coverage.test.ts` 会失败）
- [ ] 更新设置页语言选择器
- [ ] 先在 [`glossary.md`](./glossary.md) 补全术语，再批量翻译 JSON，避免同一概念多种译法
- [ ] 运行 `pnpm i18n:check:list` 确认 key 对齐，然后 `pnpm i18n:baseline` 落基线
- [ ] （可选）接入 Crowdin / Weblate 做社区校对

## 相关文件

| 文件 | 用途 |
| ---- | ---- |
| [`glossary.md`](./glossary.md) | 核心术语表 |
| [`baseline.json`](./baseline.json) | `zh` → 其余语言的缺口棘轮 |
| [`sync-baseline.json`](./sync-baseline.json) | 源码 ↔ `zh.json` 的历史欠账棘轮 |
| [`scripts/i18n-sync.mjs`](../../scripts/i18n-sync.mjs) | 源码 `defaultValue` → `zh.json` |
| [`scripts/i18n-check.mjs`](../../scripts/i18n-check.mjs) | `zh.json` → 其余语言对齐校验 |
| [`src/i18n/coverage.test.ts`](../../src/i18n/coverage.test.ts) | 回退链 / key 形状 / 孤儿键守门测试 |
