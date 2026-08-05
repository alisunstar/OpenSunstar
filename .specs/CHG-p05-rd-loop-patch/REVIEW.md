# Review — CHG-p05-rd-loop-patch

> /rd:code-review 产物 · 4R 视角（risk/resilience/readability/reliability）

## Risk（风险）

- 核心新增逻辑仅“模板搬运”：无搜索/数据源/agent 循环（K1/K2 合规）；渲染仅 {{project_name}}/{{date}} 两占位符，确定性。
- 路径安全：is_safe_template_rel_path 拒绝绝对路径/盘符/`..`/空段；preview 与 install 双校验。

## Resilience（韧性）

- safe-install：存在即 skip，永不覆盖用户文件；二次安装测试断言 skip。
- 审计纳入：carried templates 写入 preview 临时目录，经 audit::scan_dir 扫描，CRITICAL 阻断安装。
- 向后兼容：projectTemplates serde default，旧 recipe 无字段仍可解析。

## Readability（可读）

- 边界注释明确（“pure carrier”）；rd-loop.json description 含 S1—S5 治理声明；SKILL.md 分发表述与产品现状一致。

## Reliability（可靠）

- 单测 3 例（携带+skip、unsafe 拒绝、frontmatter 往返）；门禁实跑负/正各一；tsc/i18n 基线绿。

## 结论

- 通过。partial 项（GUI 点击穿透）转 CONTINUE-PROMPT U-1，不阻塞发布门禁。
