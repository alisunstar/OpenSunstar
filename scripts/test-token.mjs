/**
 * Token 连接测试脚本
 *
 * 支持测试 DeepSeek、OpenAI 及任意 OpenAI 兼容 API 的 Token 可用性。
 *
 * 用法:
 *   node scripts/test-token.mjs                          # 交互模式
 *   node scripts/test-token.mjs --provider deepseek --key sk-xxx
 *   node scripts/test-token.mjs --provider openai --key sk-xxx --url https://api.openai.com
 *   node scripts/test-token.mjs --provider deepseek --key "jwt|nextauth|device"
 */

import { createInterface } from "node:readline";

// ─── 配置 ──────────────────────────────────────────
const DEEPSEEK_BASE = "https://api.deepseek.com";
const OPENAI_BASE = "https://api.openai.com";

// ANSI color helpers
const C = {
  reset: "\x1b[0m",
  bold: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  cyan: "\x1b[36m",
};

// ─── 工具函数 ──────────────────────────────────────
function maskKey(key) {
  if (!key) return "(空)";
  if (key.length <= 12) return key.slice(0, 4) + "***" + key.slice(-4);
  return key.slice(0, 6) + "***" + key.slice(-6);
}

function hr(title = "") {
  const line = "─".repeat(60);
  if (title) console.log(`\n${C.bold}${C.cyan}${line}${C.reset}`);
  else console.log(`${C.dim}${line}${C.reset}`);
}

// ─── API 测试 ──────────────────────────────────────
async function testListModels(baseUrl, apiKey) {
  const url = `${baseUrl}/v1/models`;
  console.log(`${C.dim}  GET ${url}${C.reset}`);

  const res = await fetch(url, {
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
  });

  const body = await res.text();
  let data;
  try {
    data = JSON.parse(body);
  } catch {
    data = { _raw: body.slice(0, 500) };
  }

  return { status: res.status, ok: res.ok, data };
}

async function testChatPing(baseUrl, apiKey, model = "deepseek-chat") {
  const url = `${baseUrl}/v1/chat/completions`;
  console.log(`${C.dim}  POST ${url} (model=${model})${C.reset}`);

  const res = await fetch(url, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model,
      messages: [{ role: "user", content: "ping" }],
      max_tokens: 1,
      stream: false,
    }),
  });

  const body = await res.text();
  let data;
  try {
    data = JSON.parse(body);
  } catch {
    data = { _raw: body.slice(0, 500) };
  }

  return {
    status: res.status,
    ok: res.ok,
    data,
    tokensUsed: data?.usage
      ? {
          prompt: data.usage.prompt_tokens,
          completion: data.usage.completion_tokens,
          total: data.usage.total_tokens,
        }
      : null,
  };
}

// ─── 账户余额查询 ──────────────────────────────────
async function checkDeepseekBalance(apiKey) {
  console.log(`${C.dim}  GET https://api.deepseek.com/user/balance${C.reset}`);
  try {
    const res = await fetch("https://api.deepseek.com/user/balance", {
      headers: { Authorization: `Bearer ${apiKey}` },
    });
    const text = await res.text();
    try {
      return { status: res.status, data: JSON.parse(text) };
    } catch {
      return { status: res.status, data: { _raw: text.slice(0, 300) } };
    }
  } catch (e) {
    return { error: e.message };
  }
}

