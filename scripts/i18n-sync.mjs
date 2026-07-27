#!/usr/bin/env node
/**
 * 把源码里的 `t("key", { defaultValue: "中文" })` 同步进 `src/i18n/locales/zh.json`。
 *
 * ## 为什么需要这个脚本
 *
 * `scripts/i18n-check.mjs` 只比对 locale JSON 之间的 key 对齐，看不见源码。于是有
 * 一整类漂移它抓不到：**同一句中文写了两份**（源码的 `defaultValue` 一份、`zh.json`
 * 一份），改文案时漏掉任意一份都不会有人报错 —— 而且两份的可见范围还不一样：
 * 单元测试的 i18n 资源是空的（`tests/setupTests.ts`），渲染时拿到的**永远是
 * defaultValue**；用户在真实应用里看到的**永远是 zh.json**。漂移时测试全绿，
 * 界面是另一套文案。`methodology.sidebar` 就这么漂过（defaultValue 写「工作流与
 * 治理」，zh.json 写「项目治理」）。
 *
 * 所以这里定一条规矩：**新文案的中文只写在源码的 `defaultValue` 里**，
 * `zh.json` 由本脚本补齐。写新功能只写一次中文，跑一次 `pnpm i18n:sync` 落盘。
 *
 * `docs/i18n/README.md` 曾写「以 en.json 为唯一源语言，先改 en 再同步其他 locale」——
 * 那从来没有真正执行过：`en` 是 `zh` 的严格子集，几十个只有中文有的 key 就是这么来的。
 * 与其维护一条没人走的流程，不如把规矩改成实际在走的那条。
 *
 * ## 为什么不是简单的「源码覆盖 zh.json」
 *
 * 第一次全量扫描的结果否掉了这个更省事的方案：249 处 key 两边文案不一致，其中
 * 一大半是**源码的 defaultValue 写的是英文、zh.json 里才是中文**（`designContract.*`
 * 整块、`common.clear`、`codexOauth.*` …）。那些 `defaultValue` 当年是按「英文兜底」
 * 写的，不是文案来源。让源码无条件获胜，等于把上百条中文界面文案改回英文。
 *
 * 因此策略按「有没有信息会被销毁」分开：
 *
 * | 情况 | 处理 |
 * |------|------|
 * | zh.json 缺这个 key | **自动写入**（无信息可毁，这是日常新增文案的路径） |
 * | 两边一致 | 跳过 |
 * | 两边不一致 | **只报告、不覆盖**；要覆盖得显式 `--adopt-drift` |
 * | 同一 key 在源码里有两套 defaultValue | 报告，必须人工统一 |
 *
 * 后两类是历史欠账，用 `docs/i18n/sync-baseline.json` 做棘轮：数量只许降不许升。
 *
 * ## 用法
 *
 *   node scripts/i18n-sync.mjs                  # 补齐 zh.json 缺失的 key
 *   node scripts/i18n-sync.mjs --check          # 只检查，退出码给 CI / 测试用
 *   node scripts/i18n-sync.mjs --adopt-drift    # 危险：让源码 defaultValue 覆盖 zh.json
 *   node scripts/i18n-sync.mjs --write-baseline # 补完欠账后刷新棘轮基线
 *   node scripts/i18n-sync.mjs --verbose        # 打印全部明细
 *
 * ## 不做的事
 *
 * - **不删 key。** 源码里有大量 `t(\`a.b.${x}\`)` 的动态 key，静态扫描看不见它们，
 *   按「没扫到就是没用到」删除会删掉真在用的文案。孤儿 key 只统计、不处理。
 * - **不碰 en / ja / zh-TW。** 那三份要么人工翻译要么机器翻译，都不该由中文兜底
 *   文案覆盖。它们的缺口由 `pnpm i18n:check` 的 baseline 棘轮盯着。
 */

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import ts from "typescript";

const repoRoot = process.cwd();
const srcDir = path.join(repoRoot, "src");
const zhPath = path.join(repoRoot, "src/i18n/locales/zh.json");
const baselinePath = path.join(repoRoot, "docs/i18n/sync-baseline.json");

const args = process.argv.slice(2);
const checkOnly = args.includes("--check");
const verbose = args.includes("--verbose");
const adoptDrift = args.includes("--adopt-drift");
const writeBaseline = args.includes("--write-baseline");

/** 判断一段文案是不是中文 —— 用来区分「真漂移」和「源码只写了英文兜底」。 */
const hasCJK = (s) => /[一-鿿㐀-䶿]/.test(s);

