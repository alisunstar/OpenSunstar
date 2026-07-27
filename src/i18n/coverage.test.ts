import { describe, expect, it } from "vitest";

import en from "./locales/en.json";
import ja from "./locales/ja.json";
import zh from "./locales/zh.json";
import zhTW from "./locales/zh-TW.json";
import { FALLBACK_CHAINS } from "./index";

/**
 * i18n 的三条不变量。
 *
 * 这些断言只看 JSON 和配置，不渲染组件 —— 因为组件测试恰好是**看不见** i18n
 * 问题的地方：`tests/setupTests.ts` 用空 resources 初始化 i18n，于是
 * `t(key, { defaultValue })` 在测试里永远返回 defaultValue，而真实用户看到的
 * 永远是 `zh.json`。两边漂了，组件测试照样全绿。
 *
 * 源码 `defaultValue` → `zh.json` 的同步由 `pnpm i18n:sync --check` 守（它要
 * 用 TypeScript AST 扫全仓，放进 vitest 太慢）。这里守的是 JSON 之间、以及
 * JSON 与回落配置之间的一致性。
 */

const LOCALES = { zh, "zh-TW": zhTW, en, ja } as const;
type LocaleName = keyof typeof LOCALES;

type Json = { [k: string]: string | Json };

/** 把嵌套 JSON 压平成 `a.b.c` → 值，顺带记下哪些前缀是对象节点。 */
function flatten(obj: Json, prefix = "", out = new Map<string, string>()) {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object") flatten(v as Json, key, out);
    else out.set(key, String(v));
  }
  return out;
}

const FLAT = Object.fromEntries(
  Object.entries(LOCALES).map(([name, data]) => [
    name,
    flatten(data as unknown as Json),
  ]),
) as Record<LocaleName, Map<string, string>>;

describe("回落链末端必须是 zh", () => {
  /**
   * 这是「界面上不出现裸 key」的唯一保障。`zh.json` 是唯一全量的那份，任何
   * 一条不以 zh 收尾的链，末端语言缺键时 i18next 会把 key 本身渲染出来。
   */
  it("每一条链（含 default）都以 zh 收尾或本身就是 zh", () => {
    for (const [lng, chain] of Object.entries(FALLBACK_CHAINS)) {
      expect(chain.length, `${lng} 的回落链不能为空`).toBeGreaterThan(0);
      expect(
        chain.includes("zh") || lng === "zh",
        `${lng} 的回落链是 [${chain.join(" → ")}]，末端不是 zh —— ` +
          `该语言缺键时界面会直接显示 key 字符串`,
      ).toBe(true);
    }
  });

  it("覆盖全部四种语言，新增语种不会悄悄漏配", () => {
    for (const lng of Object.keys(LOCALES)) {
      if (lng === "zh") continue; // zh 自己就是兜底，无需链
      expect(
        FALLBACK_CHAINS[lng],
        `${lng} 没有回落链，会落到 default`,
      ).toBeDefined();
    }
  });
});

describe("同一前缀不能既是字符串又是命名空间", () => {
  /**
   * i18next 用 `.` 做层级分隔，所以 `t("a.b")` 和 `t("a.b.c")` 不可能同时生效：
   * JSON 里 `a.b` 写成字符串，`a.b.c` 就永远查不到；写成对象，`a.b` 就查不到。
   *
   * **分工说明**：源码里的这类冲突由 `pnpm i18n:sync --check` 抓（它一发现就
   * 拒绝写盘并退出 1，所以冲突进不了 JSON，本测试也就看不到）—— `skills.info`
   * （ⓘ 按钮 tooltip）与 `skills.info.*`（详情面板 7 个字段标签）共存了很久，
   * 正是被那道检查逼出来的。
   *
   * 这里守的是另一半：**手写 JSON 时引入的冲突**。四份 locale 有人工翻译也有
   * 机器翻译，谁都可能把某个前缀改成另一种形状，那条路径绕开了 i18n:sync。
   */
  it.each(Object.keys(LOCALES) as LocaleName[])("%s.json", (name) => {
    const keys = FLAT[name];
    const collisions: string[] = [];
    for (const key of keys.keys()) {
      const segs = key.split(".");
      for (let i = 1; i < segs.length; i += 1) {
        const ancestor = segs.slice(0, i).join(".");
        if (keys.has(ancestor)) {
          collisions.push(`"${ancestor}" 既是字符串又是 "${key}" 的父节点`);
        }
      }
    }
    expect(collisions, collisions.join("\n")).toEqual([]);
  });
});

describe("改过名的核心导航词四种语言都得有", () => {
  /**
   * 这批 key 在第三梯队被反复改名（跨项目工作区 → 工作区、跨Agent配置 →
   * Agent 配置、治理总览 → 配置生效率、项目落地 → AI 资产总览）。改名最容易
   * 漏的就是 `ja.json` —— 它整块缺过 `workspace.tabs`。
   *
   * 只断言「存在且非空」，不断言具体译文：译文该由 `docs/i18n/glossary.md`
   * 管，写死在测试里会让每次文案微调都变成改测试。
   */
  const NAV_KEYS = [
    "workspace.title",
    "workspace.tabs.dashboard",
    "workspace.tabs.board",
    "workspace.tabs.assetsMatrix",
    "sidebar.agentConfig",
  ];

  it.each(NAV_KEYS)("%s", (key) => {
    for (const name of Object.keys(LOCALES) as LocaleName[]) {
      const value = FLAT[name].get(key);
      expect(value, `${name}.json 缺 ${key}`).toBeTruthy();
    }
  });
});

describe("zh 是最全的那份 —— 回落链的前提", () => {
  /**
   * 上面「末端必须是 zh」只有在 zh 真的最全时才有意义。这里用覆盖率下限把它
   * 钉住：别的语言可以慢慢补，但不能出现 zh 缺、别人有的键（那种键回落到 zh
   * 依然是裸 key）。
   *
   * 允许少量例外：`ja` 有 8 个 `workspace.*` 孤儿键是废弃功能的残留，
   * `zh-TW` 也有历史孤儿。这里给一个只降不升的上限，而不是要求归零。
   */
  const ORPHAN_CEILING: Record<string, number> = {
    "zh-TW": 36,
    en: 0,
    ja: 34,
  };

  it.each(Object.keys(ORPHAN_CEILING))(
    "%s.json 里 zh 没有的孤儿键不超过上限",
    (name) => {
      const orphans = [...FLAT[name as LocaleName].keys()].filter(
        (k) => !FLAT.zh.has(k),
      );
      expect(
        orphans.length,
        `${name}.json 多出 ${orphans.length} 个 zh 没有的键（上限 ${ORPHAN_CEILING[name]}）：\n` +
          `${orphans.slice(0, 10).join("\n")}`,
      ).toBeLessThanOrEqual(ORPHAN_CEILING[name]);
    },
  );
});