// ─── 综合测试 ──────────────────────────────────────
async function testToken({ provider, apiKey, baseUrl, model }) {
  hr(`${provider.toUpperCase()} Token 测试`);
  console.log(`${C.bold}Provider:${C.reset} ${provider}`);
  console.log(`${C.bold}Base URL:${C.reset} ${baseUrl}`);
  console.log(`${C.bold}API Key:${C.reset}  ${maskKey(apiKey)}`);
  if (model) console.log(`${C.bold}Model:${C.reset}    ${model}`);
  console.log();

  let allPassed = true;
  const results = [];

  // 1. List Models
  console.log(`${C.bold}[1/3]${C.reset} 列出可用模型…`);
  try {
    const r1 = await testListModels(baseUrl, apiKey);
    results.push({ name: "List Models", ...r1 });

    if (r1.ok) {
      const models = r1.data?.data ?? [];
      const count = Array.isArray(models) ? models.length : "?";
      console.log(`  ${C.green}✓ 成功${C.reset} — ${count} 个模型可用`);
      if (Array.isArray(models) && models.length > 0 && models.length <= 10) {
        const ids = models.map((m) => m.id).join(", ");
        console.log(`  ${C.dim}模型: ${ids}${C.reset}`);
      }
    } else {
      allPassed = false;
      const msg =
        r1.data?.error?.message || r1.data?._raw || JSON.stringify(r1.data);
      console.log(
        `  ${C.red}✗ 失败${C.reset} — HTTP ${r1.status}: ${msg.slice(0, 150)}`,
      );
    }
  } catch (e) {
    allPassed = false;
    results.push({ name: "List Models", ok: false, error: e.message });
    console.log(`  ${C.red}✗ 网络错误${C.reset} — ${e.message}`);
  }

  // 2. Chat Ping
  const testModel = model || (provider === "deepseek" ? "deepseek-chat" : "gpt-3.5-turbo");
  console.log(`\n${C.bold}[2/3]${C.reset} 发送测试请求 (ping, max_tokens=1)…`);
  try {
    const r2 = await testChatPing(baseUrl, apiKey, testModel);
    results.push({ name: "Chat Ping", ...r2 });

    if (r2.ok) {
      console.log(`  ${C.green}✓ 成功${C.reset} — 模型响应正常`);
      if (r2.tokensUsed) {
        console.log(
          `  ${C.dim}Token 消耗: prompt=${r2.tokensUsed.prompt}, completion=${r2.tokensUsed.completion}, total=${r2.tokensUsed.total}${C.reset}`,
        );
      }
    } else {
      allPassed = false;
      const msg =
        r2.data?.error?.message || r2.data?._raw || JSON.stringify(r2.data);
      console.log(
        `  ${C.red}✗ 失败${C.reset} — HTTP ${r2.status}: ${msg.slice(0, 200)}`,
      );
    }
  } catch (e) {
    allPassed = false;
    results.push({ name: "Chat Ping", ok: false, error: e.message });
    console.log(`  ${C.red}✗ 网络错误${C.reset} — ${e.message}`);
  }

  // 3. Balance (DeepSeek only)
  if (provider === "deepseek") {
    console.log(`\n${C.bold}[3/3]${C.reset} 查询账户余额…`);
    try {
      const r3 = await checkDeepseekBalance(apiKey);
      results.push({ name: "Balance", ...r3 });

      if (r3.error) {
        console.log(`  ${C.yellow}⚠ 查询失败${C.reset} — ${r3.error}`);
      } else if (
        r3.data?.is_available === false ||
        r3.data?.balance_infos ||
        r3.data?.currency
      ) {
        // DeepSeek balance response
        const infos = r3.data?.balance_infos;
        if (Array.isArray(infos) && infos.length > 0) {
          for (const info of infos) {
            console.log(
              `  ${C.green}✓ 余额${C.reset} — ${info.currency}: ${info.total_balance ?? info.balance ?? "?"}`,
            );
          }
        } else {
          console.log(`  ${C.yellow}⚠ 无法解析余额${C.reset}`);
          console.log(`  ${C.dim}${JSON.stringify(r3.data).slice(0, 200)}${C.reset}`);
        }
      } else if (r3.status === 200) {
        console.log(`  ${C.yellow}⚠ 余额数据格式未知${C.reset}`);
        console.log(`  ${C.dim}${JSON.stringify(r3.data).slice(0, 200)}${C.reset}`);
      } else {
        console.log(`  ${C.yellow}⚠ HTTP ${r3.status}${C.reset}`);
      }
    } catch (e) {
      results.push({ name: "Balance", ok: false, error: e.message });
      console.log(`  ${C.red}✗ 错误${C.reset} — ${e.message}`);
    }
  }

  // ─── 总结 ──────────────────────────────────────
  hr();
  if (allPassed) {
    console.log(
      `${C.green}${C.bold}✅ Token 可用！${C.reset} 所有测试通过。`,
    );
  } else {
    console.log(
      `${C.red}${C.bold}❌ Token 测试失败${C.reset} — 请检查 Key 和网络。`,
    );
  }

  return { allPassed, results };
}

