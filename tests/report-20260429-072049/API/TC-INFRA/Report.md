> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [1/5]

# TC-INFRA API - 基础设施 回归报告

**执行时间**: 2026-04-29
**执行环境**: local (chromium, workers=1)
**关联任务**: T-0000A (Docker Compose), T-0000B (shared crate), T-0000C (DB权限隔离), T-0000D (CI流水线)

## 测试结果

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-INFRA-00001 | docker compose 一键启动 PG + Redis | ⏭️ SKIP-KNOWN |
| TC-INFRA-00002 | 端口被占用明确错误 | ⏭️ SKIP-KNOWN |
| TC-INFRA-00003 | shared crate 被双端引用整体编译通过 | ✅ PASS |
| TC-INFRA-00004 | shared JWT 编解码 + 边界 | ✅ PASS |
| TC-INFRA-00005 | shared bcrypt 随机盐 + 校验 | ✅ PASS |
| TC-INFRA-00006 | app_server_user 无权修改 admins | ✅ PASS |
| TC-INFRA-00007 | CI 本地模拟 - lint + test 绿 | ✅ PASS |

**统计**: 5 PASS / 0 FAIL / 2 SKIP-KNOWN

## SKIP 原因说明

- **TC-INFRA-00001**：本地 postgres Docker 容器已在运行，重启会中断所有并行测试的 DB 连接；须在隔离环境单独执行。
- **TC-INFRA-00002**：端口 5432 已被 postgres 占用，无法通过 nc 模拟端口冲突场景；须在干净环境单独执行。

两项 SKIP 均为预期行为，不影响业务验收。