/** 测试文件里的 `t()` 调用不是产品文案，扫进来只会污染 zh.json。 */
const IGNORED_FILE = /\.(test|spec)\.tsx?$/;
const IGNORED_DIR = new Set(["locales", "node_modules"]);

/* ------------------------------------------------------------------ *
 * 1. 收集源码里的 t() 调用
 * ------------------------------------------------------------------ */

/** @returns {string[]} */
function collectSourceFiles(dir) {
  /** @type {string[]} */
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (IGNORED_DIR.has(entry.name)) continue;
      out.push(...collectSourceFiles(full));
    } else if (/\.tsx?$/.test(entry.name) && !IGNORED_FILE.test(entry.name)) {
      out.push(full);
    }
  }
  return out;
}

/**
 * `t` 可能是 `t(...)`、`i18n.t(...)`、`i18next.t(...)`。只认函数名，不做类型推断 ——
 * 误报的代价是往 zh.json 里多一个 key，漏报的代价是文案永远同步不了。
 */
function isTranslateCallee(callee) {
  if (ts.isIdentifier(callee)) return callee.text === "t";
  if (ts.isPropertyAccessExpression(callee)) return callee.name.text === "t";
  return false;
}

function readDefaultValue(objectLiteral) {
  for (const prop of objectLiteral.properties) {
    if (!ts.isPropertyAssignment(prop)) continue;
    const name = prop.name;
    const propName = ts.isIdentifier(name)
      ? name.text
      : ts.isStringLiteral(name)
        ? name.text
        : undefined;
    if (propName !== "defaultValue") continue;

    const init = prop.initializer;
    if (ts.isStringLiteral(init) || ts.isNoSubstitutionTemplateLiteral(init)) {
      return { kind: "literal", value: init.text };
    }
    // `` `返回项目：${name}` `` —— 值要到运行时才知道，落不进 JSON。
    // 正确写法是 i18next 插值：t("k", { defaultValue: "返回项目：{{name}}", name })
    return { kind: "dynamic", value: init.getText() };
  }
  return undefined;
}

/**
 * @typedef {{ key: string, value: string, file: string, line: number }} Entry
 */

/** @type {Map<string, Entry>} */
const entries = new Map();
/** @type {{ key: string, value: string, prev: Entry, file: string, line: number }[]} */
const codeConflicts = [];
/** @type {{ key: string, expr: string, file: string, line: number }[]} */
const dynamicDefaults = [];
let dynamicKeyCalls = 0;
let callsWithoutDefault = 0;

function scanFile(file) {
  const text = fs.readFileSync(file, "utf8");
  const sf = ts.createSourceFile(
    file,
    text,
    ts.ScriptTarget.Latest,
    /* setParentNodes */ true,
    file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const rel = path.relative(repoRoot, file).replace(/\\/g, "/");

  /** @param {ts.Node} node */
  const visit = (node) => {
    if (ts.isCallExpression(node) && isTranslateCallee(node.expression)) {
      const [arg0, arg1] = node.arguments;
      const line =
        sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1;

      if (arg0 && ts.isStringLiteral(arg0)) {
        const key = arg0.text;
        const def =
          arg1 && ts.isObjectLiteralExpression(arg1)
            ? readDefaultValue(arg1)
            : undefined;

        if (!def) {
          callsWithoutDefault += 1;
        } else if (def.kind === "dynamic") {
          dynamicDefaults.push({ key, expr: def.value, file: rel, line });
        } else {
          const prev = entries.get(key);
          if (prev && prev.value !== def.value) {
            codeConflicts.push({
              key,
              value: def.value,
              prev,
              file: rel,
              line,
            });
          } else if (!prev) {
            entries.set(key, { key, value: def.value, file: rel, line });
          }
        }
      } else if (arg0) {
        dynamicKeyCalls += 1;
      }
    }
    ts.forEachChild(node, visit);
  };

  visit(sf);
}

/* ------------------------------------------------------------------ *
 * 2. 与 zh.json 比对
 * ------------------------------------------------------------------ */

function getPath(obj, key) {
  let cur = obj;
  for (const seg of key.split(".")) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = cur[seg];
  }
  return cur;
}

/**
 * 沿 dot-path 写入。中途撞上非对象节点时返回错误而不是硬覆盖 —— 那种情况是
 * `t("a.b")` 与 `t("a.b.c")` 同时存在，属于 key 设计冲突，得人来判。
 */
