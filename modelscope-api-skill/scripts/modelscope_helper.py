#!/usr/bin/env python3
"""
魔塔社区（ModelScope）API 辅助工具
支持：技能中心 / MCP广场 / 模型库 / 数据集 的查询与下载
注意：Token 只从环境变量 MODELSCOPE_API_TOKEN 读取，不存储到任何文件。
"""

import os
import sys
import json
import requests

# ── Token 管理（仅环境变量，不写文件）────────────────────────────

def load_token() -> str | None:
    """仅从环境变量读取 Token，不读写任何文件。"""
    token = os.environ.get("MODELSCOPE_API_TOKEN")
    return token.strip() if token else None


def require_token() -> str:
    """获取 Token，没有则引导注册并退出。"""
    token = load_token()
    if not token:
        guide_registration()
    return token


def guide_registration():
    """打印 Token 注册引导。"""
    print("""
╭─────────────────────────────────────────────────────────────╮
│  需要魔塔社区访问令牌（Token）                              │
╰─────────────────────────────────────────────────────────────╯

请按以下步骤获取 Token：

  1. 打开 https://www.modelscope.cn/my/overview
  2. 用手机号 / 支付宝 / 钉钉 完成注册登录
  3. 登录后点右上角头像 →「访问令牌 (Access Token)」
  4. 点「新建令牌」，名称填 workbuddy，权限选「只读」或「下载」
  5. 复制生成的 Token（格式：ms-xxxxxxxx）
     ⚠️  Token 只显示一次，务必保存！

获取 Token 后，请通过 AI 助手直接提供，或设置环境变量：
  export MODELSCOPE_API_TOKEN="ms-xxxxxxxx"

（Token 仅在当前会话中使用，不保存到任何文件）
""")
    sys.exit(1)


# ── 通用请求 ────────────────────────────────────────────────

def ms_get(url: str, token: str | None = None, **kwargs) -> dict:
    """发起 GET 请求，自动兼容两种响应格式。"""
    headers = {}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    r = requests.get(url, headers=headers, timeout=15, **kwargs)
    r.raise_for_status()
    d = r.json()
    # 兼容 /api/v1/（PascalCase）和 /openapi/v1/（snake_case）
    if "Data" in d:
        return d["Data"]
    if "data" in d:
        return d["data"]
    return d


def ms_put(url: str, body: dict, token: str | None = None) -> dict:
    """发起 PUT 请求，自动兼容两种响应格式。"""
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    r = requests.put(url, headers=headers, json=body, timeout=15)
    r.raise_for_status()
    d = r.json()
    if "Data" in d:
        return d["Data"]
    if "data" in d:
        return d["data"]
    return d


# ── 技能中心 ────────────────────────────────────────────────

BASE = "https://www.modelscope.cn"


def list_skills(search: str = "", page: int = 1, page_size: int = 10) -> dict:
    """列出技能（无需 Token）。"""
    url = f"{BASE}/openapi/v1/skills?page_number={page}&page_size={page_size}&search={search}"
    return ms_get(url)


def get_skill(skill_id: str) -> dict:
    """获取技能详情（含 install_command，无需 Token）。"""
    url = f"{BASE}/openapi/v1/skills/{skill_id}"
    return ms_get(url)


# ── MCP 广场 ────────────────────────────────────────────────

def list_mcp_servers(
    search: str = "",
    category: str = "",
    tag: str = "",
    page: int = 1,
    page_size: int = 20,
    token: str | None = None,
) -> dict:
    """列出 MCP 服务器（无需 Token）。"""
    body: dict = {
        "page_number": page,
        "page_size": page_size,
        "search": search,
    }
    if category:
        body["filter"] = {"category": category}
    elif tag:
        body["filter"] = {"tag": tag}
    return ms_put(f"{BASE}/openapi/v1/mcp/servers", body, token=token)


def get_mcp_server(server_id: str, token: str | None = None) -> dict:
    """获取 MCP 服务器详情（含 server_config）。"""
    params = "?get_operational_url=true" if token else ""
    url = f"{BASE}/openapi/v1/mcp/servers/{server_id}{params}"
    return ms_get(url, token=token)


def list_operational_mcp(token: str) -> dict:
    """获取用户运行中的 MCP 服务器（必须 Token）。"""
    url = f"{BASE}/openapi/v1/mcp/servers/operational"
    return ms_get(url, token=token)


# ── 模型库 ──────────────────────────────────────────────────

def list_models(search: str = "", page: int = 1, page_size: int = 10,
               token: str | None = None) -> dict:
    """列出模型（需 Token 查看完整信息）。"""
    body = {"page_number": page, "page_size": page_size, "search": search}
    return ms_put(f"{BASE}/api/v1/models", body, token=token)


def get_model(owner: str, name: str, token: str | None = None) -> dict:
    """获取模型详情。"""
    url = f"{BASE}/api/v1/models/{owner}/{name}"
    return ms_get(url, token=token)


# ── 数据集 ──────────────────────────────────────────────────

def list_datasets(search: str = "", page: int = 1, page_size: int = 10,
                 token: str | None = None) -> dict:
    """列出数据集。"""
    body = {"page_number": page, "page_size": page_size, "search": search}
    return ms_put(f"{BASE}/api/v1/datasets", body, token=token)


