> 当前状态机：负责人 [E2E] | 状态 [待回归] | 修复轮次 [1/5]

# TC-GIFT WEB - 礼物管理 回归报告

**执行时间**: 2026-04-28 15:44 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1, Midscene AI)  
**关联任务**: T-0000P (Midscene env 注入)

## 测试结果

| 用例 ID | 用例名称 | 浏览器 | 结果 | 错误概要 |
|---------|---------|--------|------|---------|
| TC-GIFT-00001 | 列表 + 筛选 | chromium/firefox/webkit | ✅ PASS | - |
| TC-GIFT-00002 | 新增礼物 + 图片白名单 + CRUD | chromium | ❌ FAIL | Test timeout (180000ms exceeded) |
| TC-GIFT-00002 | 新增礼物 + 图片白名单 + CRUD | firefox | ❌ FAIL | AI assertion: "该行状态列变为'已下架'" 失败 — 实际 UI 使用 toggle 开关而非文字 |
| TC-GIFT-00002 | 新增礼物 + 图片白名单 + CRUD | webkit | ✅ PASS | - |

**统计**: 4 PASS / 2 FAIL / 0 SKIP

## 失败分析

### TC-GIFT-00002 (chromium) — 超时
- **现象**: Test timeout of 180000ms exceeded
- **根因**: Midscene AI 动作序列过长，测试未在 180s 内完成
- **修复方向**: 增加 `timeout` 配置或拆分用例步骤

### TC-GIFT-00002 (firefox) — AI 断言失败
- **现象**: AI 断言"该行状态列变为'已下架'"失败
- **根因**: UI 礼物状态使用 toggle 开关（灰色/绿色），无文字"已下架"/"已上架"
- **修复方向**: 更新 AI 断言为 "礼物行的状态开关处于关闭（灰色）状态"

**截图**: `test-results/WEB-TC-GIFT-TC-GIFT-WEB---礼物管理-TC-GIFT-00002-新增礼物-图片白名单-CRUD-chromium/test-failed-1.png`  
**截图**: `test-results/WEB-TC-GIFT-TC-GIFT-WEB---礼物管理-TC-GIFT-00002-新增礼物-图片白名单-CRUD-firefox/test-failed-1.png`
