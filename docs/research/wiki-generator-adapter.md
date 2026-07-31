# Wiki 可插拔生成器协议

OpenSunstar 是 Wiki 生命周期的控制面，外部生成器只负责产出 Markdown 候选。生成器不得直接修改项目正式 `wiki/`，也不得自行宣称内容已经与源码同步。

## 目录协议

每次生成写入独立运行目录：

```text
.opensunstar/wiki/candidates/<run-id>/
├── candidate.json
└── wiki/
    ├── index.md
    └── ...
```

`wiki/index.md` 是候选有效的最低条件。`candidate.json` 使用以下字段：

```json
{
  "engine": "openwiki",
  "created_at": 1785235200,
  "source_commit": "<full git commit>",
  "model": "<provider model id>",
  "generation_seconds": 123.4
}
```

其中 `source_commit` 与 `model` 是质量横向对照的硬条件。任一候选缺失字段，或多个候选值不同，控制面都会将报告标为“不可比较”。

## OpenWiki 内置适配器

“运行 OpenWiki”执行以下边界：

1. 从当前 Git `HEAD` 创建隔离源码快照；非 Git 项目回退为排除依赖、构建产物、旧 Wiki 和控制面状态的文件快照。
2. 在隔离快照内运行 `openwiki code --init --print`，最长 30 分钟。
3. OpenWiki 写入的 `openwiki/`、`AGENTS.md`、`CLAUDE.md` 等副作用均留在隔离工作区。
4. 仅将有效的 `openwiki/` 复制到候选目录，并记录 Commit、模型与耗时。
5. 用户显式导入候选时，控制面先备份正式 `wiki/`，再写入候选，并进入“待验收”。
6. 质量 Lint 通过且用户验收后，控制面才记录新的 Commit 与内容哈希基线。

OpenWiki CLI 与模型凭据由用户环境配置；OpenSunstar 不读取、不保存、也不回显 API Key。CLI 不可用、超时或生成失败时，生命周期进入“操作失败”，正式 Wiki 保持不变。

## 其他生成器（包括 CodeWiki）

其他生成器无需链接到 OpenSunstar 进程，也无需采用同一许可证。只需把输出转换为上述候选目录和元数据即可接入。控制面随后统一负责：

- 候选发现与安全导入；
- 正式 Wiki 备份；
- Lint 与人工验收；
- Git 变更检测；
- 同 Commit、同模型质量对照。

CodeWiki 对照实验应从与 OpenWiki 相同的 Commit 生成，使用同一模型，并将转换后的结果标记为 `engine: "codewiki"`。当前自动对照指标包括页面数、质量等级、有效源码引用、无效源码引用、待查项和生成耗时；不将这些启发式指标合成为主观总分。

## 生命周期

首次生成：

```text
未初始化 → 待生成 → 正在生成 → 候选待导入 → 待验收 → 已同步到 Commit
```

增量更新：

```text
已同步到 Commit → 检测到变更 → 同步中 → 候选待导入 → 待验收 → 已更新
```

如果验收后的工作区仍有未提交源码变化，刷新会再次回到“检测到变更”；控制面不会把工作区变化误报为已同步到 Commit。
