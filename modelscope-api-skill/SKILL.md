---
name: modelscope-api
slug: modelscope-api
version: 1.0.4
author: Chenghd511
license: MIT-0
description: 魔塔社区（ModelScope）API 封装。支持模型/数据集/技能中心/MCP广场/创空间（Studio）的查询、搜索、下载。所有变更操作（部署、删除、安装等）需要用户明确确认。触发词：「魔塔」「mota」「使用mota」「ModelScope」「查模型」「搜MCP」「技能中心」「魔塔下载」。
triggers:
  - 魔塔
  - mota
  - 使用mota
  - ModelScope
  - 查模型
  - 搜MCP
  - 技能中心
  - 魔塔下载
  - modelscope
---

# 魔塔社区（ModelScope）API 完整手册

## 概述

本 Skill 封装魔塔社区全部可用 API，包括：模型库、数据集、技能中心（80K+ 技能）、MCP 广场（9.4K+ 服务器）。支持Token自动引导注册。

**两种响应格式注意**：
- `/api/v1/*`（旧）：PascalCase → `{"Code": 200, "Data": {...}}`
- `/openapi/v1/*`（新）：snake_case → `{"success": true, "data": {...}}`

---

## 第一步：Token 设置（首次使用必读）

> ⚠️ **安全警告**：此技能包含只读操作和变更操作。**部署、删除、安装等变更操作需要你明确确认**。请使用最小必要权限的 Token。

### 权限建议

| 操作类型 | 所需权限 | 说明 |
|---------|---------|------|
| **只读操作**（搜索、查询） | 读取权限 | 无需 Token，公开可用 |
| **下载操作**（模型、数据集） | 下载权限 | 需要 Token |
| **变更操作**（创建、部署、删除） | 写入权限 | **必须明确用户确认** |

> 💡 **建议**：优先使用只读 Token，仅在需要下载/部署时才提供写入权限。

### Token 获取与使用原则

> 🔒 **核心原则**：Token 只在当前会话中使用，**不写入任何文件，不保存到记忆**。

**Token 优先级**（从上到下）：
1. **环境变量** `MODELSCOPE_API_TOKEN`（最安全，推荐）
2. **直接提供**（没有环境变量时，AI 会直接询问你）

**没有 Token 时**：直接告诉 AI「我的魔塔 Token 是 xxx」，AI 会在本次会话中使用它，执行完即丢弃。

### 情况 A：已有 Token

**方式一（推荐）**：设置环境变量
```bash
export MODELSCOPE_API_TOKEN="ms-xxxxxx"
```

**方式二**：直接告诉 AI（AI 不会记录，仅当前会话使用）

**没有 Token → 引导注册**：

1. 打开注册页面：https://www.modelscope.cn/my/overview
2. 用手机号/支付宝/钉钉完成注册登录
3. 登录后，点右上角头像 → **「访问令牌」(Access Token)**
4. 点「新建令牌」，填写名称（如 `workbuddy`）
5. ⚠️ **重要**：权限选择「**只读**」或「**下载**」，除非你需要部署功能，否则不要选「全部」
6. 复制生成的 Token（格式：`ms-xxxxxx`），**只显示一次，务必保存**

> ⚠️ Token 即密码，不要提交到 GitHub。

获取 Token 后，直接告诉 AI 或设置环境变量，AI 在当前会话中使用后即丢弃，不记录。

---

## 能力一：技能中心（Skills）

> 总技能数：**80,505+**

### 1.1 搜索/列出技能

```bash
curl -s "https://www.modelscope.cn/openapi/v1/skills?page_number=1&page_size=10&search=股票" | python -m json.tool
```

**Query 参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| `page_number` | int | 页码，默认 1 |
| `page_size` | int | 每页数量，默认 10 |
| `search` | string | 搜索关键词（匹配 display_name） |

**无需 Token** 即可访问。

**响应示例**：
```json
{
  "success": true,
  "data": {
    "total": 80505,
    "skills": [
      {
        "id": "@Alipay/alipay-payment-integration",
        "display_name": "支付宝支付集成skill",
        "description": "...",
        "category": "developer-tools",
        "view_count": 21881,
        "downloads": 2069
      }
    ]
  }
}
```

### 1.2 获取技能详情（含安装命令）

```bash
curl -s "https://www.modelscope.cn/openapi/v1/skills/@Alipay/alipay-payment-integration" | python -m json.tool
```

