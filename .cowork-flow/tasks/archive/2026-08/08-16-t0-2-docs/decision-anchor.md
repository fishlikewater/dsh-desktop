# Decision Anchor

## 目标（T0.2，来自 docs/development-plan.md）
开源许可与项目文档：可交付的合规文档体系。

## 验收标准
- [ ] `LICENSE`（MIT，版权人 zhangxiang，年份 2026）。
- [ ] `CHANGELOG.md`（Keep a Changelog 格式，从 v0.1.0 起）。
- [ ] `SECURITY.md`（报告渠道 + 安全说明）。
- [ ] `README.md` 重构：移除机器绝对路径（E:\...）；补「安装/构建/发布/故障排查/安全说明」章节。
- [ ] README 构建步骤在干净环境可复现（不含本机特有路径）。

## 范围边界
- 范围内: LICENSE/CHANGELOG/SECURITY/README 文档。
- 范围外: 代码行为变更、lint 门禁（T0.3）、版本号提升。

## 验证命令
- 检查四个文档存在且无 `E:\` 绝对路径残留
- grep 关键章节标题
