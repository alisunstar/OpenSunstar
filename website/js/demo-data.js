(() => {
  window.OpenSunstarDemoData = {
    assets: [
      { key: "mcp", label: "MCP" },
      { key: "skills", label: "Skills" },
      { key: "prompts", label: "Prompts" },
      { key: "commands", label: "Commands" },
      { key: "hooks", label: "Hooks" },
      { key: "permissions", label: "Permissions" },
    ],
    tools: ["Claude Code", "Codex", "Gemini CLI", "OpenCode", "Hermes"],
    statusLabels: {
      synced: "已同步",
      drift: "有漂移",
      missing: "缺失",
      skipped: "跳过",
      blocked: "阻断",
    },
    projects: [
      {
        id: "platform",
        name: "ai-platform",
        path: "~/workspace/ai-platform",
        score: 86,
        risk: "低风险",
        summary:
          "核心 Agent 资产已就位，Codex Prompts 有轻微漂移，适合进入实现阶段。",
        command:
          "os project status --project-path ~/workspace/ai-platform --json",
        next: [
          "同步 Codex Prompts 至 AGENTS.md",
          "复核 Gemini CLI 权限策略",
          "执行 strict 阶段门禁",
        ],
        matrix: {
          "Claude Code": {
            mcp: "synced",
            skills: "synced",
            prompts: "synced",
            commands: "synced",
            hooks: "synced",
            permissions: "synced",
          },
          Codex: {
            mcp: "synced",
            skills: "synced",
            prompts: "drift",
            commands: "synced",
            hooks: "skipped",
            permissions: "synced",
          },
          "Gemini CLI": {
            mcp: "synced",
            skills: "skipped",
            prompts: "synced",
            commands: "synced",
            hooks: "skipped",
            permissions: "drift",
          },
          OpenCode: {
            mcp: "synced",
            skills: "synced",
            prompts: "synced",
            commands: "missing",
            hooks: "skipped",
            permissions: "synced",
          },
          Hermes: {
            mcp: "synced",
            skills: "missing",
            prompts: "synced",
            commands: "synced",
            hooks: "skipped",
            permissions: "synced",
          },
        },
      },
      {
        id: "desktop",
        name: "desktop-client",
        path: "~/workspace/desktop-client",
        score: 72,
        risk: "中风险",
        summary:
          "桌面端配置齐全，但 Hooks 与命令资产未覆盖所有工具，建议先修复漂移。",
        command:
          "os readiness score --project-path ~/workspace/desktop-client --json",
        next: [
          "生成 Commands 适配模板",
          "补齐 OpenCode Hooks 跳过说明",
          "同步 Skills 安全审计结果",
        ],
        matrix: {
          "Claude Code": {
            mcp: "synced",
            skills: "synced",
            prompts: "synced",
            commands: "synced",
            hooks: "drift",
            permissions: "synced",
          },
          Codex: {
            mcp: "synced",
            skills: "synced",
            prompts: "synced",
            commands: "missing",
            hooks: "skipped",
            permissions: "synced",
          },
          "Gemini CLI": {
            mcp: "synced",
            skills: "skipped",
            prompts: "drift",
            commands: "missing",
            hooks: "skipped",
            permissions: "synced",
          },
          OpenCode: {
            mcp: "drift",
            skills: "synced",
            prompts: "synced",
            commands: "missing",
            hooks: "missing",
            permissions: "drift",
          },
          Hermes: {
            mcp: "synced",
            skills: "missing",
            prompts: "synced",
            commands: "synced",
            hooks: "skipped",
            permissions: "synced",
          },
        },
      },
      {
        id: "agent-lab",
        name: "agent-lab",
        path: "~/workspace/agent-lab",
        score: 58,
        risk: "高风险",
        summary:
          "实验仓库缺少统一 Permissions 与 Skills 来源审计，严格阶段门禁会阻断。",
        command:
          "os flow validate --project-path ~/workspace/agent-lab --strict --json",
        next: [
          "初始化 AGENTS.md 与权限基线",
          "对 Skills 执行来源审计",
          "安装项目级 workflow.profile.json",
        ],
        matrix: {
          "Claude Code": {
            mcp: "drift",
            skills: "missing",
            prompts: "synced",
            commands: "drift",
            hooks: "missing",
            permissions: "blocked",
          },
          Codex: {
            mcp: "missing",
            skills: "missing",
            prompts: "drift",
            commands: "missing",
            hooks: "skipped",
            permissions: "blocked",
          },
          "Gemini CLI": {
            mcp: "missing",
            skills: "skipped",
            prompts: "missing",
            commands: "missing",
            hooks: "skipped",
            permissions: "blocked",
          },
          OpenCode: {
            mcp: "drift",
            skills: "missing",
            prompts: "synced",
            commands: "missing",
            hooks: "missing",
            permissions: "blocked",
          },
          Hermes: {
            mcp: "missing",
            skills: "missing",
            prompts: "missing",
            commands: "missing",
            hooks: "skipped",
            permissions: "blocked",
          },
        },
      },
    ],
    terminalCommands: [
      {
        id: "doctor",
        label: "os doctor --json",
        output: {
          status: "ok",
          version: "1.1.4",
          checks: [
            { name: "database", status: "ok" },
            { name: "keychain", status: "ok" },
            { name: "local_proxy", status: "ok", port: 15721 },
          ],
          hint: "环境可用于 GUI 与 CLI 双模态治理。",
        },
      },
      {
        id: "drift",
        label: "os drift check --json",
        output: {
          status: "found",
          findings: [
            {
              tool: "Codex",
              asset: "Prompts",
              severity: "medium",
              action: "sync_from_project_profile",
            },
            {
              tool: "OpenCode",
              asset: "Commands",
              severity: "low",
              action: "generate_template",
            },
          ],
          exitCode: 1,
        },
      },
      {
        id: "flow",
        label: "os flow validate --strict --json",
        output: {
          status: "blocked",
          stage: "implementation",
          blockers: [
            "missing .specs/design-contract.md",
            "missing workflow.profile.json",
          ],
          exitCode: 2,
        },
      },
      {
        id: "sync",
        label: "os sync status --json",
        output: {
          status: "ready",
          backend: "webdav",
          encrypted: true,
          snapshots: 10,
          lastSnapshot: "2026-07-10T14:38:12Z",
        },
      },
    ],
    syncBackends: [
      {
        id: "webdav",
        label: "WebDAV",
        endpoint: "https://dav.example.com/opensunstar",
        latency: "120ms",
      },
      {
        id: "s3",
        label: "S3",
        endpoint: "s3://opensunstar-sync/profiles",
        latency: "95ms",
      },
      {
        id: "gist",
        label: "Gist",
        endpoint: "gist:opensunstar-agent-assets",
        latency: "180ms",
      },
    ],
    securityStates: {
      manifest: {
        label: "加密 manifest",
        output: {
          manifestVersion: 2,
          cipher: "AES-256-GCM",
          kdf: "PBKDF2-HMAC-SHA256",
          kdfSalt: "7df1c24e8a0b4d19b65c9e332b1a0f5e",
          snapshotDigest: "sha256:8f3d9e4c0c5b7e2c...",
          assets: ["mcp", "skills", "prompts", "permissions"],
        },
      },
      wrongKey: {
        label: "错误主密钥",
        output: {
          error: true,
          code: "SYNC_DECRYPT_FAILED",
          message: "无法使用当前主密钥解密快照。",
          hint: "检查主密钥或恢复其他设备导出的密钥。",
        },
      },
      corrupted: {
        label: "损坏 manifest",
        output: {
          error: true,
          code: "SYNC_MANIFEST_INVALID",
          message: "manifest 摘要与快照内容不匹配。",
          hint: "拒绝导入并保留本地最近可用备份。",
        },
      },
    },
  };
})();
