# 测试套件：E2E 端到端 - 强制关闭房间闭环（Web + AppServer + Android + DB + Redis）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

---

## TC-ROOM-00001：Web 强制关闭 → App 被动退出房间 闭环
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 active，owner=U_owner，内有 U1/U2/U3（Android 在线，WS 活跃）。
2. operator 登录 AdminWeb，打开 R1 的 RoomDetailDrawer。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击红色"强制关闭"按钮 | 弹出 Modal.confirm "确定强制关闭该房间？" |
| 2 | `AdminWeb` | 点击"确认" | 发起 DELETE `/api/v1/admin/rooms/{R1}` |
| 3 | `AdminServer` | 日志 | 200 code=0 |
| 4 | `DB` | `SELECT status FROM rooms WHERE id={R1}` | `closed` |
| 5 | `DB` | admin_logs 最新 | action=`room_force_close`，target_id=R1 |
| 6 | `Redis` | MONITOR | PUBLISH `admin:events` type=`close_room` room_id=R1 |
| 7 | `AppServer` | 日志 | 收到 admin event，广播 RoomClosed + 清理房间内存态 |
| 8 | `Android(U1/U2/U3)` | 各自 WS | 3 秒内收到 `{"type":"RoomClosed","payload":{"room_id":"{R1}"}}` |
| 9 | `Android(U1/U2/U3)` | UI | 弹出 Dialog "房间已被管理员关闭"，确认后回到大厅页 |
| 10 | `Android(U1)` | 大厅下拉刷新 | 房间列表中不再出现 R1 |
| 11 | `AdminWeb` | Drawer 状态 | 房间状态 Tag 变为红色 `已关闭`；"强制关闭"按钮置灰；列表中 R1 行状态同步 |
| 12 | `AdminWeb` | 对已 closed 的 R1 再次 DELETE | 409 code=40901 |

**【数据清理】**
- 无（保留 closed 状态）。
