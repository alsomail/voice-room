> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [1/5]

# TC-ANALYTICS WEB - 用户行为流Tab 回归报告

**执行时间**: 2026-04-29
**执行环境**: local (chromium, workers=1)
**关联任务**: T-20013 (用户详情页"行为流"Tab EventStreamTab)
**备注**: 本 spec 为本次 QA Gate 新建，首次执行即全绿。

## 测试结果

| 用例 ID | 用例名称 | 结果 |
|---------|---------|------|
| TC-ANALYTICS-00001 | 行为流Tab默认加载 + 空状态占位 | ✅ PASS |
| TC-ANALYTICS-00002 | 时间窗切换 + 自定义超30天前端限制 | ✅ PASS |
| TC-ANALYTICS-00003 | event_name多选下拉 + CSV导出按钮 | ✅ PASS |

**统计**: 3 PASS / 0 FAIL / 0 SKIP

## T-20013 关键验证点

- **TC-ANALYTICS-00001**: `[data-testid="event-stream-tab"]` 存在 ✅；默认加载最近 24h 数据 ✅；无数据时空状态占位渲染 ✅
- **TC-ANALYTICS-00002**: 时间筛选控件 `[data-testid="event-time-range"]` 可切换 ✅；自定义区间超 30 天前端拦截 ✅
- **TC-ANALYTICS-00003**: `[data-testid="event-name-select"]` 多选下拉展示 ✅；`[data-testid="btn-export-csv"]` 可点击 ✅