**返回额外字段** `install_command`（3 种安装方式）：
```json
{
  "install_command": [
    "npx skills add https://modelscope.cn/skills/@Alipay/alipay-payment-integration",
    "curl -fsSL https://modelscope.cn/skills/install.sh | bash -s -- @Alipay/alipay-payment-integration",
    "pip install --upgrade modelscope && modelscope skills add @Alipay/alipay-payment-integration"
  ]
}
```

### 1.3 下载技能（需 Token）

> 🔒 **下载操作**：需要 Token 权限，但不会修改账户数据。

```python
from modelscope.hub.api import HubApi

api = HubApi(token="ms-xxxxxx")
skill_dir = api.download_skill(skill_id="@Alipay/alipay-payment-integration")
print(f"技能已下载到: {skill_dir}")
```

> ⚠️ **安装到本地前，请确认该技能的来源和用途**。
```

底层 API（一般不直接调用）：
```
GET /api/v1/skills/{path}/{name}/archive/zip/master
Header: Cookie: m_session_id={token}
```

### 1.4 技能分类列表

| 分类 ID | 说明 |
|----------|------|
| `developer-tools` | 开发工具 |
| `ai-media` | AI 媒体 |
| `skill-management` | 技能管理 |
| `cloud-devops` | 云 DevOps |
| `frontend-development` | 前端开发 |
| `code-quality-testing` | 代码质量测试 |
| `marketing-seo` | 营销 SEO |
| `ai-automation` | AI 自动化 |
| `doc-processing` | 文档处理 |

---

## 能力二：MCP 广场

> 总 MCP 服务器数：**9,477+**

### 2.1 搜索/列出 MCP 服务器

```bash
curl -s -X PUT "https://www.modelscope.cn/openapi/v1/mcp/servers" \
  -H "Content-Type: application/json" \
  -d '{"filter": {"category": "developer-tools"}, "page_number": 1, "page_size": 10, "search": ""}' | python -m json.tool
```

**Body 参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| `filter.category` | string | 分类过滤（见下方分类表） |
| `filter.tag` | string | 标签过滤 |
| `filter.is_hosted` | bool | 是否只查托管服务器 |
| `page_number` | int | 页码，默认 1 |
| `page_size` | int | 每页数量，最大 100，默认 20 |
| `search` | string | 搜索（匹配中文名/英文名/作者） |

> `filter` 中的多个条件取**交集**；`search` 与 `filter` 可组合使用。

**无需 Token** 即可访问列表。

**响应示例**：
```json
{
  "success": true,
  "data": {
    "total_count": 9477,
    "mcp_server_list": [
      {
        "id": "@modelcontextprotocol/fetch",
        "name": "Fetch网页内容抓取",
        "chinese_name": "Fetch网页内容抓取",
        "description": "该服务器使大型语言模型能够检索和处理网页内容...",
        "categories": ["browser-automation"],
        "view_count": 457094
      }
    ]
  }
}
```

### 2.2 获取 MCP 服务器详情（含配置）

```bash
curl -s "https://www.modelscope.cn/openapi/v1/mcp/servers/@modelcontextprotocol/fetch?get_operational_url=true" | python -m json.tool
```

**返回关键字段**：

| 字段 | 说明 |
|------|------|
| `server_config` | MCP 客户端配置（command + args），直接用于 `mcp.json` |
| `readme` | 服务器说明文档（Markdown） |
| `is_hosted` | 是否已触发托管服务 |
| `is_verified` | 是否官方认证 |
| `env_schema` | 环境变量 JSON Schema（如需配置 API Key） |
| `source_url` | 源码地址（通常是 GitHub） |
| `github_stars` | GitHub 星标数 |

**`server_config` 示例**（直接复制到 MCP 客户端配置）：
```json
{
  "mcpServers": {
    "fetch": {
      "command": "uvx",
      "args": ["mcp-server-fetch"]
    }
  }
}
```

### 2.3 获取用户运行中的 MCP 服务器（需 Token）

```bash
curl -s "https://www.modelscope.cn/openapi/v1/mcp/servers/operational" \
  -H "Authorization: Bearer ms-xxxxxx" | python -m json.tool
