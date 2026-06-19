#!/usr/bin/env python3
"""生成 ModelScope API Skill 的示例截图"""

from PIL import Image, ImageDraw, ImageFont
import os

# 获取脚本所在目录
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCREENSHOTS_DIR = os.path.join(BASE_DIR, "screenshots")
os.makedirs(SCREENSHOTS_DIR, exist_ok=True)

def create_screenshot(title, content_lines, filename, width=800, height=600):
    """创建一个示例截图"""
    # 创建白色背景
    img = Image.new('RGB', (width, height), color='white')
    draw = ImageDraw.Draw(img)

    # 尝试使用系统字体
    try:
        title_font = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 32)
        content_font = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", 18)
    except:
        title_font = ImageFont.load_default()
        content_font = ImageFont.load_default()

    # 绘制标题背景
    draw.rectangle([0, 0, width, 80], fill='#0066CC')
    draw.text((20, 20), title, fill='white', font=title_font)

    # 绘制内容
    y_offset = 100
    for line in content_lines:
        if line.startswith("## "):
            # 二级标题
            draw.text((20, y_offset), line[3:], fill='#0066CC', font=content_font)
            y_offset += 35
        elif line.startswith("```"):
            # 代码块
            y_offset += 10
        else:
            # 普通文本
            draw.text((20, y_offset), line, fill='black', font=content_font)
            y_offset += 30

        if y_offset > height - 100:
            break

    # 保存图片
    img.save(filename)
    print(f"Created: {filename}")

# 截图1：Skill 功能概览
create_screenshot(
    "ModelScope API Skill - 功能概览",
    [
        "## 核心能力",
        "✓ 模型库搜索与下载 (19万+ 模型)",
        "✓ 数据集检索与管理 (4万+ 数据集)" ,
        "✓ 技能中心浏览与安装 (8万+ 技能)",
        "✓ MCP广场发现与部署 (9400+ MCP服务器)",
        "✓ 创空间(Studio)全生命周期管理",
        "",
        "## 使用示例",
        "```",
        "使用ModelScope API Skill搜索Qwen模型",
        "```",
        "",
        "触发词：魔塔、ModelScope、查模型、搜MCP、技能中心",
    ],
    os.path.join(SCREENSHOTS_DIR, "01-overview.png")
)

# 截图2：模型搜索示例
create_screenshot(
    "模型搜索示例 - Qwen2.5",
    [
        "## 搜索请求",
        "用户: 使用ModelScope API搜索Qwen2.5模型",
        "",
        "## API调用",
        "GET /models?search=Qwen2.5&sort=downloads",
        "",
        "## 返回结果 (前3条)",
        "1. Qwen/Qwen2.5-72B-Instruct",
        "   - 下载量: 1,234,567",
        "   - 点赞数: 8,901",
        "   - 任务: text-generation",
        "",
        "2. Qwen/Qwen2.5-7B-Instruct",
        "   - 下载量: 987,654",
        "   - 点赞数: 7,890",
        "",
        "3. Qwen/Qwen2.5-Coder-7B-Instruct",
        "   - 下载量: 654,321",
    ],
    os.path.join(SCREENSHOTS_DIR, "02-model-search.png")
)

# 截图3：创空间部署示例
create_screenshot(
    "创空间(Studio)部署示例",
    [
        "## 部署请求",
        "用户: 部署一个Gradio聊天机器人到创空间",
        "",
        "## 执行步骤",
        "1. 创建 app.py (Gradio入口文件)",
        "2. 创建 requirements.txt",
        "3. 初始化Git仓库",
        "4. 推送到ModelScope",
        "",
        "## API调用",
        "POST /studios (创建创空间)",
        "POST /studios/{id}/deploy (部署)",
        "",
        "## 部署结果",
        "✓ 创空间ID: 12345",
        "✓ 访问地址: https://modelscope.cn/studios/user/app",
        "✓ 状态: running",
    ],
    os.path.join(SCREENSHOTS_DIR, "03-studio-deploy.png")
)

# 截图4：MCP工具调用示例
create_screenshot(
    "MCP广场搜索与安装",
    [
        "## 搜索MCP服务器",
        "用户: 搜索文件操作相关的MCP服务器",
        "",
        "## API调用",
        "GET /mcp/servers?search=file&sort=downloads",
        "",
        "## 返回结果",
        "1. @modelcontextprotocol/server-filesystem",
        "   - 下载量: 123,456",
        "   - 描述: 文件系统访问MCP服务器",
        "",
        "## 安装命令",
        "```bash",
        "npm install @modelcontextprotocol/server-filesystem",
        "```",
    ],
    os.path.join(SCREENSHOTS_DIR, "04-mcp-search.png")
)

# 截图5：技能中心示例
create_screenshot(
    "技能中心 - 浏览与安装",
    [
        "## 技能中心统计",
        "• 总技能数: 80,000+",
        "• 分类: 33个",
        "",
        "## 热门技能",
        "1. 图像生成技能 (下载: 50万+)",
        "2. 文本分析技能 (下载: 30万+)",
        "3. 数据分析技能 (下载: 25万+)",
        "",
        "## 安装技能示例",
        "```bash",
        "# 搜索技能",
        "modelscope skills search --keyword 'image generation'",
        "",
        "# 安装技能",
        "modelscope skills install --skill-id 12345",
        "```",
    ],
    os.path.join(SCREENSHOTS_DIR, "05-skills-center.png")
)

print("\nAll screenshots created successfully!")
print(f"Screenshots saved to: {SCREENSHOTS_DIR}")
