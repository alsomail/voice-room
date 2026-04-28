> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-CHAT API - 公屏聊天 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)  
**关联任务**: T-00043 (Chat 消息持久化)

## 测试结果

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-CHAT-00001 | SendMessage 成功 + 广播 | ⏭️ SKIP-KNOWN | - |
| TC-CHAT-00002 | 内容长度边界 0/1/500/501 | ✅ PASS | 29ms |
| TC-CHAT-00003 | 敏感词过滤 / XSS | ✅ PASS | 55ms |
| TC-CHAT-00004 | CHAT_MUTED 禁言 | ⏭️ SKIP-KNOWN | - |
| TC-CHAT-00005 | 历史记录分页查询 | ✅ PASS | ~20ms |

**统计**: 3 PASS / 0 FAIL / 2 SKIP (× 3 browsers = 9 PASS / 0 FAIL / 6 SKIP)

**跳过原因**: TC-CHAT-00001 需要 `E2E_USER_B_TOKEN`；TC-CHAT-00004 需要 `E2E_MUTED_TOKEN`（seed 未生成）。SKIP-KNOWN。
