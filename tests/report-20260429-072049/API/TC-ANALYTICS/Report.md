> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [1/5]

# TC-ANALYTICS API - 埋点与观测性基建 回归报告

**执行时间**: 2026-04-29
**执行环境**: local (chromium, workers=1)
**关联任务**: T-00022 (events表 + HTTP批量接收), T-00023 (WS ReportEvent), T-10015 (用户行为查询API)
**备注**: 本 spec 为本次 QA Gate 新建，首次执行即全绿。

## 测试结果

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-ANALYTICS-00001 | HTTP批量上报 - 未登录device_id路径 | ✅ PASS |
| TC-ANALYTICS-00002 | JWT user_id 覆盖 + 超100事件截断 | ✅ PASS |
| TC-ANALYTICS-00003 | WS ReportEvent - server_ts覆盖 + ACK | ✅ PASS |
| TC-ANALYTICS-00004 | Admin用户行为查询API | ✅ PASS |
| TC-ANALYTICS-00005 | 时间窗超30天 → 400 | ✅ PASS |

**统计**: 5 PASS / 0 FAIL / 0 SKIP

## 关键验证点

- **T-00022 (HTTP batch)**: 未登录路径 `user_id IS NULL` ✅；JWT 路径 `user_id` 从 token 覆盖 ✅；100+ 事件截断至前 100 条，`rejected_indices` 含超界索引 ✅
- **T-00023 (WS ReportEvent)**: `EventReportAck` 正确返回 `{received, rejected_indices}` ✅；server_ts 以服务端时间覆盖 ✅；超 100 条截断 ✅
- **T-10015 (Admin查询)**: super_admin/operator 均可查非 admin_* 事件 ✅；时间窗超 30 天返回 400 ✅；event_name 过滤 ✅
