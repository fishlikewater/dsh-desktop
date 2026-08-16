# Decision Anchor

## 目标（T6.2，来自 docs/development-plan.md）
最终验收：全量回归 + 发布产物核验 + 交付文档核对。

## 验收清单
1. **全量回归**
   - npm run check（fmt/clippy/CSP hash/版本一致）
   - cargo test --lib（23 例）
   - npm run smoke（前置检查 + 4 场景：window-center/theme-sync/isolation/window-ctrl）
2. **发布产物**
   - release exe + NSIS 安装包存在
   - SHA256SUMS.txt 与安装包 hash 一致
3. **文档核对**
   - README.md 是否包含全部新功能入口（托盘菜单/设置页/服务管理/快捷键/开机自启）
   - docs/ 目录:architecture/testing/perf/updater/release-process 齐全
4. **限制记录核对**
   - 无 GitHub 仓库（CI/updater endpoints/Releases 占位）→ 已记录
   - 无代码签名证书 → 已记录
   - windows-gnu 测试缺陷 0xc0000139 → 已记录（docs/testing.md）

## 实施
- 跑全量回归,修复发现的问题
- README 补充新功能说明（如缺失）
- 产出 docs/final-acceptance.md（验收报告,含各项结果与限制清单）

## 验收标准
- [ ] 全量回归通过（或问题已修复后重跑通过）
- [ ] 发布产物 + 校验和一致
- [ ] README 与文档完整
- [ ] 验收报告落盘