```

> ⚠️ 此端点**必须传 Token**，否则返回 401。

### 2.4 MCP 分类列表（27 个）

| 分类 ID | 说明 | 分类 ID | 说明 |
|---------|------|---------|------|
| `ai-gc` | AI 生成 | `browser-automation` | 浏览器自动化 |
| `communication` | 通讯 | `content-management-systems` | 内容管理 |
| `databases` | 数据库 | `data-platforms` | 数据平台 |
| `developer-tools` | 开发工具 | `entertainment-and-media` | 娱乐媒体 |
| `finance` | 金融 | `file-systems` | 文件系统 |
| `image-and-video-processing` | 图像视频 | `knowledge-and-memory` | 知识记忆 |
| `location-services` | 位置服务 | `monitoring` | 监控 |
| `note-taking` | 笔记 | `other` | 其他 |
| `os-automation` | 系统自动化 | `research-and-data` | 研究数据 |
| `search` | 搜索 | `travel-and-transportation` | 出行交通 |
| `version-control` | 版本控制 | `cloud-platforms` | 云平台 |
| `art-and-culture` | 艺术文化 | `calendar-management` | 日历管理 |

---

## 能力三：模型库（Models）

> 总模型数：**194,001+**

### 3.1 列出模型

```bash
curl -s -X PUT "https://www.modelscope.cn/api/v1/models" \
  -H "Authorization: Bearer ms-xxxxxx" \
  -H "Content-Type: application/json" \
  -d '{"page_number": 1, "page_size": 10, "search": "llama"}' | python -m json.tool
```

> ⚠️ 模型 API 使用**旧格式**（`/api/v1/`，PascalCase）

**响应格式**：
```json
{
  "Code": 200,
  "Success": true,
  "Data": {
    "TotalCount": 194001,
    "Models": [...]
  }
}
```

### 3.2 获取模型详情

```bash
curl -s "https://www.modelscope.cn/api/v1/models/{owner}/{name}" \
  -H "Authorization: Bearer ms-xxxxxx" | python -m json.tool
```

### 3.3 下载模型

```bash
curl -s "https://www.modelscope.cn/api/v1/models/{owner}/{name}/archive/zip/{rev}" \
  -H "Authorization: Bearer ms-xxxxxx" -o model.zip
```

或用 SDK（推荐）：
```python
from modelscope.hub.api import HubApi
api = HubApi(token="ms-xxxxxx")
model_dir = api.download_model(model_id="qwen/Qwen-7B")
```

### 3.4 常用模型端点汇总

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/models` | PUT | 列出模型（分页/搜索） |
| `/api/v1/models/{owner}/{name}` | GET | 获取模型详情 |
| `/api/v1/models/{owner}/{name}/archive/zip/{rev}` | GET | 下载模型（ZIP） |
| `/api/v1/models/{owner}/{name}/repo/tag` | POST | 创建标签 |
| `/api/v1/models/{owner}/{name}/repo/files` | GET | 获取模型文件列表 |

---

## 能力四：数据集（Datasets）

### 4.1 列出数据集

```bash
curl -s -X PUT "https://www.modelscope.cn/api/v1/datasets" \
  -H "Authorization: Bearer ms-xxxxxx" \
  -H "Content-Type: application/json" \
  -d '{"page_number": 1, "page_size": 10, "search": "finance"}' | python -m json.tool
```

### 4.2 获取数据集详情

```bash
curl -s "https://www.modelscope.cn/api/v1/datasets/{namespace}/{name}" \
  -H "Authorization: Bearer ms-xxxxxx" | python -m json.tool
```

### 4.3 获取数据集文件树

```bash
curl -s "https://www.modelscope.cn/api/v1/datasets/{namespace}/{name}/repo/tree" \
  -H "Authorization: Bearer ms-xxxxxx" | python -m json.tool
```

