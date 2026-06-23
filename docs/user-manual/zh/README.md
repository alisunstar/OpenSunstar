# OpenSunstar 用户操作手册（简体中文）

**版本：** v0.6.x · **许可证：** MIT

> 面向 Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes 的一站式 AI 工具管理桌面应用。

---

## 目录

1. [一、OpenSunstar 是什么](#一opensunstar-是什么)
2. [二、安装指南](#二安装指南)
3. [三、快速开始](#三快速开始)
4. [四、API 接入](#四api-接入)
5. [五、Agent 配置管理](#五agent-配置管理)
6. [六、代理与高可用](#六代理与高可用)
7. [七、项目 AI 看板](#七项目-ai-看板)
8. [八、用量统计与会话管理](#八用量统计与会话管理)
9. [九、同步与备份](#九同步与备份)
10. [十、设置](#十设置)
11. [十一、常见问题 FAQ](#十一常见问题-faq)
12. [附录](#附录)

---

## 一、OpenSunstar 是什么

### 1.1 问题背景

在日常 AI 辅助编程中，你可能同时使用多个 CLI 工具：

- **Claude Code** — Anthropic 的 AI 编程助手
- **Claude Desktop** — Claude 桌面应用，支持官方登录与第三方 API
- **Codex** — OpenAI 的代码生成工具
- **Gemini CLI** — Google 的 AI 命令行工具
- **OpenCode** — 开源 AI 编程终端工具
- **OpenClaw** — 开源多供应商 AI 编程助手
- **Hermes** — Hermes Agent，支持供应商、MCP、Skills 和 Memory 管理

**痛点：**

- 每个工具有不同的配置文件格式（JSON、TOML、.env），手动编辑繁琐易错
- 切换 API 供应商需要逐个修改配置文件
- 多个供应商之间无法自动故障转移，一个出问题整个工作流中断
- MCP 服务器、Skills、Prompts 等 Agent 配置分散在各处，难以统一管理
- 无法直观监控 API 用量和费用
- 团队之间缺乏便捷的配置分享手段

### 1.2 OpenSunstar 解决方案

**OpenSunstar** 是一个跨平台桌面应用，让你：

- **一个应用管理七个工具** — Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes
- **三步快速接入** — 选择供应商 → 保存密钥 → 应用到 CLI，无需手动编辑配置文件
- **Agent 配置双向同步** — 一站式管理 MCP、Skills、Prompts、Commands、Hooks、Ignore 规则、工具权限、Subagents，跨工具跨设备同步
- **系统托盘快速切换** — 无需打开主窗口即可切换供应商
- **本地代理与高可用** — 格式转换、自动故障转移、熔断器保护、健康监控
- **项目 AI 看板** — 多项目组合矩阵，全生命周期管控、健康状态研判、成本动态监控
- **云同步** — WebDAV、S3 兼容存储、GitHub Gist 跨设备同步配置
- **跨平台** — Windows、macOS、Linux 原生桌面应用

### 1.3 核心特性一览

| 特性 | 说明 |
|------|------|
| **7 个 CLI 工具支持** | Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes |
| **快速接入向导** | 3 步完成供应商配置，预设 DeepSeek / OpenRouter / 智谱 GLM / Anthropic 官方 |
| **高级 Provider 模式** | 支持 20+ 供应商类型，完整 CRUD、端点测速、故障转移队列 |
| **Agent 配置管理** | MCP / Skills / Prompts / Commands / Hooks / Ignore / Permissions / Subagents 共 9 大模块 |
| **本地代理服务** | Anthropic ↔ OpenAI ↔ Gemini 格式转换，流式/非流式超时控制 |
| **故障转移与熔断器** | 多供应商优先级队列 + 三态熔断器 + 三层整流器 |
| **密钥安全存储** | 优先使用 OS Keychain（Windows 凭据管理器 / macOS Keychain / Linux Secret Service），AES-256-GCM 加密兜底 |
| **项目 AI 看板** | 三阶段管理（MVP/Rapid/Stable），AI 健康评分、风险分析、周报、自然语言查询 |
| **用量统计** | Token / 请求数 / 费用仪表盘，趋势图表，按供应商/模型分组统计 |
| **会话管理** | 浏览、搜索、恢复 AI 对话历史 |
| **云同步** | WebDAV / S3 兼容 / GitHub Gist，自定义配置目录 |
| **DeepLink 协议** | 一键导入供应商、MCP、Prompts、Skills 配置 |
| **4 语言国际化** | 简体中文 · 繁體中文 · English · 日本語 |
| **主题** | 深色 / 浅色 / 跟随系统 |
| **Skill 安全审计** | 安装前自动扫描，55+ 条规则覆盖 10 大威胁类别 |

---

## 二、安装指南

### 2.1 系统要求

| 平台 | 要求 |
|------|------|
| **Windows** | Windows 10 及以上 |
| **macOS** | macOS 12 (Monterey) 及以上 |
| **Linux** | Ubuntu 22.04+ / Debian 11+ / Fedora 34+（x64 / ARM64） |

### 2.2 Windows 安装

**方法 1：MSI 安装程序（推荐）**

1. 从 [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest) 下载最新版 `.msi` 文件
2. 双击运行安装程序
3. 按照提示完成安装

**方法 2：便携版**

1. 下载 Portable `.zip` 绿色版
2. 解压到任意目录
3. 运行 `OpenSunstar.exe`

### 2.3 macOS 安装

**方法 1：Homebrew（推荐）**

```bash
brew install --cask opensunstar
```

**方法 2：手动下载**

1. 从 [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest) 下载 `.dmg` 文件（已签名公证）
2. 双击挂载，拖入应用程序文件夹

### 2.4 Linux 安装

从 [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest) 下载对应包：

```bash
# Debian/Ubuntu
sudo dpkg -i OpenSunstar-{version}-Linux.deb

# Fedora/RHEL
sudo rpm -i OpenSunstar-{version}-Linux.rpm

# AppImage（通用）
chmod +x OpenSunstar-{version}-Linux.AppImage
./OpenSunstar-{version}-Linux.AppImage

# Flatpak
flatpak install --user ./OpenSunstar-{version}-Linux.flatpak
flatpak run com.opensunstar.desktop

# Arch Linux (AUR)
paru -S opensunstar-bin
```

---

## 三、快速开始

### 3.1 首次启动

1. **启动应用** — Windows 从开始菜单或桌面快捷方式；macOS 从应用程序文件夹；Linux 从应用菜单或命令行
2. **导入现有配置（自动）** — 首次启动时，系统自动扫描并导入现有 CLI 工具配置作为 `default` 供应商
3. **引导向导** — 自动弹出引导向导，展示扫描发现的供应商和 MCP 服务器
4. **进入快速接入** — 引导完成后默认停留在「快速接入」视图

### 3.2 界面总览

OpenSunstar 主界面采用经典的**侧边栏 + 内容区**布局。

**侧边栏结构（从上到下）：**

| 区域 | 包含内容 |
|------|----------|
| **API 接入** | 快速接入向导（SimpleConnect） |
| **Agent 配置** | 可折叠菜单：MCP、Skills、Prompts & Rules、命令管理、钩子管理、配置转换、忽略规则、工具权限、Subagent 管理 |
| **运行监控** | 会话管理（Context）、AI Tokens 用量 |
| **项目** | 项目 AI 看板、已添加项目列表、添加项目入口 |
| **同步备份** | WebDAV / S3 / Gist 同步管理 + 导入导出 |
| **底部** | 同步状态条、主题切换按钮、侧边栏折叠按钮、设置入口 |

**内容区：**
- 顶部工具栏：页面标题 + 当前 App 切换器（Prompts、Sessions 页面可见）+ 操作按钮
- 页面主体：各功能模块的操作界面

### 3.3 三步完成首次配置

推荐所有新用户通过「快速接入」完成首次配置：

1. **选供应商** — 在预设卡片中选择（DeepSeek / OpenRouter / 智谱 GLM / Anthropic 官方），或使用自定义 OpenAI 兼容端点
2. **配密钥** — 输入 API Key，保存到操作系统安全密钥链（Keychain）
3. **应用到 CLI** — 选择目标 CLI 工具，拉取模型列表，点击应用

> 详细操作步骤见 [第四章：API 接入](#四api-接入)。

### 3.4 切换供应商

**方法 1：系统托盘快速切换**

1. 点击系统托盘中的 OpenSunstar 图标
2. 直接点击要切换的供应商名称
3. 即时生效（无需打开主窗口）

**方法 2：快速接入页面切换**

1. 在快速接入页面重新完成三步向导
2. 应用到目标 CLI 工具

**方法 3：高级 Provider 面板**

1. 在快速接入页面切换到「高级 Provider」标签页
2. 选择供应商卡片，点击启用

> **注意：** 大多数 CLI 工具需要重启终端才能使用新配置。**Claude Code** 例外，支持热切换，无需重启终端。

### 3.5 切换回官方登录

如果需要从第三方 API 供应商切回官方登录方式：

1. 添加「Anthropic 官方」预设供应商（或选择「官方登录」等效配置）
2. 切换到该供应商
3. 在对应 CLI 工具中执行 Log out → Log in 流程
4. 之后可在官方和第三方供应商之间自由切换

### 3.6 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `Alt + 1` | 跳转 MCP 管理 |
| `Alt + 2` | 跳转 Prompts & Rules |
| `Alt + 3` | 跳转 Skills 管理 |
| `Alt + 4` | 跳转会话管理（Context） |
| `Alt + 5` | 跳转 AI Tokens 用量 |
| `Alt + 6` | 跳转项目 AI 看板 |
| `Ctrl + B` / `Cmd + B` | 折叠 / 展开侧边栏 |
| `?` 或 `Ctrl + /` | 呼出快捷键帮助面板 |
| `Esc` | 从子页面返回上级（MCP 发现 / Skills 发现） |

---

## 四、API 接入

OpenSunstar 提供两种 API 供应商配置方式：**快速接入**（SimpleConnect，推荐新手使用）和**高级 Provider 面板**（完整功能）。

### 4.1 快速接入（SimpleConnect）

快速接入是一个三步向导，提供简化的 API 供应商配置流程。核心设计原则：

- **密钥仅存操作系统 Keychain** — API Key 绝不出现于任何配置文件中
- **CLI 只写本地 Token** — CLI 配置文件中写入的是 `sc-local-{uuid}` 本地令牌
- **本地代理转发** — 真实请求通过 `127.0.0.1:17172` 本地代理转发，代理从 Keychain 取出真实 Key 完成上游调用

#### 4.1.1 第一步：选择供应商

进入快速接入页面，你将看到供应商卡片网格：

| 预设供应商 | API 端点 | 默认模型 |
|-----------|----------|----------|
| **DeepSeek** | `api.deepseek.com` | `deepseek-chat` |
| **OpenRouter** | `openrouter.ai/api` | `anthropic/claude-3.5-sonnet` |
| **智谱 GLM** | `open.bigmodel.cn` | `glm-4-flash` |
| **Anthropic 官方** | `api.anthropic.com` | `claude-sonnet-4-20250514` |
| **自定义 OpenAI 兼容** | 手动输入 | 按需配置 |

**操作：**

1. 点击选择供应商卡片（选中后高亮显示）
2. 如选择「自定义 OpenAI 兼容」，需手动输入 API Base URL（如 `https://api.example.com/v1`）
3. 点击「下一步」

> **注意：** 如果已为某供应商保存过 Key，切换到其他供应商时会弹出确认对话框，提示「切换后将使用新的 upstream，已保存的 Key 仍对应原供应商命名空间」。

#### 4.1.2 第二步：密钥管理

**基本操作：**

1. 输入 **主 API Key**（必填）
2. 点击「保存」或按 Enter 键，Key 将被写入操作系统安全密钥链
3. 保存成功后 Key 显示为脱敏格式（前 4 位 + `****` + 后 4 位）

**密钥池（可选）：**

开启密钥池后，可以添加多把 API Key，系统通过**加权轮询**算法自动分配请求：

- 点击「添加备用 Key」添加额外 Key
- 为每个备用 Key 设置**权重**（1-99）
- 可单独启用/禁用某个 Key
- 遇到 429（限流）响应时自动**阶梯冷却**：第 1 次冷却 3 秒 → 第 2 次 10 秒 → 第 3 次 30 秒 → 第 4 次 90 秒 → 第 5+ 次 300 秒

**高级设置：**

- **保存前校验 Key**（默认开启）：保存前调用 `/v1/models` 验证 Key 有效性；关闭后跳过验证
- **允许 URL 导入 Key**（默认开启）：允许通过 DeepLink URL 导入配置
- **备份目录安全扫描**：扫描备份目录，检查是否有 API Key 明文落盘

> **安全隐私声明：** OpenSunstar 遵循「本地优先、密钥存系统 Keychain、无遥测」的隐私原则。API Key 绝不写入任何配置文件、日志或备份文件。

#### 4.1.3 第三步：选择 CLI 并应用

**操作步骤：**

1. 从 CLI 工具网格中选择目标工具（6 个可选）：

   | CLI 工具 | 配置方式 |
   |----------|----------|
   | **Claude Code** | 写入 `~/.claude/settings.json` 环境变量 |
   | **Codex** | 写入 `~/.codex/config.toml` + auth.json |
   | **Gemini CLI** | 写入 `~/.gemini/settings.json` + `.env` 文件 |
   | **OpenCode** | 写入 OpenCode provider 配置 |
   | **OpenClaw** | 写入 `openclaw.json` 的 models.providers |
   | **Hermes** | 写入 `hermes.json` 的 custom_providers |

2. 点击「拉取模型」按钮，从供应商 API 动态获取可用模型列表
3. 在模型下拉菜单中选择目标模型
4. 点击「应用到 {CLI名}」按钮

**应用成功后：**
- 系统自动备份原有 CLI 配置到 `backups/` 目录
- 配置写入 CLI 工具的本地配置文件
- 本地代理自动启动（监听 `127.0.0.1:17172`）

**第三步可展开的折叠面板：**

- **CLI 配置状态** — 显示所有 6 个 CLI 的配置状态（绿色勾 = 已配置，灰色圈 = 未配置），已配置的工具可点击垃圾桶清除 SimpleConnect 写入的配置
- **密钥池运行状态**（仅密钥池开启时可见）— 实时轮询代理状态，显示每个 Key 的可用性和统计数据
- **用量概览** — 扫描本地 CLI 会话文件，汇总 Token 用量

> **注意：** 点击垃圾桶清除配置前，系统会先备份当前配置。清除后 CLI 工具恢复到被 OpenSunstar 管理前的原始状态。

### 4.2 高级 Provider 面板

高级 Provider 面板提供完整的供应商 CRUD 管理能力，适用于需要精细控制供应商配置的高级用户。

**切换到高级面板：** 在快速接入页面点击「高级 Provider」标签页。

#### 4.2.1 添加供应商

1. 点击「添加供应商」按钮
2. 选择供应商类型（Claude / Codex / Gemini / OpenCode / OpenClaw / Hermes / Claude Desktop 等，支持 20+ 类型）
3. 填写配置：
   - **名称** — 自定义供应商名称
   - **API Key** — 存入 Keychain
   - **Base URL** — API 端点地址
   - **模型列表** — 可用模型，支持手动添加和从 API 拉取
   - **高级配置** — 通用配置片段、用户代理、预算限制等
4. 点击「保存」

#### 4.2.2 供应商操作

- **启用** — 将供应商配置写入对应应用的 live 配置文件
- **编辑** — 修改 Key、端点、模型、通用配置片段等
- **删除** — 移除供应商（无法删除当前正在使用的供应商）
- **排序** — 拖拽供应商卡片调整顺序，影响故障转移优先级
- **端点测速** — 测试 API 端点的连通性和延迟
- **健康状态** — 查看供应商当前健康状态徽章

#### 4.2.3 通用配置片段

**解决的问题：** 切换供应商后，插件等扩展配置可能丢失。

**使用方法：**

1. 编辑供应商 → 打开通用配置面板
2. 点击「从当前供应商提取」保存所有通用数据
3. 创建新供应商时，默认勾选「写入通用配置」以保留扩展设置

#### 4.2.4 故障转移队列

在高级 Provider 面板中，可以将多个供应商加入故障转移队列：

1. 为供应商设置故障转移优先级（P1 → P2 → P3 …）
2. 拖拽排序调整优先级
3. 主供应商（P1）失败时自动按优先级切换到备用供应商

### 4.3 DeepLink 协议

OpenSunstar 支持通过 `OpenSunstar://` URL 协议一键导入配置，方便团队分享和快速部署。

**协议格式：**

```
OpenSunstar://v1/import?resource={类型}&app={应用}&name={名称}&apiKey={密钥}&endpoint={端点}&...
```

**支持的导入类型：**

| 资源类型 | 说明 |
|----------|------|
| `provider` | 导入供应商配置（含 API Key、端点、模型） |
| `mcp` | 导入 MCP 服务器配置 |
| `prompt` | 导入 Prompt 模板 |
| `skill` | 导入 Skill（GitHub 仓库地址） |

**SimpleConnect 专用 DeepLink：**

SimpleConnect 还支持额外的 URL 格式用于快速导入密钥：

```
beeapi://import?key={api_key}&supplier={供应商ID}&model={模型}&pool={是否启用池}
OpenSunstar://simple-connect/import?key={api_key}&supplier={供应商ID}
```

**使用方式：**

- 浏览器或终端中点击链接自动唤起 OpenSunstar
- 系统弹出确认对话框，展示脱敏后的导入信息
- 用户确认后导入到对应位置

---

## 五、Agent 配置管理

Agent 配置是 OpenSunstar 的核心能力之一，侧边栏以可折叠菜单形式组织，包含 **9 个子模块**，跨工具统一管理 AI Agent 的各类配置。

### 5.1 MCP 服务器管理

**MCP（Model Context Protocol）** 是 AI 工具与外部服务交互的标准协议。OpenSunstar 支持统一管理跨所有 CLI 工具的 MCP 服务器配置。

#### 添加 MCP 服务器

**方法 1：手动添加**

1. 进入 MCP 管理页面
2. 点击顶部「添加」按钮
3. 填写配置：
   - **名称** — 服务器名称
   - **命令** — 启动命令
   - **环境变量** — 运行时环境变量
   - **参数** — 命令行参数
4. 点击保存

**方法 2：从现有配置导入**

1. 点击顶部「导入」按钮
2. 系统自动扫描现有 CLI 配置文件中的 MCP 配置
3. 选择要导入的服务器，确认导入

**方法 3：从注册表发现**

1. 点击顶部「发现 MCP」按钮
2. 浏览 MCP 注册表中的公开可用服务器
3. 点击一键添加

#### 管理 MCP 服务器

- **应用绑定** — 为每个 MCP 服务器独立控制在各 CLI 工具（Claude / Codex / Gemini / OpenCode 等）中的启用/禁用状态
- **双向同步** — 在 OpenSunstar 中修改 → 同步到 live 配置文件；在配置文件中修改 → 回填到 OpenSunstar
- **连接验证** — 测试 MCP 服务器是否可正常启动和通信
- **编辑/删除** — 修改配置或移除服务器

### 5.2 Skills 管理

**Skills** 是 AI 工具的可复用能力模块，可以扩展 AI 的编程、分析、创作等能力。

#### 安装 Skills

**方法 1：从发现页安装**

1. 进入 Skills 管理页面，点击「发现」按钮
2. 在 Skills 发现页浏览或搜索可用 Skills
3. 支持从以下源发现：**skills.sh** / **ClawHub** / **ModelScope** / 自定义 Git 仓库
4. 点击安装按钮，一键安装

**方法 2：从 GitHub 仓库安装**

1. 在 Skills 管理页中直接输入 GitHub 仓库地址（格式：`owner/repo`）
2. 系统自动拉取并安装

**方法 3：从 ZIP 文件安装**

1. 点击「安装」按钮
2. 选择本地 ZIP 文件

**方法 4：从备份恢复**

1. 点击「恢复」按钮
2. 从卸载前自动备份的 Skill 中选择恢复

#### 管理 Skills

- **应用开关** — 为每个 Skill 独立控制在各 CLI 工具中的启用/禁用状态
- **仓库管理** — 添加/删除自定义 Skills 来源仓库
- **导入** — 从已有配置中导入 Skills

#### 安装方式

- **软链接（符号链接）** — 默认方式，在应用目录创建指向规范副本的符号链接，单一事实来源，统一更新
- **文件复制** — 为每个应用创建独立副本，适用于不支持符号链接的环境

可在「设置 → Skills 同步方式」中切换。

#### 自动备份与安全审计

- **卸载前自动备份** — 卸载 Skill 前自动备份到 `~/.OpenSunstar/skill-backups/`，保留最近 20 份
- **安装前安全审计** — 安装 Skill 前自动扫描源代码：
  - **55+ 条审计规则**，覆盖 10 大威胁类别
  - 检测项包括：提示注入（Prompt Injection）、隐藏 Unicode 字符攻击、数据外泄、凭证窃取、破坏性命令、配置篡改、动态代码执行、可疑 URL、自我传播等
  - 默认阻断 **CRITICAL** 级别安全发现
  - 可配置阻断阈值（Critical / High / Medium / Never）

### 5.3 Prompts & Rules

**Prompts & Rules** 用于编辑和管理 CLI 工具的提示词及规则文件。

**支持的文件：**
- `CLAUDE.md` — Claude Code 提示词与规则
- `AGENTS.md` — OpenClaw Agent 定义
- `GEMINI.md` — Gemini CLI 提示词
- 其他 Markdown 格式的提示词文件

**功能：**

- **按 App 绑定** — 顶部 AppSwitcher 切换不同 CLI 工具，每个工具独立的 Prompt 配置
- **Markdown 编辑器** — 内置编辑器，支持实时预览和语法高亮
- **桥接同步** — 编辑后同步到 live 配置文件；读取时从 live 文件回填，防止配置丢失
- **预览模式（Dry Run）** — 预览 Prompt 激活效果后再正式应用

### 5.4 命令管理（Commands）

自定义 AI 可调用的斜杠命令。

**操作：**

1. 点击「添加命令」按钮
2. 填写命令名、描述和执行内容
3. 可使用变量帮助面板查看支持的变量
4. 点击保存

**功能：**
- 添加 / 编辑 / 删除命令
- 变量帮助面板（查看可用变量语法）
- 按 App 启用/禁用

### 5.5 钩子管理（Hooks）

配置 AI 工具的生命周期钩子，在特定事件触发时执行自定义逻辑。

**支持的钩子事件类型：**

| 事件类型 | 触发时机 |
|----------|----------|
| **PreToolUse** | 工具调用前 |
| **PostToolUse** | 工具调用后 |
| **Notification** | 系统通知事件 |
| **Stop** | 会话停止时 |

**操作：**

1. 点击「添加钩子」按钮
2. 选择事件类型
3. 编写钩子脚本内容
4. 保存

每个钩子在列表中显示事件类型徽章，支持编辑/删除。

### 5.6 忽略规则（Ignore）

配置文件/目录的忽略规则，控制 AI 工具不读取或修改指定内容。

**操作：**

1. 点击「添加规则」按钮
2. 输入文件/目录路径模式
3. 保存

**功能：**
- 添加 / 编辑 / 删除忽略规则
- 从 `.gitignore` 文件一键导入规则

### 5.7 工具权限（Permissions）

管理 AI 工具的执行权限，精确控制哪些工具可以被 AI 调用。

**权限类型：**

| 类型 | 说明 |
|------|------|
| **allow** | 允许使用 |
| **deny** | 禁止使用 |
| **autoApprove** | 自动批准，无需用户确认 |

**功能：**
- 添加 / 编辑 / 删除权限规则
- 权限预设模板（一键应用常见权限组合）

### 5.8 Subagent 管理

定义和管理子 Agent 配置，让 AI 可以调度专门的子任务处理器。

**操作：**

1. 点击「添加 Subagent」按钮
2. 填写名称、描述和可用工具列表
3. 按 App 启用/禁用
4. 保存

### 5.9 配置转换（Convert）

在不同 AI 工具之间迁移配置。

**操作流程：**

1. 选择源工具（如 Claude Code）
2. 选择目标工具（如 Codex）
3. 系统自动检测可转换的配置项并生成预览
4. 确认后执行迁移

**支持的转换：** 供应商配置、MCP 服务器、Prompts、Skills 等。

---

## 六、代理与高可用

OpenSunstar 内置了一个强大的本地 HTTP 代理服务，位于 CLI 工具和 AI API 供应商之间，提供格式转换、故障转移、熔断器等功能。

### 6.1 本地代理服务

#### 是什么

运行在 `127.0.0.1:15721` 的本地 HTTP 代理服务器，接管 CLI 工具发出的 API 请求，在转发前进行格式转换、注入认证、故障转移等处理。

#### 为什么要用

- **统一格式转换** — 自动完成 Anthropic ↔ OpenAI Chat ↔ OpenAI Responses ↔ Gemini 之间的 API 格式互转
- **密钥安全** — CLI 工具不直接持有真实 API Key，降低密钥泄露风险
- **自动故障转移** — 主供应商失败时自动切换到备用，保障工作流不中断
- **用量记录** — 所有经过代理的请求被自动计入用量统计
- **请求优化** — 内置整流器自动修正不兼容的请求参数

#### 怎么用

1. 打开**设置 → 高级（Proxy 标签页）**
2. 开启本地代理服务
3. 配置监听地址和端口（默认 `127.0.0.1:15721`）
4. 通过**应用级接管开关**独立控制 Claude / Codex / Gemini 是否走代理

**代理覆盖的端点矩阵：**

| 端点路径 | 目标服务 |
|----------|----------|
| `/v1/messages` | Anthropic Messages API（Claude） |
| `/v1/chat/completions` | OpenAI Chat Completions API（Codex） |
| `/v1/responses` | OpenAI Responses API（Codex） |
| `/v1beta/*` | Gemini API |
| `/v1/models` | 模型列表查询 |
| `/claude-desktop/*` | Claude Desktop 3P 网关 |

> **注意：** 关闭代理服务时，系统会自动还原各 CLI 工具为直连配置，确保不影响正常使用。

### 6.2 自动故障转移

#### 是什么

当主供应商（P1）不可用时，按优先级队列自动切换到备用供应商（P2 → P3 → …），保障服务连续性。

#### 配置方法

1. 在高级 Provider 面板中为供应商设置故障转移优先级（P1、P2、P3...）
2. 拖拽排序调整优先级
3. 系统根据**健康检查**结果自动触发切换

#### 故障转移流程

1. 请求到达代理服务器
2. 代理尝试使用 P1 供应商
3. P1 失败（超时 / 5xx 错误 / 熔断器阻止）→ 自动尝试 P2
4. P2 失败 → 尝试 P3，以此类推
5. 成功后将切换事件通知前端 UI

**可配置的最大重试次数**（默认 3 次）防止无限重试。

### 6.3 熔断器

#### 是什么

熔断器是一种**三态机保护机制**，防止在供应商持续故障时浪费重试资源：

```
Closed（正常）→ Open（熔断）→ HalfOpen（探测恢复）→ Closed
```

**三态转换逻辑：**

| 状态 | 行为 | 转换条件 |
|------|------|----------|
| **Closed** | 所有请求正常放行 | 连续失败 ≥ 阈值 或 错误率 ≥ 阈值 → Open |
| **Open** | 所有请求被拒绝（快速失败） | 等待时间 ≥ timeout → HalfOpen |
| **HalfOpen** | 限流放行（最多 1 个探测请求） | 成功 ≥ 阈值 → Closed；失败 → 回到 Open |

#### 配置参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| 失败阈值 | 4 次 | 连续失败多少次后打开熔断器 |
| 恢复阈值 | 2 次 | HalfOpen 状态下成功多少次后恢复 Closed |
| 熔断超时 | 60 秒 | Open 状态维持多久后尝试 HalfOpen |
| 错误率阈值 | 60% | 滑动窗口内的错误率触发熔断 |
| 最小样本 | 10 次 | 计算错误率所需的最小请求数 |

在**设置 → 高级（熔断器配置）**中可按应用独立调整参数。

### 6.4 整流器

#### 是什么

三层请求/响应整流器，自动修正与上游 API 不兼容的请求和响应。

| 整流器 | 功能 | 触发场景 |
|--------|------|----------|
| **媒体降级整流器** | 将不兼容的图片输入替换为文本标记 | 上游因图片输入报错时 |
| **Thinking 签名整流器** | 移除无效的 thinking signature 字段 | 检测到签名不匹配错误 |
| **Thinking Budget 整流器** | 修正 budget_tokens 与 thinking 约束冲突 | budget 参数不合规时 |

整流器配置位于**设置 → 高级**，可独立开关。

### 6.5 超时控制

代理支持精细的超时配置：

| 超时类型 | 默认值 | 可配置范围 | 说明 |
|----------|--------|-----------|------|
| **流式首字节超时** | 60 秒 | 1-120 秒 | 流式响应的第一个字节到达时限 |
| **流式静默超时** | 120 秒 | 60-600 秒（0=禁用） | 流式响应中相邻数据块之间的最大间隔 |
| **非流式总超时** | 600 秒 | 60-1200 秒 | 非流式请求的完整超时 |

### 6.6 Copilot 优化器（实验性）

针对 GitHub Copilot 代理场景的专用优化器，包含：

- **请求分类** — 自动区分 agent / warmup 请求
- **工具结果合并** — 合并 tool result 减少 API 调用
- **Warmup 降级** — 将 warmup 请求降级到更经济的模型
- **Thinking 剥离** — 对不支持 thinking 的模型自动剥离 thinking blocks

---

## 七、项目 AI 看板

### 7.1 是什么

项目 AI 看板是**多 Git 仓库的研发驾驶舱**，不是传统拖拽式任务看板。它基于代码提交和 AI 交互数据，提炼项目的健康度、活跃度与成本效率指标。

### 7.2 添加项目

1. 点击侧边栏底部的「+ 添加项目」按钮，或在看板页点击添加
2. 填写**项目名称**和**本地 Git 仓库路径**
3. 系统自动扫描代码统计（代码行数、语言分布）和 Git 提交历史

### 7.3 三阶段管理

看板按项目生命周期分为三个阶段列：

| 阶段 | 含义 |
|------|------|
| **MVP** | 最小可行产品阶段，快速迭代验证 |
| **Rapid** | 快速成长阶段，功能密集开发 |
| **Stable** | 稳定维护阶段，质量与优化优先 |

- 每个项目卡片可通过 StagePicker 自由切换阶段
- 拖拽项目卡片跨阶段移动

### 7.4 核心指标与分析

**汇总卡片：**
- 近 7 天 Git 提交数
- 代码总行数
- 贡献者数量

**提交趋势图：**
- 支持 7 天 / 30 天窗口切换
- 可视化展示提交频率变化

**AI 健康评分：**
- 基于 30 天提交频率、代码变更量、贡献者活跃度等维度
- 自动判定健康等级

**项目组合矩阵（Portfolio Matrix）：**
- X 轴：提交活跃度
- 气泡大小：代码规模
- 一图纵览所有项目的相对位置

**AI 风险分析：**
- 自动识别低活跃度、代码质量波动等风险信号
- 并在项目卡片上标记

**AI 周报：**
- 基于 7 天窗口自动生成项目组合周报
- 包含关键指标摘要、趋势判断和关注建议

**自然语言查询（NL Query）：**
- 在看板页的查询栏中使用自然语言提问
- 例如「最近一周哪个项目最活跃？」「哪些项目超过 30 天没有更新？」
- AI 自动解析并返回答案

**AI 成本面板：**
- 按项目维度统计 AI 调用费用
- 帮助评估 AI 工具的 ROI

> **注意：** AI 洞察功能（健康评分、风险分析、周报、NL 查询）需要在「设置 → AI 供应商」中配置可用的 AI 模型。

---

## 八、用量统计与会话管理

### 8.1 AI Tokens 用量仪表盘

进入「AI Tokens」页面，查看跨供应商的 AI 用量全景视图。

**用量概览卡片：**
- 总 Token 消耗
- 总请求次数
- 总费用（基于配置的模型定价）

**趋势图表：**
- 可视化 Token 消耗趋势
- 支持自定义日期范围

**按供应商统计表：**
- 分组展示各供应商的 Token 消耗、请求数、费用

**按模型统计表：**
- 分组展示各模型的使用量

**请求日志表：**
- 单次请求级别的详细信息：时间、模型、Token 输入/输出、延迟、状态码

### 8.2 定价配置

OpenSunstar 允许自定义模型定价，确保成本统计准确：

1. 打开 AI Tokens 页面 → 定价配置面板（或 设置 → 用量标签页）
2. 添加/编辑模型价格：
   - **输入价格**（每百万 Token）
   - **输出价格**（每百万 Token）
   - **缓存读取价格**（每百万 Token）
   - **缓存写入价格**（每百万 Token）
   - **倍率系数**（用于中转供应商的价格加成）
3. 保存

> **注意：** 定价配置仅影响本地的成本统计显示，不影响实际 API 账单。

### 8.3 用量导出

在 AI Tokens 页面顶部可找到导出菜单，将用量数据导出为外部格式。

### 8.4 预算告警

为每个供应商设置费用上限，防止意外超支：

1. 编辑供应商 → 高级配置 → 预算限制
2. 设置**日预算**和/或**月预算**上限
3. 超限时系统通过桌面通知提醒

### 8.5 会话管理（Context）

「Context」页面用于浏览和搜索 AI CLI 工具的对话历史。

**功能：**

- **App 切换** — 顶部 AppSwitcher 切换查看不同 CLI 工具的会话记录
- **会话列表** — 每项显示时间戳和对话摘要
- **消息详情** — 点击进入查看完整对话消息列表
- **搜索** — 按关键词搜索历史对话
- **删除** — 删除不需要的会话记录
- **导出** — 导出会话内容

**支持查看会话的 CLI 工具：** Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、Hermes。

---

## 九、同步与备份

### 9.1 云同步

OpenSunstar 支持三种云同步方式，确保配置跨设备可迁移。

> **注意：** WebDAV 和 S3 同步同时只能启用一种。

#### WebDAV 同步

适用于自建 NAS 或支持 WebDAV 的云存储服务。

**配置步骤：**

1. 进入「同步备份」页面
2. 填写 WebDAV 服务器信息：
   - **服务器地址**（如 `https://your-nas.com/webdav`）
   - **用户名**
   - **密码**
   - **路径**（如 `/opensunstar/`）
3. 测试连接
4. 选择同步模式：
   - **手动** — 手动点击上传/下载
   - **自动** — 定时自动同步

#### S3 兼容存储同步

支持 AWS S3、Cloudflare R2、MinIO、阿里云 OSS、腾讯云 COS、华为云 OBS 等所有 S3 兼容存储。

**配置步骤：**

1. 选择 S3 服务类型（预设或自定义）
2. 填写连接信息：
   - **Endpoint** — 服务端点 URL
   - **Access Key** — 访问密钥
   - **Secret Key** — 秘密密钥
   - **Bucket** — 存储桶名称
   - **Region**（可选）
3. 测试连接
4. 启用自动同步或手动操作

#### GitHub Gist 同步

适用于开发者，利用 GitHub Gist 同步配置。

1. 进入「同步备份」页面 → Gist 同步区块
2. 输入 **GitHub Personal Access Token**（需要 gist 权限）
3. 测试连接
4. 启用自动/手动同步

### 9.2 配置目录自定义

在**设置 → 常规（目录设置）**中，可以将数据目录指向任意路径，实现与云盘的自动同步：

- Dropbox：`~/Dropbox/OpenSunstar/`
- OneDrive：`~/OneDrive/OpenSunstar/`
- iCloud：`~/Library/Mobile Documents/com~apple~CloudDocs/OpenSunstar/`
- 坚果云 / NAS 等任意路径

### 9.3 本地备份

**自动备份：**
- 配置更改时自动备份完整快照
- 保留最近 **10 份**自动备份
- 备份位置：`~/.OpenSunstar/backups/`

**手动备份：**
- 设置 → 关于 → 备份管理
- 可随时手动触发备份
- 可选择历史备份进行恢复

### 9.4 导入 / 导出

**导出：**
- 完整导出：SQLite 数据库（供应商、MCP、Prompts、Skills、项目、设置等）+ Skills 文件
- 在「同步备份」页面或「设置 → 常规」中操作

**导入：**
- 选择导出包文件
- 系统解析并展示预览
- 确认后恢复

> **注意：** 导入前强烈建议先手动备份当前配置，以防覆盖重要数据。

---

## 十、设置

设置页面分为 **4 个标签页**：常规、高级、用量、关于。

### 10.1 常规设置

| 设置项 | 说明 |
|--------|------|
| **主题** | 深色 / 浅色 / 跟随系统 |
| **语言** | 简体中文 / 繁體中文 / English / 日本語 |
| **终端设置** | 选择终端类型（影响配置写入行为） |
| **窗口设置** | 窗口行为（如启用自定义标题栏） |
| **开机启动** | 系统登录时自动启动 OpenSunstar |
| **App 可见性** | 隐藏不常用的 CLI 工具，精简界面 |
| **Skills 存储位置** | 自定义 Skills 存储路径（默认 `~/.OpenSunstar/skills/`） |
| **Skills 同步方法** | 软链接（符号链接）/ 文件复制 |
| **目录设置** | 数据目录路径（详见 9.2 节） |
| **导入/导出** | 完整配置导出与恢复 |
| **备份列表** | 查看和管理历史备份 |

### 10.2 高级设置

| 设置项 | 说明 |
|--------|------|
| **代理配置** | 启用/禁用代理、监听地址/端口、应用级接管开关 |
| **全局代理** | HTTP/HTTPS 出站代理（用于通过代理访问外网 API） |
| **故障转移** | 自动故障转移开关与相关阈值 |
| **熔断器** | 失败阈值 / 恢复阈值 / 熔断超时 / 错误率阈值（按应用独立配置） |
| **整流器** | 媒体降级 / Thinking 签名修复 / Thinking Budget 修正（独立开关） |
| **Copilot 优化器** | 请求分类 / 工具结果合并 / Warmup 降级 / Thinking 剥离（独立开关） |
| **Codex 认证** | OpenAI 登录配置（用于 Codex OAuth 模式） |
| **AI 供应商** | 为看板 AI 功能配置模型（健康评分 / 周报 / NL 查询等） |
| **日志配置** | 日志级别和输出控制 |
| **Dry Run 模式** | 预览模式开关（操作前预览效果但不实际写入） |

### 10.3 用量设置

- **定价配置** — 编辑各模型的价格参数（详见 8.2 节）
- **模型测试配置** — 发送测试请求验证模型可用性和延迟

### 10.4 关于

- **版本信息** — 当前版本号与更新日志链接
- **自动更新** — 检查并安装应用内更新
- **备份管理** — 查看、创建、恢复备份
- **法律信息** — 开源许可证（MIT）与相关声明

---

## 十一、常见问题 FAQ

### 配置相关

**Q：切换供应商后需要重启终端吗？**

A：大多数 CLI 工具需要重启终端才能加载新配置。**Claude Code** 例外，支持热切换，无需重启。Codex、Gemini CLI、OpenCode、OpenClaw、Hermes 需要重启终端。Claude Desktop 需要重启应用。

**Q：为什么不能删除当前正在使用的供应商？**

A：遵循「最小侵入」设计原则，确保 CLI 工具始终至少有一个可用配置。如需删除，先切换到其他供应商，再删除目标供应商。

**Q：如何切回官方登录（如 Claude Desktop Pro 订阅）？**

A：添加 Anthropic 官方预设供应商 → 切换到该供应商 → 在对应 CLI 中执行 Log out → Log in 流程。之后可在官方和第三方供应商间自由切换。

**Q：切换供应商后插件配置消失了怎么办？**

A：使用「通用配置片段」功能：编辑供应商 → 通用配置面板 → 点击「从当前供应商提取」，保存所有通用扩展数据。新建供应商时保持「写入通用配置」勾选即可保留。

### 安全相关

**Q：API Key 存储安全吗？**

A：安全。API Key 优先存储于操作系统原生 Keychain（Windows 凭据管理器 / macOS Keychain / Linux Secret Service）。若 Keychain 不可用，使用 AES-256-GCM 加密文件兜底。CLI 配置文件中只写入 `local` 令牌，绝不写入真实 API Key。

**Q：SimpleConnect 的本地代理（端口 17172）与高级代理（端口 15721）有什么区别？**

A：两者都是本地 HTTP 代理，但职责不同：
- **SimpleConnect 代理（17172）** — 专为快速接入模式服务，负责密钥池轮询和 429 冷却
- **高级代理（15721）** — 完整的代理功能，支持格式转换、故障转移、熔断器、整流器等全部能力

两个代理都仅绑定 `127.0.0.1`，不暴露到外部网络。

### 使用相关

**Q：项目看板的数据存储在哪里？**

A：项目列表存储在 SQLite 数据库 `~/.OpenSunstar/OpenSunstar.db` 的 `projects` 表中。项目阶段和进度信息存储在浏览器 `localStorage` 中。AI 洞察缓存、成本日志、查询日志也存储在 SQLite 中。

**Q：用量统计数据从哪里来？**

A：来源包括：(1) 代理请求日志 — 所有经过代理的 API 请求自动记录 Token 用量；(2) CLI 会话文件扫描 — 扫描本地 CLI 工具（Claude Code / Codex 等）的会话日志提取用量数据；(3) 可选官方订阅额度模板。

**Q：WebDAV 和 S3 同步可以同时启用吗？**

A：不能。WebDAV 和 S3 同步同时只能启用一种。GitHub Gist 同步独立于二者。

**Q：怎么恢复误删的 Skill？**

A：Skills 卸载前自动备份到 `~/.OpenSunstar/skill-backups/`（保留最近 20 份），在 Skills 管理页点击「恢复」按钮即可选择恢复。

### 故障排查

**Q：本地代理启动失败怎么排查？**

A：(1) 检查端口（默认 15721）是否被其他程序占用；(2) 在设置中修改监听端口；(3) 查看日志配置确认错误详情。

**Q：供应商健康状态显示异常但实际可用？**

A：(1) 检查网络连接和防火墙设置；(2) 在供应商设置中执行端点测速验证；(3) 可手动重置该供应商的熔断器状态。

**Q：DeepLink 导入没有反应？**

A：(1) 确认 URL 格式正确（以 `OpenSunstar://` 开头）；(2) 确认应用已注册 URL 协议（首次启动自动注册）；(3) 检查安全设置中是否开启了「允许 URL 导入 Key」。

**Q：键盘快捷键冲突怎么处理？**

A：按 `?` 或 `Ctrl+/` 查看完整的快捷键列表。当前不支持自定义快捷键。

---

## 附录

### A. 数据路径速查表

| 路径 | 内容 |
|------|------|
| `~/.OpenSunstar/OpenSunstar.db` | SQLite 主数据库 — 供应商、MCP、Prompts、Skills、项目、AI 缓存、用量日志等（29 张表） |
| `~/.OpenSunstar/settings.json` | 界面与设备偏好 |
| `~/.OpenSunstar/backups/` | 自动备份（保留最近 10 份） |
| `~/.OpenSunstar/skills/` | Skills 默认存储目录 |
| `~/.OpenSunstar/skill-backups/` | Skills 卸载前自动备份（保留最近 20 份） |
| `~/.OpenSunstar/keystore.enc` | AES-256-GCM 加密的密钥兜底文件（当 Keychain 不可用时） |

### B. 支持的应用列表

| App ID | 应用名称 | 配置目录 | 格式兼容性 |
|--------|----------|----------|-----------|
| `claude` | Claude Code | `~/.claude/` | Anthropic-compatible |
| `claude-desktop` | Claude Desktop | Claude Desktop 配置 | Anthropic-compatible |
| `codex` | Codex | `~/.codex/` | OpenAI-compatible |
| `gemini` | Gemini CLI | `~/.gemini/` | Gemini / OpenAI bridge |
| `opencode` | OpenCode | `~/.opencode/` | OpenCode provider |
| `openclaw` | OpenClaw | `~/.openclaw/` | OpenClaw models.providers |
| `hermes` | Hermes | Hermes Agent 配置 | Hermes custom_providers |

### C. SimpleConnect 预设供应商

| 供应商 ID | 名称 | API 端点 | 默认模型 |
|-----------|------|----------|----------|
| `deepseek` | DeepSeek | `api.deepseek.com` | `deepseek-chat` |
| `openrouter` | OpenRouter | `openrouter.ai/api` | `anthropic/claude-3.5-sonnet` |
| `zhipu` | 智谱 GLM | `open.bigmodel.cn` | `glm-4-flash` |
| `anthropic` | Anthropic 官方 | `api.anthropic.com` | `claude-sonnet-4-20250514` |
| `custom` | 自定义 OpenAI 兼容 | 手动输入 | 按需配置 |

### D. 键盘快捷键完整参考

| 快捷键 | 功能 |
|--------|------|
| `Alt + 1` | 跳转 MCP 管理 |
| `Alt + 2` | 跳转 Prompts & Rules |
| `Alt + 3` | 跳转 Skills 管理 |
| `Alt + 4` | 跳转会话管理（Context） |
| `Alt + 5` | 跳转 AI Tokens 用量 |
| `Alt + 6` | 跳转项目 AI 看板 |
| `Ctrl + B` / `Cmd + B` | 折叠 / 展开侧边栏 |
| `?` 或 `Ctrl + /` | 呼出快捷键帮助面板 |
| `Esc` | 从子页面返回上级 / 关闭弹窗 |

---

> **相关文档：** [繁體中文手冊](../zh-TW/README.md) · [English Manual](../en/README.md) · [日本語マニュアル](../ja/README.md) · [Deutsch Handbuch](../de/README.md)
>
> [← 手册索引](../../user-manual/README.md) · [v0.1.0 发布说明](../../release-notes/v0.1.0-zh.md) · [项目组合模块说明](../../kanban.md)

---

*本文档基于 OpenSunstar v0.6.x 编写，功能可能随版本更新而变化。请参考 [更新日志](../../../CHANGELOG.md) 获取最新信息。*
