# Decision Anchor

## 目标（T6.1，来自 docs/development-plan.md）
性能与资源优化：内存/CPU 基线测量、轮询节流（T5.3 已做 10s/2s 自适应）、
发布构建 profile 优化、启动时间基线。

## 现状
- T5.3 已实现轮询自适应（在线 10s / 离线 2s）✓
- debug 构建体积极大（~200MB exe），release 打包产物 1.6MB 安装包（T4.3）✓
- release profile：未定制（默认 opt-level=3? tauri 模板默认）

## 实施
1. release profile 加固（Cargo.toml）：opt-level="s"（体积优先）+ strip=true +
   lto="thin" + codegen-units=1 + panic="abort"
2. 运行时基线测量脚本（scripts/bench/baseline.mjs）：
   启动后 30s 内存/CPU 采样（Get-Process 工作集、CPU 时间），输出基线 JSON；
   release 构建下：内存 < 300MB、CPU 空闲 < 1%
3. 启动时间：T1.4 已测（~1.5s），记录 release 启动耗时
4. 壳页资源：iframe 加载为 WebView2 共享进程（无需优化）

## 验收标准
- [ ] release 构建成功且体积不比当前显著增大（< 20MB exe）
- [ ] release 运行基线：内存 < 300MB、空闲 CPU < 1%（记录实测值）
- [ ] npm run check 全绿 + 23 单测通过
- [ ] 基线数据写入 docs/perf.md

## 验证命令
- cargo build --release；Get-Item exe 体积
- 启动 release + node scripts/bench/baseline.mjs
- npm run check；cargo test --lib