// ─── 交互模式 ──────────────────────────────────────
function ask(rl, question) {
  return new Promise((resolve) => {
    rl.question(question, (answer) => resolve(answer.trim()));
  });
}

async function interactiveMode() {
  console.log(`
${C.bold}${C.cyan}╔══════════════════════════════════════════════════╗
║          🔑 Token 连接测试工具                    ║
╚══════════════════════════════════════════════════╝${C.reset}
`);

  const rl = createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  // Provider selection
  console.log(`${C.bold}选择 Provider:${C.reset}`);
  console.log("  1. DeepSeek");
  console.log("  2. OpenAI");
  console.log("  3. OpenAI 兼容 (自定义 URL)");
  const choice = await ask(rl, `\n请输入选项 (1/2/3) [默认: 1]: `);

  let provider, baseUrl, model;
  switch (choice || "1") {
    case "1":
      provider = "deepseek";
      baseUrl = DEEPSEEK_BASE;
      model = "deepseek-chat";
      break;
    case "2":
      provider = "openai";
      baseUrl = OPENAI_BASE;
      break;
    case "3":
      provider = "custom";
      baseUrl = await ask(rl, "  API Base URL: ");
      const customModel = await ask(rl, "  Model 名称 (可选): ");
      if (customModel) model = customModel;
      break;
    default:
      provider = "deepseek";
      baseUrl = DEEPSEEK_BASE;
      model = "deepseek-chat";
  }

  const apiKey = await ask(rl, `\nAPI Key: `);
  rl.close();

  if (!apiKey) {
    console.log(`${C.red}未提供 API Key，退出。${C.reset}`);
    process.exit(1);
  }

  const result = await testToken({ provider, apiKey, baseUrl, model });
  process.exit(result.allPassed ? 0 : 1);
}

// ─── CLI 参数模式 ──────────────────────────────────
function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--provider" && args[i + 1]) opts.provider = args[++i];
    else if (args[i] === "--key" && args[i + 1]) opts.key = args[++i];
    else if (args[i] === "--url" && args[i + 1]) opts.url = args[++i];
    else if (args[i] === "--model" && args[i + 1]) opts.model = args[++i];
    else if (args[i] === "--help" || args[i] === "-h") opts.help = true;
  }
  return opts;
}

// ─── 入口 ──────────────────────────────────────────
const opts = parseArgs();

if (opts.help) {
  console.log(`
${C.bold}用法:${C.reset}
  node scripts/test-token.mjs                          交互模式
  node scripts/test-token.mjs --provider <name> --key <api-key> [--url <base-url>] [--model <model>]

${C.bold}选项:${C.reset}
  --provider   deepseek | openai | custom
  --key        API Key / Token 字符串
  --url        API Base URL (custom provider 时必填)
  --model      测试聊天使用的模型名 (可选)
  --help, -h   显示此帮助

${C.bold}示例:${C.reset}
  node scripts/test-token.mjs --provider deepseek --key sk-xxxxxxxxxxxxxxxx
  node scripts/test-token.mjs --provider openai --key sk-xxxxxxxxxxxxxxxxxxxxxxxx
  node scripts/test-token.mjs --provider custom --key sk-xxx --url https://api.example.com/v1
`);
  process.exit(0);
}

if (opts.provider && opts.key) {
  let baseUrl = opts.url;
  let model = opts.model;

  if (!baseUrl) {
    if (opts.provider === "deepseek") baseUrl = DEEPSEEK_BASE;
    else if (opts.provider === "openai") baseUrl = OPENAI_BASE;
    else {
      console.error(`${C.red}错误: --url 参数必填 (custom provider)${C.reset}`);
      process.exit(1);
    }
  }

  const result = await testToken({
    provider: opts.provider,
    apiKey: opts.key,
    baseUrl,
    model,
  });
  process.exit(result.allPassed ? 0 : 1);
} else {
  await interactiveMode();
}
