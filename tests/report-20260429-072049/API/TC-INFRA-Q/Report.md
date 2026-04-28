> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [1/5]

# TC-INFRA-Q API - e2e-up 端口冲突检测 回归报告

**执行时间**: 2026-04-29
**执行环境**: local (chromium, workers=1)
**关联任务**: T-0000A (Docker Compose / e2e-up.sh)

## 测试结果

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| I-1 | e2e-up.sh 端口冲突时阻止启动并显示清晰错误 | ✅ PASS |
| I-2 | e2e-up.sh 所有端口空闲时正常启动（可选） | ⏭️ SKIP-KNOWN |

**统计**: 1 PASS / 0 FAIL / 1 SKIP-KNOWN

## SKIP 原因说明

- **I-2**：需要所有端口完全空闲的干净 E2E 环境（`test.skip(true, '需要完整 E2E 环境，手动测试')`）；本地开发环境各服务均已运行，无法满足前提条件。属预期 SKIP，不影响验收。
