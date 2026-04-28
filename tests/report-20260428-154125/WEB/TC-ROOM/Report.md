> 当前状态机：负责人 [E2E] | 状态 [待回归] | 修复轮次 [1/5]

# TC-ROOM WEB - 房间监控 回归报告

**执行时间**: 2026-04-28 15:44 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1, Midscene AI)  
**关联任务**: T-0000P (Midscene env 注入)

## 测试结果

| 用例 ID | 用例名称 | 浏览器 | 结果 | 错误概要 |
|---------|---------|--------|------|---------|
| TC-ROOM-00001 | 列表 - 当前活跃房间可见 | chromium/firefox/webkit | ✅ PASS | - |
| TC-ROOM-00002 | 房间列表 - 筛选 / 分页 | chromium | ❌ FAIL | AI 断言失败：当前页码为第2页（实际停留在第1页） |
| TC-ROOM-00002 | 房间列表 - 筛选 / 分页 | firefox | ❌ FAIL | AI 断言失败：当前页码为第2页（实际停留在第1页） |
| TC-ROOM-00002 | 房间列表 - 筛选 / 分页 | webkit | ❌ FAIL | AI 断言失败：当前页码为第2页（实际停留在第1页） |
| TC-ROOM-00003 | 详情弹窗 - 强制关闭完整闭环 | chromium | ❌ FAIL | strict mode violation: getByText('确认强制关闭') 匹配 2 个元素 |
| TC-ROOM-00003 | 详情弹窗 - 强制关闭完整闭环 | firefox | ❌ FAIL | strict mode violation: getByText('确认强制关闭') 匹配 2 个元素 |
| TC-ROOM-00003 | 详情弹窗 - 强制关闭完整闭环 | webkit | ❌ FAIL | strict mode violation: getByText('确认强制关闭') 匹配 2 个元素 |
| TC-ROOM-00004 | XSS 防护 - 标题注入 | chromium/firefox/webkit | ✅ PASS | - |
| TC-ROOM-00005 | 房间统计数据展示 | chromium/firefox/webkit | ✅ PASS | - |

**统计**: 9 PASS / 6 FAIL / 0 SKIP

## 失败分析

### TC-ROOM-00002 (all browsers) — 分页断言失败
- **现象**: AI 断言"当前页码显示为第 2 页"失败，实际停在第 1 页
- **根因**: 测试环境中房间数量不足 10 条，无法触发分页（分页条件需要超过一页数据）
- **修复方向**: 
  1. 在 `beforeAll` 中批量创建 >10 个房间，确保分页存在
  2. 或将断言改为 "点击下一页后页码变化" 而非硬断言"第 2 页"
- **截图**: `test-results/WEB-TC-ROOM-...-TC-ROOM-00002-房间列表---筛选-分页-chromium/test-failed-1.png`

### TC-ROOM-00003 (all browsers) — Playwright strict mode violation
- **现象**: `locator.waitFor: Error: strict mode violation: getByText('确认强制关闭') resolved to 2 elements`
- **根因**: Ant Design Modal 的确认框同时渲染了 `.ant-modal-title` 和 `.ant-modal-confirm-title` 两个节点，都含文字"确认强制关闭？"
- **修复方向**: 将 `getByText('确认强制关闭')` 改为 `.first()` 或使用更精确的选择器 `page.locator('.ant-modal-title').getByText('确认强制关闭')`
- **截图**: `test-results/WEB-TC-ROOM-...-TC-ROOM-00003-详情弹窗---强制关闭完整闭环-chromium/test-failed-1.png`
