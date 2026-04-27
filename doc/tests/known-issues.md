# 测试套件已知问题登记

> 本文件登记 voice-room 测试套件（Rust / E2E / Playwright）已知 flake 或环境性问题，便于研发与 QA 快速定位规避策略与跟踪 Task。新增条目请遵循「现象 / 触发条件 / 临时规避 / 手动跑命令 / 长期方向」5 必填字段模板。

---

<a id="r08"></a>

## #r08 — ranking_test::r08_response_time_under_100ms perf flake

- **首次发现**：2026-04-27（T-0000M 联调期）
- **测试位置**：`app/server/tests/ranking_test.rs:458`
- **现象**：p95 偶发 ~315ms，超 100ms SLO 阈值。
- **触发条件**：
  1. 与其它 DB 集成测试并发跑（cargo 默认 `--test-threads`）。
  2. 冷连接池 warmup 抖动。
  3. 共享 docker postgres 资源争用。
- **临时规避**：测试已加 `#[ignore = "perf flake; tracked by T-0000O"]`；运行 `cargo test -p voice-room-server` 默认 skip。
- **手动跑**：`cargo test -p voice-room-server --test ranking_test r08_response_time_under_100ms -- --ignored --test-threads=1`。
- **长期方向**：迁移到独立 perf 套件（`tests/perf/` 或 `app/server/perf/`），加 warmup（N=5）+ p95 over N≥20 测量协议；CI 加 `perf-nightly` job 在低并发节点跑。
- **跟踪 Task**：T-0000O（Phase 1 已闭环），Phase 2 独立 perf 套件待立项。
