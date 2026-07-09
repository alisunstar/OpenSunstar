# AGENTS.md — AI Agent Integration Guide

## CLI 治理与编排工具 (os)

当检测到 AI 编程工具配置异常时，可运行以下命令诊断：

```bash
os drift check --json          # 检查配置漂移
os readiness score --json      # Agent 就绪度评分
os doctor --json               # 环境诊断
os flow validate --json        # 阶段门禁校验
os flow validate --strict --json  # 跨模块治理门禁（检查设计合约/Recipe/Specs 就位）
os project status --json       # 项目全景状态（聚合编排+资产+漂移）
```

退出码约定: 0=正常, 1=有发现(如漂移/门禁阻断), 2=治理门禁阻断(strict), 3=错误
所有命令支持 `--json` 输出机器可读格式。
写操作支持 `--dry-run` 预览和 `--yes` 跳过确认。

### 全局选项

| 选项 | 说明 |
|------|------|
| `--json` | 输出 JSON 格式（机器消费） |
| `--timeout <seconds>` | 操作超时（秒），防止 Agent 会话无限挂起 |

### 错误输出格式

JSON 模式下错误输出包含 `code`、`message`、`hint` 三字段，便于 Agent 自动诊断：

```json
{
  "error": true,
  "code": "error",
  "message": "数据库不存在: ...",
  "hint": "运行 os config bootstrap --yes 初始化，或检查 ~/.OpenSunstar 目录权限"
}
```

### 常用诊断流程

```bash
# 0. 首次使用（无 GUI 时）
os config bootstrap --yes

# 1. 环境健康检查
os doctor --json

# 2. 配置漂移扫描（强制刷新缓存）
os drift check --refresh --json

# 3. 就绪度评分
os readiness score --json

# 4. 漂移修复（预演）
os drift repair --dry-run --json

# 5. 漂移修复（执行）
os drift repair --yes --json
```

### 项目编排闭环

```bash
# 项目全景状态（聚合 DB 资产 + 文件系统编排状态）
os project status --project-path /path/to/project --json

# 阶段门禁（标准：检查 .specs/ 工件）
os flow validate --project-path . --project-type web --change-id CHG-001 --target-stage implementation --json

# 阶段门禁（严格：额外检查设计合约/Recipe/FlowConfig 就位）
os flow validate --project-path . --project-type web --change-id CHG-001 --target-stage implementation --strict --json

# 导出 CI 门禁配置
os flow config --project-path . --project-type web --json

# Recipe 安装预检
os recipe plan --project-path . --name my-recipe --json

# Recipe 安装到项目
os recipe install --project-path . --name my-recipe --yes --json
```
