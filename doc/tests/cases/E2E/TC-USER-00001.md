# 测试套件：E2E 端到端 - 封禁用户闭环（Web + AppServer + Android + DB + Redis）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

---

## TC-USER-00009：Web 封禁 → Android 被踢 → Web 状态刷新 完整闭环
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 Android 在线，在房间 R1 内麦位 3 上；R1 内还有 U2/U3。
2. operator 已登录 AdminWeb，打开 U1 的 UserDetailDrawer。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击"封禁"按钮 → 在 BanModal 填写 类型=永久 原因="刷屏辱骂" → 确认 → 二次 Confirm 确认 | 按钮 Loading，发起 POST `/api/v1/admin/users/{U1}/ban` |
| 2 | `AdminServer` | 后端日志 | 返回 200 code=0 |
| 3 | `DB` | `SELECT status FROM users WHERE id={U1}` | `banned` |
| 4 | `DB` | admin_logs 最新 | action=`user_ban`，target_id=U1，detail 含 "刷屏辱骂" |
| 5 | `Redis` | MONITOR | PUBLISH `admin:events` 含 type=`ban_user` user_id=U1 |
| 6 | `AppServer` | 日志 | 收到 admin event，查找 U1 的活跃 WS 连接 |
| 7 | `Android(U1)` | 3 秒内 | 收到 BanNotice 消息弹出 Toast "您的账号已被封禁"；WS 被服务端关闭；自动清除本地 JWT；跳转 LoginScreen（清栈） |
| 8 | `Android(U2)` | 同一房间的 WS | 收到 `MicLeft{slot_index:3,user_id:U1}` + `UserLeft{user_id:U1}` 广播，麦位 3 显示为空 |
| 9 | `AdminWeb` | Drawer 内状态 | 自动刷新：状态 Tag 变红 `已封禁`；"封禁"按钮消失，"解封"按钮出现 |
| 10 | `AppServer` | U1 用原 token 调 GET /users/me | 401 或 403 |

**【数据清理】**
- 解封 U1 恢复状态。
