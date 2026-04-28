# 测试套件已知问题登记

> 本文件登记 voice-room 测试套件（Rust / E2E / Playwright）已知 flake 或环境性问题，便于研发与 QA 快速定位规避策略与跟踪 Task。新增条目请遵循「现象 / 触发条件 / 临时规避 / 手动跑命令 / 长期方向」5 必填字段模板。

---

<a id="web-001"></a>

## #web-001 — webkit Midscene 时序通用 wait pattern（抽屉/模态框关闭动画）

- **首次发现**：2026-04-29（T-0000R TC-USER-00002 webkit only FAIL）
- **测试位置**：`tests/scripts/WEB/TC-USER.spec.ts:89-95`（首次实现）
- **现象**：webkit 下 Midscene AI 断言抽屉关闭后的列表状态时，截图时机过早导致截到抽屉关闭动画中间帧（`.ant-drawer-open` 仍存在 DOM）。
- **触发条件**：
  1. 操作触发 Ant Design 抽屉（`.ant-drawer`）或模态框（`.ant-modal`）关闭。
  2. 操作后立即调用 Midscene `ai.assert()` 或 `ai.aiQuery()`（AI 断言依赖截图）。
  3. webkit 浏览器（chromium / firefox 无此问题，推测 webkit 渲染流水线时序差异）。
- **临时规避**（已在 T-0000R 中固化为通用模式）：
  ```typescript
  // 操作后串行等待两步再交给 AI 断言
  await page.waitForLoadState('networkidle');  // Step 1: 网络静默
  await page.locator('.ant-drawer-open').waitFor({ state: 'detached', timeout: 5_000 });  // Step 2: 抽屉 DOM 完全移除
  // 此时 AI 断言安全
  await ai.assert('用户列表中【E2E User A】的状态为【已封禁】');
  ```
  - **适用场景**：任何「操作 → 抽屉/模态框关闭 → AI 断言页面状态」链路。
  - **选择器替换**：`.ant-drawer-open` 对应抽屉；模态框用 `.ant-modal-wrap`（或子元素 `.ant-modal-mask`）。
- **手动跑**：`npm run e2e:local -- tests/scripts/WEB/TC-USER.spec.ts --grep "TC-USER-00002" --project=webkit --repeat-each=5`（重复 5 次验证稳定性）。
- **长期方向**：
  1. 抽象为 fixtures 辅助函数 `waitForDrawerClose(page)`（避免每个用例内联重复）。
  2. Midscene 自身可能增强「截图前自动等待动画结束」能力（未确认，需跟踪上游 issue）。
  3. 纯 DOM 断言不受影响（`expect(page.locator('.ant-drawer-open')).toHaveCount(0)` 无需此 pattern，仅 AI 断言需要）。
- **跟踪 Task**：T-0000R（已闭环 webkit 时序）；后续抽象为 fixtures 可立项 T-xxxxx。

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