### 4.4 常用数据集端点汇总

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/datasets` | PUT | 列出数据集 |
| `/api/v1/datasets/{ns}/{name}` | GET | 获取数据集详情 |
| `/api/v1/datasets/{ns}/{name}/repo/tree` | GET | 获取文件树 |
| `/api/v1/datasets/{ns}/{name}/archive/zip/{rev}` | GET | 下载数据集（ZIP） |

---

## 能力五：创空间（Studios）

创空间（Studio）提供 MCP 工具和 HTTP API 两种操作方式。

> **前置条件**：需要 `MODELSCOPE_API_TOKEN`，获取方式见「第一步：Token 设置」。

> ⚠️ **【重要】变更操作需要用户明确确认**：以下操作会创建、修改或删除资源，执行前必须获得用户确认。

### 5.1 MCP 工具（推荐）

配置 ModelScope MCP 服务后，可直接调用以下工具：

| 操作类型 | 操作 | 工具 | 说明 |
|---------|-----|-----|------|
| 🔒 只读 | 获取用户信息 | `getCurrentUser` | 获取当前用户名 |
| ⚠️ **变更** | 创建创空间 | `createStudio` | **需确认** - 在账户中创建新资源 |
| 🔒 只读 | 获取创空间详情 | `getStudio` | 查看现有资源 |
| ⚠️ **变更** | 更新设置 | `updateStudioSettings` | **需确认** - 修改配置 |
| ⚠️ **变更** | 部署（启动/重启） | `deployStudio` | **需确认** - 触发构建和部署 |
| ⚠️ **变更** | 停止 | `stopStudio` | **需确认** - 停止服务 |
| 🔒 只读 | 获取日志 | `getStudioLogs` | 查看运行日志 |
| 🔒 只读 | 环境变量列表 | `listStudioSecrets` | 查看变量（不含值） |
| ⚠️ **变更** | 添加环境变量 | `addStudioSecret` | **需确认** - 添加新变量 |
| ⚠️ **变更** | 更新环境变量 | `updateStudioSecret` | **需确认** - 修改变量 |
| ⚠️ **变更** | 删除环境变量 | `deleteStudioSecret` | **需确认** - 删除变量 |

### 5.2 HTTP API（MCP 不可用时的备选）

基础地址：`https://modelscope.cn/openapi/v1`，所有请求需携带 `Authorization: Bearer ${MODELSCOPE_API_TOKEN}`。

#### MCP 服务管理（配置 MCP 前使用）

| 操作类型 | 操作 | 方法 | 端点 |
|---------|-----|------|------|
| 🔒 只读 | 查询 MCP 服务详情 | GET | `/mcp/servers/{id}?get_operational_url=true` |
| ⚠️ **变更** | 部署 MCP 服务 | POST | `/mcp/servers/{id}/deploy` **（需确认）** |

#### 创空间操作

| 操作类型 | 操作 | 方法 | 端点 | Body |
|---------|-----|------|------|------|
| 🔒 只读 | 获取当前用户 | GET | `/users/me` | — |
| ⚠️ **变更** | 创建创空间 | POST | `/studios` | `{owner, repo_name, ...}` **（需确认）** |
| 🔒 只读 | 获取创空间详情 | GET | `/studios/{owner}/{repo_name}` | — |
| ⚠️ **变更** | 更新创空间设置 | PATCH | `/studios/{owner}/{repo_name}/settings` | **（需确认）** |
| ⚠️ **变更** | 部署创空间 | POST | `/studios/{owner}/{repo_name}/deploy` | **（需确认）** |
| ⚠️ **变更** | 停止创空间 | POST | `/studios/{owner}/{repo_name}/stop` | **（需确认）** |
| 🔒 只读 | 获取日志 | GET | `/studios/{owner}/{repo_name}/logs/{log_type}` | — |
| 🔒 只读 | 获取环境变量列表 | GET | `/studios/{owner}/{repo_name}/secrets` | — |
| ⚠️ **变更** | 添加环境变量 | POST | `/studios/{owner}/{repo_name}/secrets` | **（需确认）** |
| ⚠️ **变更** | 更新环境变量 | PUT | `/studios/{owner}/{repo_name}/secrets` | **（需确认）** |
| ⚠️ **变更** | 删除环境变量 | DELETE | `/studios/{owner}/{repo_name}/secrets` | **（需确认）** |

完整 OpenAPI 文档：https://modelscope.cn/.well-known/openapi.json

### 5.3 实战经验（来自 fortune-mcp 部署验证）

**sdk_type 选择关键差异**：

| sdk_type | push 后行为 | 适用场景 |
|----------|-----------|---------|
| `docker` | **不自动重建**，需手动调用 `deployStudio` | FastAPI、自定义服务 |
| `gradio` | 自动识别 `app.py` 中的 `demo` 变量并启动 | Gradio 应用 |
| `streamlit` | 自动启动 Streamlit | Streamlit 应用 |
| `static` | 直接托管静态文件 | 纯前端 |

> ⚠️ `docker` 模式 push 后页面不变是常见问题，改用 `gradio` 或手动调 `deployStudio` 可解决。

**Git 推送要点**：
- 默认分支 **`master`**（不是 `main`），禁止 force push
- 推送地址：`https://oauth2:${TOKEN}@www.modelscope.cn/studios/${owner}/${repo_name}.git`
- 首次推送前需 `fetch` + `merge --allow-unrelated-histories`

