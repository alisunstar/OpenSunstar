# ModelScope API Skill

魔塔社区（ModelScope）完整 API 封装 Skill，支持模型库、数据集、技能中心、MCP 广场、创空间（Studio）的查询、搜索、部署与管理。

## What It Does

本 Skill 封装了魔塔社区（ModelScope）的全部可用 API，覆盖 5 大能力模块：

| 能力 | 说明 | 数据规模 |
|------|------|---------|
| 技能中心（Skills） | 搜索/浏览/下载 AI 技能 | 80,505+ 技能 |
| MCP 广场 | 搜索/获取 MCP 服务器配置 | 9,477+ 服务器 |
| 模型库（Models） | 查询/下载 AI 模型 | 194,001+ 模型 |
| 数据集（Datasets） | 查询/下载数据集 | — |
| 创空间（Studios） | 创建/部署/管理云端应用 | 支持 4 种 SDK 类型 |

## How to Use

### 前置条件

需要一个 ModelScope API Token：
1. 访问 https://www.modelscope.cn/my/overview 注册登录
2. 点右上角头像 → 「访问令牌」(Access Token) → 新建令牌
3. 复制生成的 Token（格式：`ms-xxxxxx`）

### 在 WorkBuddy 中使用

当用户提到以下关键词时，Skill 自动触发：
- 「魔塔」「ModelScope」「查模型」「搜MCP」「技能中心」「魔塔下载」
- 「创空间」「Studio 部署」「魔搭部署」

### 手动调用 API

**搜索技能：**
```bash
curl -s "https://www.modelscope.cn/openapi/v1/skills?page_number=1&page_size=10&search=股票"
```

**搜索 MCP 服务器：**
```bash
curl -s -X PUT "https://www.modelscope.cn/openapi/v1/mcp/servers" \
  -H "Content-Type: application/json" \
  -d '{"page_number": 1, "page_size": 10, "search": ""}'
```

**获取 MCP 服务器配置（可直接用于 mcp.json）：**
```bash
curl -s "https://www.modelscope.cn/openapi/v1/mcp/servers/@modelcontextprotocol/fetch" \
  -H "Authorization: Bearer ms-xxxxxx"
```

### Python SDK

```python
from modelscope.hub.api import HubApi
from modelscope.hub.mcp_api import MCPApi

api = HubApi(token="ms-xxxxxx")
mcp = MCPApi(token="ms-xxxxxx")

# 下载技能
skill_dir = api.download_skill(skill_id="@Alipay/alipay-payment-integration")

# 搜索 MCP 服务器
result = mcp.list_mcp_servers(search="高德")
```

## 内置辅助脚本

`scripts/modelscope_helper.py` 提供 CLI 工具：
- `python modelscope_helper.py setup --token ms-xxxxxx` — 配置 Token
- `python modelscope_helper.py skills --search 股票` — 搜索技能
- `python modelscope_helper.py skill-detail @Alipay/alipay-payment-integration` — 技能详情
- `python modelscope_helper.py mcp --search 高德` — 搜索 MCP 服务器
- `python modelscope_helper.py models` — 列出模型（需 Token）

## API 兼容性

| 路径前缀 | Key 风格 | 数据字段 |
|----------|----------|----------|
| `/api/v1/`（旧） | PascalCase | `Data` |
| `/openapi/v1/`（新） | snake_case | `data` |

## Requirements

- Python 3.8+（仅 CLI 脚本需要）
- ModelScope API Token（公开接口无需 Token，私有接口需要）
- `pip install modelscope`（SDK 使用需要）

## Troubleshooting

| 问题 | 解决方案 |
|------|---------|
| `/sse` 端点 404 | FastMCP 需显式 `http_app(transport="sse")` |
| docker 模式 push 后不更新 | 改用 gradio 模式或手动 `deployStudio` |
| Token 无效 | 重新访问 https://modelscope.cn/my/myaccesstoken 获取 |
| Git push 失败 | 确认分支为 `master`，Token 有效 |

## License

Apache-2.0
