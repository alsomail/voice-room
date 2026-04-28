> 当前状态机：负责人 [E2E] | 状态 [✅ PASS] | 修复轮次 [2/5]

# TC-ROOM API - 房间管理 回归报告

**执行时间**: 2026-04-28 15:41 ~ 16:17  
**执行环境**: local (chromium + firefox + webkit, workers=1)

## 测试结果

| 用例 ID | 用例名称 | 结果 | 耗时 |
|---------|---------|------|------|
| TC-ROOM-00001 | 创建房间 201 | ✅ PASS | 36ms |
| TC-ROOM-00002 | 标题长度边界 0/1/30/31 | ✅ PASS | 138ms |
| TC-ROOM-00003 | room_type 枚举 + 密码字段 | ✅ PASS | 3ms |
| TC-ROOM-00004 | 同用户并发创建仅一成功 | ✅ PASS | 376ms |
| TC-ROOM-00005 | 未登录 / Token 过期 | ✅ PASS | 3ms |
| TC-ROOM-00006 | 列表 热度降序 + 分页 | ✅ PASS | 12ms |
| TC-ROOM-00007 | 已关闭/软删除房间不可见 | ✅ PASS | 10ms |
| TC-ROOM-00008 | 详情 合法/非法/不存在 | ✅ PASS | 71ms |
| TC-ROOM-00009 | 关闭房间 权限 + 状态机 | ✅ PASS | 43ms |
| TC-ROOM-00010 | Admin 列表 筛选 + RBAC | ✅ PASS | 5ms |
| TC-ROOM-00011 | Admin 详情 closed 可见 / 软删 404 | ✅ PASS | 1ms |
| TC-ROOM-00012 | Admin 强制关闭 + 审计 | ✅ PASS | 102ms |

**统计**: 12 PASS / 0 FAIL / 0 SKIP (× 3 browsers = 36 PASS / 0 FAIL / 0 SKIP)