def get_dataset(namespace: str, name: str, token: str | None = None) -> dict:
    """获取数据集详情。"""
    url = f"{BASE}/api/v1/datasets/{namespace}/{name}"
    return ms_get(url, token=token)


# ── CLI 入口 ────────────────────────────────────────────────

def cmd_verify(args: list[str]):
    """验证 Token（从环境变量读取，不存储）。"""
    token = load_token()
    if not token:
        print("❌ 未找到 MODELSCOPE_API_TOKEN 环境变量")
        guide_registration()
    try:
        result = list_models(search="test", page=1, page_size=1, token=token)
        print(f"✅ Token 验证成功！模型总数：{result.get('TotalCount', '?')}")
    except Exception as e:
        print(f"⚠️  Token 验证失败：{e}")


def cmd_skills(args: list[str]):
    """搜索技能。用法：skills [关键词] [--page N] [--size N]"""
    search = args[0] if args and not args[0].startswith("--") else ""
    page = 1
    size = 10
    if "--page" in args:
        page = int(args[args.index("--page") + 1])
    if "--size" in args:
        size = int(args[args.index("--size") + 1])
    result = list_skills(search=search, page=page, page_size=size)
    print(f"技能总数：{result.get('total', result.get('TotalCount', '?'))}")
    for s in result.get("skills", []):
        print(f"  - {s['id']:40s} {s.get('display_name', '')}")


def cmd_skill_detail(args: list[str]):
    """查看技能详情。用法：skill-detail <skill_id>"""
    if not args:
        print("用法：skill-detail <skill_id>"); return
    detail = get_skill(args[0])
    print(json.dumps(detail, ensure_ascii=False, indent=2))


def cmd_mcp(args: list[str]):
    """搜索 MCP 服务器。用法：mcp [关键词] [--category xxx]"""
    search = ""
    category = ""
    for i, a in enumerate(args):
        if a == "--category":
            category = args[i + 1]
        elif not a.startswith("--"):
            search = a
    result = list_mcp_servers(search=search, category=category)
    total = result.get("total_count", result.get("TotalCount", "?"))
    print(f"MCP 服务器总数：{total}")
    for s in result.get("mcp_server_list", []):
        name = s.get("chinese_name") or s.get("name", "")
        print(f"  - {s['id']:40s} {name}")


def cmd_mcp_detail(args: list[str]):
    """查看 MCP 详情（含配置）。用法：mcp-detail <server_id>"""
    if not args:
        print("用法：mcp-detail <server_id>"); return
    token = load_token()
    detail = get_mcp_server(args[0], token=token)
    print(json.dumps(detail, ensure_ascii=False, indent=2))


def cmd_models(args: list[str]):
    """搜索模型。用法：models [关键词]"""
    search = args[0] if args else ""
    token = require_token()
    result = list_models(search=search, token=token)
    print(f"模型总数：{result.get('TotalCount', '?')}")
    for m in result.get("Models", result.get("models", [])):
        print(f"  - {m.get('ModelId', m.get('id', '')):40s}")


def print_usage():
    print("""
用法：python modelscope_helper.py <command> [args]

命令：
  verify                           验证 Token（从环境变量 MODELSCOPE_API_TOKEN 读取）
  skills [关键词] [--page N]      搜索技能（无需 Token）
  skill-detail <skill_id>         查看技能详情
  mcp [关键词] [--category xxx]   搜索 MCP 服务器（无需 Token）
  mcp-detail <server_id>         查看 MCP 详情（含 server_config）
  models [关键词]                 搜索模型（需 Token）

Token 说明：
  仅从环境变量 MODELSCOPE_API_TOKEN 读取，不保存到任何文件。
  如未设置，models 等需要 Token 的命令会提示引导注册。

示例：
  export MODELSCOPE_API_TOKEN="ms-xxxxxxxx"
  python scripts/modelscope_helper.py verify
  python scripts/modelscope_helper.py skills 股票
  python scripts/modelscope_helper.py mcp 高德 --category browser-automation
  python scripts/modelscope_helper.py mcp-detail @amap/amap-maps
""")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print_usage()
        sys.exit(0)

    cmd = sys.argv[1]
    # bash 可能会把 mcp-detail 拆成 mcp 和 -detail，合并回去
    if cmd == "skill" and len(sys.argv) > 2 and sys.argv[2] == "-detail":
        cmd = "skill-detail"
        rest = sys.argv[3:]
    elif cmd == "mcp" and len(sys.argv) > 2 and sys.argv[2] == "-detail":
        cmd = "mcp-detail"
        rest = sys.argv[3:]
    else:
        rest = sys.argv[2:]

    dispatch = {
        "verify": cmd_verify,
        "skills": cmd_skills,
        "skill-detail": cmd_skill_detail,
        "mcp": cmd_mcp,
        "mcp-detail": cmd_mcp_detail,
        "models": cmd_models,
    }

    if cmd in dispatch:
        dispatch[cmd](rest)
    else:
        print(f"未知命令：{cmd}")
        print_usage()
