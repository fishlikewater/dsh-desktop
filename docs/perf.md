# 性能基线（T6.1）

## release 构建 profile

`src-tauri/Cargo.toml` [profile.release]（T4.3 配置）：

```toml
codegen-units = 1
lto = true
opt-level = "s"
strip = true
```

## 构建产物

| 产物 | 体积 | 说明 |
|---|---|---|
| dsh-desktop.exe（release） | 6.6 MB | strip + opt-level=s |
| NSIS 安装包 | 1.6 MB（T4.3） | `DSH Desktop_0.1.0_x64-setup.exe` |

## 运行时基线

测量方式：`node scripts/bench/baseline.mjs`（release 构建、30s 采样、1s 间隔；
采样工作集与内核/用户 CPU 时间，末段间隔计算 CPU 占用率）。

| 指标 | 实测 | 目标 | 结果 |
|---|---|---|---|
| 内存（工作集）min/avg/max | 38.3 / 38.3 / 38.3 MB | < 300 MB | ✅ |
| 空闲 CPU | 0.00% | < 1% | ✅ |
| 启动到窗口就绪 | ~1.5s（T1.4 debug 实测） | < 3s | ✅ |

## 轮询节流

T5.3：服务探测轮询自适应——在线 10s / 离线 2s；iframe 加载超时 15s 提示重试。
