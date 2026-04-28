> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-USER WEB - 用户管理 回归报告

**执行时间**: 2026-04-29  
**执行环境**: local (chromium, Midscene AI cache=1)  
**耗时**: 1m 45s

## 测试结果（chromium）

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-USER-00001 | 列表 - 分页/搜索/角色权限 | ✅ PASS | 29.7s |
| TC-USER-00002 | 详情抽屉 + 封禁 E2E 多端闭环 | ✅ PASS | 36.6s |
| TC-USER-00003 | 解封弹窗 - 原因必填 + 二次确认 | ✅ PASS | 38.4s |

**统计**: 3 PASS / 0 FAIL / 0 SKIP（chromium）

## 修复记录

**BUG-WEB-001** (先前轮次已修复): TC-USER-00003 用 `?status=normal` 筛选正常用户，解决页面无解封操作目标的问题。  
修复文件: `tests/scripts/WEB/TC-USER.spec.ts`
