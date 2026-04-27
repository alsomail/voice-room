# TC-ROOM API — 房间 测试报告

> 当前状态机：负责人 E2E | 状态 ✅ PASS | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (AppServer :3000 + AdminServer :3001 + Postgres + Redis)  
**总计**：12 通过 / 0 失败 / 0 阻塞

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-ROOM-00001 | 创建房间 201 | ✅ PASS |
| TC-ROOM-00002 | 标题长度边界 0/1/30/31 | ✅ PASS |
| TC-ROOM-00003 | room_type 枚举 + 密码字段 | ✅ PASS |
| TC-ROOM-00004 | 同用户并发创建仅一成功 | ✅ PASS |
| TC-ROOM-00005 | 未登录 / Token 过期 | ✅ PASS |
| TC-ROOM-00006 | 列表 热度降序 + 分页 @prod-safe | ✅ PASS |
| TC-ROOM-00007 | 已关闭/软删除房间不可见 | ✅ PASS |
| TC-ROOM-00008 | 详情 合法/非法/不存在 @prod-safe | ✅ PASS |
| TC-ROOM-00009 | 关闭房间 权限 + 状态机 | ✅ PASS |
| TC-ROOM-00010 | Admin 列表 筛选 + RBAC | ✅ PASS |
| TC-ROOM-00011 | Admin 详情 closed 可见 / 软删 404 | ✅ PASS |
| TC-ROOM-00012 | Admin 强制关闭 + 审计 | ✅ PASS |

---

### 🛠️ TDD 修复记录 (Round 3/5)

- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：房间详情返回缺少 `cover_url`, `category`, `announcement`, `admin_user_id` 字段
- **根本原因 (Root Cause)**：`app/server/src/modules/room/repository.rs` SELECT 查询未包含上述列
- **修复方案 (Solution)**：
  - `app/server/src/modules/room/repository.rs`: 在 SELECT 语句中补充 `cover_url, category, announcement, admin_user_id` 列
