> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-ROOM WEB - 房间监控 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium, Midscene AI cache=1)  
**耗时**: 2m 05s

## 测试结果（chromium）

| 用例 ID | 用例名称 | 结果 | 耗时 | 跳过原因 |
|---------|---------|------|------|---------|
| TC-ROOM-00001 | Dashboard 概览 + ECharts + 30s 自动刷新 | ✅ PASS | 50.6s | - |
| TC-ROOM-00002 | 房间列表 - 筛选 / 分页 | ✅ PASS | 30.2s | - |
| TC-ROOM-00003 | 详情弹窗 - 强制关闭完整闭环 | ✅ PASS | 22.5s | - |
| TC-ROOM-00004 | XSS 防护 - 标题恶意输入 | ✅ PASS | 22.3s | - |
| TC-ROOM-00005 | 活跃房间监控增强 - 状态/时长/异常高亮 | ⏭️ SKIP | - | 对应后端路由未实装（T-0000R 范围外）|

**统计**: 4 PASS / 0 FAIL / 1 SKIP（chromium）