**Gradio 模式要求**：
- 入口文件：`app.py`（固定）
- 必须有全局 `demo` 变量：`demo = gr.Blocks(title="...")`
- `requirements.txt` 需含 `gradio>=4.0.0`
- 平台自动调用 `demo.launch(server_name="0.0.0.0", server_port=7860)`

**MCP 服务在创空间中运行**：
- `FastMCP.http_app(transport="sse")` 必须**显式指定 transport**（默认是 streamable-http，注册 `/mcp` 而非 `/sse`）
- MCP SSE 需用独立线程运行，不能阻塞 Gradio 主线程
- 端口必须 `0.0.0.0`，禁止 `8080`（平台占用）

> 💡 完整的部署实战经验和踩坑记录见 `modelscope-studio` Skill。

---

## Python SDK 快速使用

### 安装

```bash
pip install modelscope
```

### 技能操作

```python
from modelscope.hub.api import HubApi
from modelscope.hub.mcp_api import MCPApi

# 初始化（Token 可选，无 Token 只能访问公开数据）
api = HubApi(token="ms-xxxxxx")
mcp = MCPApi(token="ms-xxxxxx")

# 登录（保存 Cookie 到本地，30 天有效）
git_token, cookies = api.login()

# 下载技能
skill_dir = api.download_skill(skill_id="@Alipay/alipay-payment-integration")
print(f"技能已下载到: {skill_dir}")

# 列出 MCP 服务器（无需 Token）
result = mcp.list_mcp_servers(
    filter={"category": "developer-tools"},
    total_count=20,
    search=""
)
print(f"总数: {result['total_count']}")

# 搜索 MCP 服务器
result = mcp.list_mcp_servers(search="高德")

# 获取 MCP 详情
detail = mcp.get_mcp_server(server_id="@amap/amap-maps")
print(f"配置: {detail['server_config']}")

# 获取运行中的 MCP（需 Token）
running = mcp.list_operational_mcp_servers()
```

### CLI 命令

```bash
# 登录
modelscope login --token ms-xxxxxx

# 安装技能（并发下载，默认 8 线程）
modelscope skills add @Alipay/alipay-payment-integration

# 安装多个
modelscope skills add @steipete/github @steipete/weather --max-workers 8

# 指定安装目录
modelscope skills add @Alipay/alipay-payment-integration --local_dir ./my-skills
```

---

## 热门资源 Top 10

### 技能 Top 10（按下载量）

| 排名 | 名称 | ID | 下载量 |
|------|------|-----|--------|
| 1 | self-improvement | @pskoett/self-improving-agent | 2,377 |
| 2 | 支付宝支付集成 | @Alipay/alipay-payment-integration | 2,069 |
| 3 | find-skills | @vercel-labs/find-skills | 2,000 |
| 4 | frontend-design | @anthropics/frontend-design | 2,304 |
| 5 | web-search | @inference-sh/web-search | 1,360 |

### MCP 服务器 Top 10（按浏览量）

| 排名 | 名称 | ID | 浏览量 |
|------|------|-----|--------|
| 1 | Fetch 网页抓取 | @modelcontextprotocol/fetch | 457K |
| 2 | 高德地图 | @amap/amap-maps | 290K |
| 3 | 12306 车票 | @Joooook/12306-mcp | 182K |
| 4 | 必应搜索中文 | slcatwujian/bing-cn-mcp-server | 154K |
| 5 | 可视化图表 | antvis/mcp-server-chart | 107K |

---

## 响应格式速查

| 路径前缀 | Key 风格 | 成功标志 | 数据字段 |
|----------|----------|----------|----------|
| `/api/v1/` | PascalCase | `Success: true` | `Data` |
| `/openapi/v1/` | snake_case | `success: true` | `data` |

**代码示例（通用解析）**：
```python
import requests

def ms_request(url, token=None):
    headers = {"Authorization": f"Bearer {token}"} if token else {}
    r = requests.get(url, headers=headers)
    d = r.json()
    # 兼容两种格式
    if "Data" in d:
        return d["Data"]          # /api/v1/ 格式
    elif "data" in d:
        return d["data"]          # /openapi/v1/ 格式
    else:
        raise ValueError(f"未知响应格式: {d}")

# 用法
skills = ms_request("https://www.modelscope.cn/openapi/v1/skills?page_size=5")
print(skills["total"])  # → 80505
```
