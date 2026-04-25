# 测试套件：GOVERNANCE 房间治理跨端闭环（E2E）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖闭环：Android 房主踢人 + 禁麦 → AppServer 审计 + WS 广播 → 被踢用户 UI 状态 + Web 治理日志实时可见。

---

## TC-GOVERNANCE-00001：房主踢人 E2E - Android × AppServer × DB × Web
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. Android 设备 A：房主 U_OWNER 已进入房间 R1。
2. Android 设备 B：观众 U_VICTIM 已进入房间 R1。
3. Web 浏览器：`super_admin` 已登录 `/governance/logs`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android (A)` | 打开观众席 → 点击 U_VICTIM → 选"踢出" → 选"辱骂" → 点击"确定" | 弹窗关闭，Toast "已踢出" |
| 2 | `AppServer` | 抓包验证 WS 消息 | 收到 `KickUser {room_id:R1, target_user_id:U_VICTIM, reason:"辱骂"}` |
| 3 | `Android (B)` | 观察 U_VICTIM 客户端 | 500ms 内弹出全屏 Dialog `Key('dialog_kicked')`，显示"你已被移出房间，原因：辱骂" |
| 4 | `Android (B)` | 点击"知道了" | 自动导航回大厅；R1 卡片"进入"按钮灰色 10min 倒计时 |
| 5 | `DB` | `SELECT reason, operator_user_id FROM room_kick_records WHERE target_user_id=U_VICTIM ORDER BY created_at DESC LIMIT 1` | reason=`辱骂`，operator_user_id=U_OWNER |
| 6 | `Redis` | `TTL kicked:R1:U_VICTIM` | 在 595~600 秒之间 |
| 7 | `Web` | 治理日志页刷新（或自动轮询） | 列表首行新增一条 type=kick、target=U_VICTIM、operator=U_OWNER、reason=辱骂 |
| 8 | `Android (B)` | 立即尝试重新进入 R1 | 点"进入"显示"冷却中 xxx 秒"，无网络请求 |
| 9 | `Android (B)` | 跳过冷却后进入 | 后端返回 `KICKED_COOLDOWN` 时 Toast 提示；冷却结束后进入成功 |

**【数据清理】**
- DEL Redis `kicked:R1:U_VICTIM`。
- 清理 room_kick_records 本用例行。

---

## TC-GOVERNANCE-00002：管理员禁麦强制下麦 E2E + Web 实时审计
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. Android 设备 A：管理员 U_ADMIN（admin_user_id=U_ADMIN）在 R1。
2. Android 设备 B：U_TARGET 在 R1 且在麦位 slot=2。
3. Web：`super_admin` 登录并停留在 `/governance/logs?type=mute`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android (A)` | 点击 U_TARGET 头像 → 菜单"禁麦" → 选"5 分钟" → 确认 | 请求发出，Toast "已禁麦" |
| 2 | `Android (B)` | U_TARGET UI | Toast "你已被禁麦 5 分钟"；底部 Chip `Key('mute_countdown')` 显示 5:00；麦位 slot=2 自动空闲 |
| 3 | `AppServer` | RTC 推流检查 | U_TARGET 推流已被服务端中断 |
| 4 | `DB` | `SELECT type,duration_sec,operator_user_id FROM room_mute_records ORDER BY id DESC LIMIT 1` | type=`mic`，duration_sec=300，operator=U_ADMIN |
| 5 | `Redis` | `TTL mic_muted:R1:U_TARGET` | 295~300 |
| 6 | `Web` | 列表顶部 | 新增 1 行 type=禁麦、duration=5min、target=U_TARGET |
| 7 | `Android (B)` | 尝试点其他空麦位 "+" | 灰色禁用；Toast "你已被禁麦" |
| 8 | `Android (B)` | 尝试送礼给房主 | 成功（送礼不受禁麦影响） |
| 9 | `Android (A)` | 300s 后观察 Chip | 自动消失，麦位可用 |

**【数据清理】**
- DEL `mic_muted:R1:U_TARGET`；清理 room_mute_records 本行。

---

## TC-GOVERNANCE-00003：房主转移管理员 E2E - 徽章实时刷新
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. Android 设备 A：房主 U_OWNER 在 R1（无管理员）。
2. Android 设备 B：U2 在 R1。
3. Android 设备 C：U3（普通观众）在 R1。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android (A)` | 点 U2 → 菜单"任命管理员" → 确认 | 请求发出，Toast "已任命管理员" |
| 2 | `AppServer` | 广播 `AdminChanged {admin_user_id:U2, previous_admin_id:null}` | 房间所有成员的 WS 接收 |
| 3 | `Android (B)` | U2 自己 UI | 500ms 内观众席/麦位/弹幕昵称旁显示 🛡️ 金色盾牌徽章 |
| 4 | `Android (C)` | U3 UI | 看到 U2 昵称旁徽章同步渲染 |
| 5 | `DB` | `SELECT admin_user_id FROM rooms WHERE id=R1` | U2 |
| 6 | `Android (A)` | 再次点 U3 → 菜单"任命管理员" → 确认 | 广播 `AdminChanged {admin_user_id:U3, previous_admin_id:U2}` |
| 7 | `Android (B)` | 500ms 内 U2 徽章消失，U3 徽章出现 | 全端同步 |
| 8 | `Android (A)` | 点 U3 → 菜单"卸任管理员" → 二次确认 | 广播 AdminChanged admin_user_id=null |
| 9 | `DB` | admin_user_id | NULL |

**【数据清理】**
- 无。
