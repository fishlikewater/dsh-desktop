# 最终验收报告（T6.2）

> 验收日期：2026-08-16 ｜ 版本：v0.1.0 ｜ 工具链：stable-x86_64-pc-windows-gnu（rustc 1.95.0）

## 一、全量回归

| 检查项 | 结果 |
|---|---|
| `npm run check`（fmt/clippy -D warnings/CSP hash/版本一致） | ✅ 通过 |
| `cargo test --lib` | ✅ 23 例全绿 |
| `npm run smoke`（window-center / theme-sync / isolation / window-ctrl） | ✅ 全部 4 场景通过 |
| release 构建 + NSIS 打包 | ✅ `DSH Desktop_0.1.0_x64-setup.exe`（1.6 MB） |
| 运行时基线（scripts/bench/baseline.mjs） | ✅ 内存 38.3MB（<300）、空闲 CPU 0.00%、exe 6.6MB |
| 设置页实测（T5.4） | ✅ 弹层开关、autostart 读取、update_check 404 可读报错 |
| 服务管理实测（T5.1/T5.2） | ✅ 假 dsh 拉起/停止、进程树杀灭、在线幂等、重复停止不误杀 |
| 错误恢复实测（T5.3） | ✅ 离线覆盖层 → 恢复自动进入 → 再断覆盖层重现 |

## 二、发布产物

| 产物 | 路径 | SHA256 |
|---|---|---|
| NSIS 安装包 | `src-tauri/target/release/bundle/nsis/DSH Desktop_0.1.0_x64-setup.exe` | `A74BE41016443FE6725C0725F05D9E3BD4D1BC3673E8608FD9D0B9AD3C257A25` |
| release exe | `src-tauri/target/release/dsh-desktop.exe` | 6.6 MB |

校验和文件：`src-tauri/target/release/bundle/nsis/SHA256SUMS.txt`（发布时随安装包一同分发）。

## 三、文档核对

- `README.md`：功能清单、安装/开发、架构、故障排查、安全、路线图 ✅（T6.2 补充设置页/服务管理/错误恢复条目）
- `docs/`：production-hardening-plan、development-plan、architecture、testing、perf、updater、release-process、final-acceptance ✅
- `CHANGELOG.md` 与版本号（0.1.0）三处一致 ✅（scripts/check-version.mjs）

## 四、已知限制与决策记录（均已在对应文档落盘）

| 限制 | 说明 | 记录位置 |
|---|---|---|
| 无 GitHub 远程 | CI workflow 已就绪但未触发过；updater endpoints 为占位（需填入 OWNER/REPO）；Releases 未发布 | README「持续集成」、docs/updater.md |
| 无代码签名证书 | SmartScreen 提示"未知发布者"；以 SHA256 校验和 + 官方渠道指引缓解 | SECURITY.md、docs/release-process.md |
| windows-gnu 测试缺陷 | test 二进制在特定代码形态下 0xc0000139（Child static / tauri async command / 集成测试引用 lib）；已按规避约定修复全部触发点 | docs/testing.md |
| 服务托管为方案 B（纯拉起） | 不捆绑 sidecar、不托管生命周期；停止仅作用于本应用拉起的进程树 | docs/architecture 相关、T5.1/T5.2 任务记录 |
| Ctrl+Shift+D 注册告警 | 本机其他应用占用该快捷键时注册失败（既有环境问题，不影响运行） | README 故障排查 |

## 五、结论

22 个开发任务（T0.1~T6.2）全部执行完毕并归档；全量回归通过；
发布产物（安装包 + SHA256）就绪。项目达到可交付状态（v0.1.0），
外部资源类限制均已明确记录。
