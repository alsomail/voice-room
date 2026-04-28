> 当前状态机：负责人 [E2E] | 状态 [待回归] | 修复轮次 [1/5]

# TC-USER WEB - 用户管理 回归报告

**执行时间**: 2026-04-28 15:44 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1, Midscene AI)  
**关联任务**: T-0000P (Midscene env 注入)

## 测试结果

| 用例 ID | 用例名称 | 浏览器 | 结果 | 错误概要 |
|---------|---------|--------|------|---------|
| TC-USER-00001 | 列表 - 分页/搜索/XSS | chromium/firefox/webkit | ✅ PASS | - |
| TC-USER-00002 | 详情抽屉 + 封禁 E2E 多端闭环 | chromium | ✅ PASS | - |
| TC-USER-00002 | 详情抽屉 + 封禁 E2E 多端闭环 | firefox | ✅ PASS | - |
| TC-USER-00002 | 详情抽屉 + 封禁 E2E 多端闭环 | webkit | ❌ FAIL | AI 断言失败：封禁操作成功确认 (webkit 时序差异) |
| TC-USER-00003 | 解封闭环 | chromium/firefox/webkit | ✅ PASS | - |

**统计**: 8 PASS / 1 FAIL / 0 SKIP

## 失败分析

### TC-USER-00002 (webkit only) — AI 断言时序问题
- **现象**: AI 断言"封禁操作成功：用户抽屉已关闭，回到用户列表页"失败
- **根因**: webkit 渲染速度较慢，封禁操作后抽屉关闭动画未完成时 Midscene 已截图，AI 无法确认操作结果
- **修复方向**: 在封禁操作后增加 `page.waitForLoadState('networkidle')` 或 `page.waitForTimeout(1000)` 等待 UI 更新
- **截图**: `test-results/WEB-TC-USER-TC-USER-WEB----ba546-USER-00002-详情抽屉-封禁-E2E-多端闭环-webkit/test-failed-1.png`

**备注**: chromium + firefox 均通过，webkit 为边缘时序问题。
