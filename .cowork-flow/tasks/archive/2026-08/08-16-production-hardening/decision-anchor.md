# Decision Anchor

## 目标
产出 DSH Desktop 生产级改造的完整规划产物：
1. `docs/production-hardening-plan.md` — 总方案（现状盘点、六阶段差距清单、决策记录、成功标准）
2. `docs/development-plan.md` — 开发计划与任务分解（22 个实施任务的权威清单：目标/范围/验收/依赖/规模/执行约定）

## 已确认决策（用户）
- 工具链：保留 windows-gnu，config 可移植化，CI 用 gnu runner
- 许可证：MIT
- 发布通道：GitHub Releases + tauri updater 自动更新
- 代码签名：无证书，文档化降级（SHA256 + SmartScreen 说明）
- 范围：含全部阶段 P0~P6
- 服务集成：方案 B（纯拉起系统 dsh，壳层托管生命周期 + 状态感知 UI）

## 验收标准
- [x] 方案文档完整（P0~P6 每阶段差距清单、优先级、依赖、规模、验收）。
- [x] 开发计划将方案拆为 22 个任务，每任务含目标/范围/验收/依赖/规模。
- [x] 决策记录与用户确认一致（含服务集成方案 B）。
- [x] 执行约定明确（任务创建方式、DoD 门槛、依赖顺序、范围边界）。

## 后续
方案批准后按阶段创建实施任务：每任务用 `task next --run --title <T编号> ... --from-plan .cowork-flow/plans/2026-08-16-production-hardening-devplan.md` 创建，逐项走 planning → implement → review → archive。

## 验证命令
- task next --validate 通过
- docs 与 .cowork-flow/plans/ 副本一致
