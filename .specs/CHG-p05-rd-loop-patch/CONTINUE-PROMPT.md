# Continue Prompt — CHG-p05-rd-loop-patch

> 跨会话接续产物 · 新会话读此文件 + REQUIREMENT.md 即可继续

## 当前进度

- 七步闭环：verify-prd ✔ clarify/analyze ✔ decompose ✔ verify-requirement ✔（人工确认） apply ✔ validate ✔（本文件） code-review/release 见 REVIEW.md，backfill 见 KNOWLEDGE-BACKFILL.md。
- 门禁：2-decompose 负向阻断已证；7-backfill 正向放行已证（os 二进制独立构建）。

## 关键上下文

- 决策：A=A2（projectTemplates 携带扩展）、B=B2（zip skill 包）、C=保持命名+声明。
- 扩展边界：recipe 代码仅搬运；frontmatter serde_yaml 全结构往返；safe-install 不覆盖；路径校验拒绝 ../ 与绝对路径。
- i18n 计数语义：missing=zh∖locale 差集，新键四语言同加故计数不变（非异常）。

## 未决问题

- U-1 Skills GUI zip 安装点击穿透（人工验收）。
- U-2 WikiPanel 条件文案运行时确认（需导出 workflow.profile.json 后打开面板）。

## 接续指令

下一阶段（P1）：routing-extractor 适配器、backfill 生成器适配器、UI P1-a/b、rd-loop 独立语义规则集；先关闭 U-1/U-2。