function setPath(obj, key, value) {
  const segs = key.split(".");
  let cur = obj;
  for (let i = 0; i < segs.length - 1; i += 1) {
    const seg = segs[i];
    if (!(seg in cur)) {
      cur[seg] = {};
    } else if (typeof cur[seg] !== "object" || cur[seg] === null) {
      return `祖先节点 "${segs.slice(0, i + 1).join(".")}" 已是字符串`;
    }
    cur = cur[seg];
  }
  const last = segs[segs.length - 1];
  if (typeof cur[last] === "object" && cur[last] !== null) {
    return `"${key}" 在 zh.json 里是一个对象，不能写成字符串`;
  }
  cur[last] = value;
  return undefined;
}

function main() {
  for (const file of collectSourceFiles(srcDir)) scanFile(file);

  const zh = JSON.parse(fs.readFileSync(zhPath, "utf8"));

  /** @type {Entry[]} */
  const added = [];
  /** @type {(Entry & { was: string })[]} */
  const drifted = [];
  /** @type {{ key: string, reason: string }[]} */
  const shapeErrors = [];

  for (const entry of [...entries.values()].sort((a, b) =>
    a.key.localeCompare(b.key),
  )) {
    const current = getPath(zh, entry.key);

    if (typeof current === "object" && current !== null) {
      shapeErrors.push({
        key: entry.key,
        reason: "zh.json 里是对象，源码却当字符串用",
      });
      continue;
    }

    if (current === undefined) {
      // 唯一会自动写盘的分支：zh.json 里根本没有这个 key，写进去毁不掉任何东西。
      added.push(entry);
      if (!checkOnly) {
        const err = setPath(zh, entry.key, entry.value);
        if (err) shapeErrors.push({ key: entry.key, reason: err });
      }
      continue;
    }

    if (current === entry.value) continue;

    const was = String(current);
    drifted.push({
      ...entry,
      was,
      // 分类决定这条漂移该怎么修，见文件头的表。
      bucket: hasCJK(was)
        ? hasCJK(entry.value)
          ? "both-zh" // 两边都是中文且不同 —— 真漂移，得人来定
          : "src-en" // 源码是英文兜底，zh.json 才是真文案 —— 该改源码
        : hasCJK(entry.value)
          ? "zh-en" // zh.json 还是英文，源码已经中文化 —— 该改 zh.json
          : "neither", // 两边都不是中文，多半是技术串
    });
    if (adoptDrift && !checkOnly) {
      const err = setPath(zh, entry.key, entry.value);
      if (err) shapeErrors.push({ key: entry.key, reason: err });
    }
  }

  /* ---------------- 报告 ---------------- */

  console.log(
    `扫描 ${entries.size} 个静态 key（含 defaultValue），` +
      `另有 ${callsWithoutDefault} 处 t() 未写 defaultValue、` +
      `${dynamicKeyCalls} 处动态 key。`,
  );
  console.log("");

  const show = (label, list, fmt) => {
    if (list.length === 0) return;
    console.log(`${label}（${list.length}）:`);
    const limit = verbose ? list.length : 15;
    for (const item of list.slice(0, limit)) console.log(`  ${fmt(item)}`);
    if (list.length > limit) {
      console.log(`  … 另有 ${list.length - limit} 条（--verbose 看全部）`);
    }
    console.log("");
  };

  show(
    checkOnly
      ? "[缺失] zh.json 里没有 —— 跑 `pnpm i18n:sync` 即可补齐"
      : "[新增] 已写入 zh.json",
    added,
    (e) => `${e.key} = ${JSON.stringify(e.value)}   ← ${e.file}:${e.line}`,
  );

  const BUCKET_LABEL = {
    "src-en": "源码是英文兜底、zh.json 才是真文案 → 该把源码 defaultValue 改成中文",
    "both-zh": "两边都是中文且不同 → 真漂移，需人工定夺哪句对",
    "zh-en": "zh.json 还是英文、源码已中文化 → 该更新 zh.json（--adopt-drift）",
    neither: "两边都不是中文 → 多半是技术串，低优先",
  };
  const byBucket = { "src-en": [], "both-zh": [], "zh-en": [], neither: [] };
  for (const d of drifted) byBucket[d.bucket].push(d);

  if (drifted.length > 0) {
    console.log(
      `${adoptDrift && !checkOnly ? "[覆盖]" : "[漂移]"} 两边文案不一致（${drifted.length}）:`,
    );
    for (const [bucket, list] of Object.entries(byBucket)) {
      if (list.length === 0) continue;
      console.log(`  ── ${list.length} 处 · ${BUCKET_LABEL[bucket]}`);
      const limit = verbose ? list.length : 3;
      for (const e of list.slice(0, limit)) {
        console.log(`     ${e.key}   ← ${e.file}:${e.line}`);
        console.log(`       zh.json: ${JSON.stringify(e.was)}`);
        console.log(`       源码:    ${JSON.stringify(e.value)}`);
      }
      if (list.length > limit) {
        console.log(`     … 另有 ${list.length - limit} 条（--verbose 看全部）`);
      }
    }
    console.log("");
  }

  show(
    "[冲突] 同一个 key 在源码里有两套 defaultValue —— 渲染成哪句取决于先加载谁",
    codeConflicts,
    (c) =>
      `${c.key}\n      ${JSON.stringify(c.prev.value)}   ← ${c.prev.file}:${c.prev.line}\n      ${JSON.stringify(c.value)}   ← ${c.file}:${c.line}`,
  );
  show("[结构] key 形状冲突", shapeErrors, (s) => `${s.key} — ${s.reason}`);

  if (dynamicDefaults.length > 0) {
    console.log(
      `[欠账] ${dynamicDefaults.length} 处 defaultValue 是运行时表达式，落不进 JSON。`,
    );
    console.log(
      `        改成 i18next 插值即可入库：t(k, { defaultValue: "共 {{n}} 项", n })`,
    );
    if (verbose) {
      for (const d of dynamicDefaults) {
        console.log(`  ${d.key}   ← ${d.file}:${d.line}`);
      }
    }
    console.log("");
  }

  /* ---------------- 棘轮基线 ---------------- */

  const counts = {
    drift: drifted.length,
    codeConflicts: codeConflicts.length,
    dynamicDefaults: dynamicDefaults.length,
  };

  if (writeBaseline) {
    fs.mkdirSync(path.dirname(baselinePath), { recursive: true });
    fs.writeFileSync(
      baselinePath,
      `${JSON.stringify(
        {
          $comment:
            "i18n-sync 历史欠账棘轮：这些数字只许降不许升。补完欠账后跑 pnpm i18n:sync:baseline 刷新。",
          generatedAt: new Date().toISOString().slice(0, 10),
          ...counts,
        },
        null,
        2,
      )}\n`,
      "utf8",
    );
    console.log(`基线已写入 ${path.relative(repoRoot, baselinePath)}`);
    process.exit(0);
  }

  const baseline = fs.existsSync(baselinePath)
    ? JSON.parse(fs.readFileSync(baselinePath, "utf8"))
    : undefined;

  /** @type {string[]} */
  const regressions = [];
  if (baseline) {
    for (const [name, value] of Object.entries(counts)) {
      const expected = baseline[name];
      if (typeof expected !== "number") continue;
      if (value > expected) {
        regressions.push(`${name}: ${expected} → ${value}`);
      } else if (value < expected) {
        console.log(
          `[改善] ${name}: ${expected} → ${value}（跑 \`pnpm i18n:sync:baseline\` 收紧棘轮）`,
        );
      }
    }
  }

  /* ---------------- 退出码 ---------------- */

  if (shapeErrors.length > 0) {
    console.error("");
    console.error("key 形状冲突必须人工处理，未写入 zh.json。");
    process.exit(1);
  }

  if (checkOnly) {
    // 新增的 key 是硬失败：补它不需要任何判断，跑一条命令的事，
    // 放行只会让「写了中文却没落盘」重新变成常态。
    if (added.length > 0) {
      console.error(
        `i18n:sync --check 失败：${added.length} 个 key 只存在于源码 defaultValue。` +
          `\n运行 \`pnpm i18n:sync\` 落盘后重新提交。`,
      );
      process.exit(1);
    }
    if (regressions.length > 0) {
      console.error(
        `i18n:sync --check 失败：历史欠账变多了 —— ${regressions.join("；")}`,
      );
      process.exit(1);
    }
    console.log("zh.json 已覆盖源码里所有静态 defaultValue，欠账未增加。");
    process.exit(0);
  }

  if (regressions.length > 0) {
    console.log(`[警告] 欠账变多：${regressions.join("；")}`);
  }

  const writes = added.length + (adoptDrift ? drifted.length : 0);
  if (writes === 0) {
    console.log("zh.json 已是最新，无需改动。");
    process.exit(0);
  }

  fs.writeFileSync(zhPath, `${JSON.stringify(zh, null, 2)}\n`, "utf8");
  console.log(
    `已写入 ${path.relative(repoRoot, zhPath)}：新增 ${added.length}` +
      (adoptDrift ? `、覆盖 ${drifted.length}` : "") +
      "。",
  );
  console.log("其余语言的缺口由 `pnpm i18n:check` 的 baseline 棘轮盯着。");
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.stack : error);
  process.exit(1);
}
