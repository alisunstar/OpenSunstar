# /rd:extract-routing — 源码锚点提取适配器（外置）

> 知识资产层适配器：从源码提取业务锚点（Topic/状态枚举/业务身份常量/接口名/表名），产出**锚点补表草稿**入 knowledge/candidate/，经 owner review 后才并入正式页 frontmatter。核心不内建提取逻辑（K1），本命令由外部 Agent 执行。

## 先读规则

1. 读 knowledge/ROUTING.md 与 knowledge/applications/INDEX.md（现状路由）
2. 读 knowledge/KNOWLEDGE-RULES.md（candidate 治理规则）
3. 扫描范围仅限当前需求涉及的仓库路径（最小读取边界）

## 执行

1. 在扫描范围内识别锚点候选：MQ Topic 常量、状态枚举、业务身份常量、对外接口名、核心表名；
2. 为每个锚点给出：锚点类型（TOPIC/STATE/BIZ_IDENTITY/INTERFACE/MODEL）、字面值、候选应用、证据文件路径、建议挂载的正式页（applications/<app>/…）；
3. 只产出候选，不修改任何正式 knowledge/ 页与 ROUTING.md；
4. 证据不足者标 confidence: low，禁止推断成事实。

## 产物

- `knowledge/candidate/anchors-<YYYY-MM-DD>.md`，frontmatter 含：
  ```yaml
  type: candidate
  sourceType: ai-assisted
  engine: rd-extract-routing
  status: CANDIDATE
  confidence: medium
  evidence:
    - code: <证据文件路径>
  ```
  正文为锚点表：| 类型 | 字面值 | 候选应用 | 建议挂载页 | 证据 | confidence |

## 何时停止

- 扫描范围无法从需求/ROUTE 推断 → 停止，等人工划定范围。
- 敏感路径（密钥/客户数据/线上配置）一律不读不写。

## 人工确认

**是** — 草稿需 owner review：确认的锚点由人（或后续命令）写入正式页 frontmatter anchors；未确认者留在 candidate，不得被 os wiki routing 合并（合并器只读正式区）。

## 与索引合并器的关系

- 本适配器产“候选锚点”（外部智能）；
- `os wiki routing` 只合并正式区已声明 anchors（确定性核心）；
- 二者以 owner review 为界，错误知识不入 ROUTING。
