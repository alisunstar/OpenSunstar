# Analysis — CHG-p1-rd-adapters

> /rd:clarify + /rd:analyze 产物 · ROUTING 命中：BIZ_IDENTITY:rd-delivery → main/INDEX.md；APPLICATION:opensunstar → applications/INDEX.md

## 影响面

| 模块 | 入口 | 结论 |
|------|------|------|
| 对账确定性段 | services/rd_validate.rs（新）+ cli/commands/rd.rs（新） | lib 重导出 pub use services::rd_validate |
| 索引合并器 | services/knowledge_routing.rs（新）+ wiki CLI Routing 子命令 | 受管标记 + 幂等 |
| 治理规则 | cli/commands/flow.rs rd_loop_governance_checks | 仅 preset=rd-loop 且 strict |
| readiness | ai/agent_readiness.rs compute_delivery_dimensions + 权重 93/7 | cli_api 追加 details |
| UI | ProjectFlowOrchestratorPanel（fullPreset 数据驱动）/ ProjectWikiPanel（engine 选择） | tsc 绿 |

## 澄清

- C-1：fullPreset vs selectedPreset——Summary 无 stages，七步条/工件清单必须取 fullPreset。
- C-2：交付维度未采纳记 not_required 满分，保 100 分制与未采纳语义不变。
- C-3：G-RD4 仅在 IMPLEMENTATION-CHECK 存在时校验；缺失由工件门禁阻断（职责分离）。
