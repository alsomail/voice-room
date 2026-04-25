# 测试套件：GOVERNANCE 房间主权与管理员体系（API）

> **需求模糊点 (Ambiguity Notes)**：
> - `T-00030` AdminChanged 广播字段 `previous_admin_id` 在 assign 且无旧管理员时取 null 还是省略，契约未明示，这里按 null 断言。
> - `T-00028` 冷却 Key 在目标用户被重新任命为该房间管理员后是否自动解除，未规定，本套件按"自动解除 + 可立即重进"断言（与产品语义对齐），若实现不同需反馈。

覆盖 Task：T-00024（rooms 扩字段 + 治理审计表）、T-00025（创建房间升级）、T-00026（密码房校验 + 锁定）、T-00027（观众席列表）、T-00028（KickUser + 10min 冷却）、T-00029（MuteUser/UnmuteUser + 双重拦截）、T-00030（TransferAdmin + ForceTakeMic/ForceLeaveMic）、T-10016（Admin 治理日志查询）。

---

## TC-GOVERNANCE-00001：创建房间升级 - 封面/分类/密码/公告 全字段校验
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 用户 U1 登录获得 `TOKEN_U1`，DB 无 U1 活跃房间。
2. 白名单封面集合已预置 8 张：`cover_01.png` ~ `cover_08.png`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/api/v1/rooms` Body `{"title":"情感夜谈","category":"emotion","cover_url":"cover_03.png","announcement":"欢迎新人","password":"888888"}` | HTTP 201，`data.has_password=true`，`data.category="emotion"` |
| 2 | `DB` | `SELECT category,cover_url,announcement,password_hash,admin_user_id FROM rooms WHERE id=data.room_id` | category=`emotion`，cover_url=`cover_03.png`，announcement=`欢迎新人`，password_hash 以 `$2` 开头（bcrypt），admin_user_id IS NULL |
| 3 | `AppServer` | POST `/api/v1/rooms` password=`12345`（5 位） | HTTP 400，code=`INVALID_PASSWORD_FORMAT` |
| 4 | `AppServer` | POST `/api/v1/rooms` password=`abcdef`（非数字） | HTTP 400，code=`INVALID_PASSWORD_FORMAT` |
| 5 | `AppServer` | POST `/api/v1/rooms` cover_url=`https://evil.com/x.png` | HTTP 400，code=`COVER_NOT_WHITELISTED` |
| 6 | `AppServer` | POST `/api/v1/rooms` announcement 长度=201 | HTTP 400，code=`ANNOUNCEMENT_TOO_LONG` |
| 7 | `AppServer` | POST `/api/v1/rooms` category=`unknown` | HTTP 400，code=`INVALID_CATEGORY` |

**【数据清理】**
- DELETE FROM rooms WHERE owner_id=U1 AND status='active'。

---

## TC-GOVERNANCE-00002：PATCH 房间 - 仅房主可改 + 广播 RoomInfoUpdated
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 房主 U1 已创建房间 R1；U2 已 JoinRoom R1；管理员 U3（admin_user_id=U3）已 JoinRoom。
2. U2/U3 均有活跃 WS 连接。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 发 PATCH `/api/v1/rooms/R1` Body `{"announcement":"新公告"}` | HTTP 403，code=`PERMISSION_DENIED` |
| 2 | `AppServer` | U3（管理员）发同请求 | HTTP 403，code=`PERMISSION_DENIED`（PATCH 仅房主） |
| 3 | `AppServer` | U1 发 PATCH Body `{"announcement":"新公告","category":"music"}` | HTTP 200 |
| 4 | `AppServer` | 观察 U2/U3 的 WS 收到消息 | 均收到 `{"type":"RoomInfoUpdated","payload":{"announcement":"新公告","category":"music"}}` |
| 5 | `DB` | `SELECT announcement,category FROM rooms WHERE id=R1` | announcement=`新公告`，category=`music` |
| 6 | `AppServer` | U1 发 PATCH announcement 长度=201 | HTTP 400，code=`ANNOUNCEMENT_TOO_LONG` |

**【数据清理】**
- 恢复 R1 原始 announcement/category 或关闭 R1。

---

## TC-GOVERNANCE-00003：密码房校验 + 5 次错误锁定 + token TTL
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. R1 为密码房，密码 `666666`；用户 U2 未在 R1 内。
2. 清空 Redis Key `pwd_fail:U2:R1`、`pwd_lock:U2:R1`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 POST `/api/v1/rooms/R1/verify-password` Body `{"password":"000000"}` | HTTP 401，code=`PASSWORD_WRONG`，`remaining_attempts=4` |
| 2 | `AppServer` | 再错 4 次（共 5 次）使用错误密码 | 第 5 次返回 401 code=`PASSWORD_LOCKED`，`locked_sec=1800` |
| 3 | `Redis` | `GET pwd_lock:U2:R1` 并 `TTL pwd_lock:U2:R1` | 值存在，TTL 在 1790~1800 之间 |
| 4 | `AppServer` | U2 第 6 次尝试（即使正确 666666） | HTTP 401，code=`PASSWORD_LOCKED` |
| 5 | `AppServer` | U3（未锁定用户）POST 正确密码 `666666` | HTTP 200，返回 `password_token`（JWT），claim `room_access=R1` |
| 6 | `AppServer` | U3 立即 WS JoinRoom，携带 `password_token` | JoinRoom 成功，收到房间快照 |
| 7 | `AppServer` | U3 等待 65s 再次 WS JoinRoom 复用同 token | 拒绝，返回 `TOKEN_EXPIRED` |
| 8 | `AppServer` | U3 WS JoinRoom R1 不携带 token | 拒绝，返回 `PASSWORD_REQUIRED` |
| 9 | `AppServer` | U2 对非密码房 R2 调用 verify-password | HTTP 400，code=`ROOM_NOT_PASSWORD_PROTECTED` |

**【数据清理】**
- DEL `pwd_fail:U2:R1` `pwd_lock:U2:R1`。

---

## TC-GOVERNANCE-00004：观众席列表 - 角色优先级 + 麦上置顶 + 性能
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. R1 房主 U1，管理员 U2（admin_user_id=U2），麦上用户 U3（slot=1）、U4（slot=2），观众 U5~U100 按 joined_at 递增顺序进入。
2. U3/U5 为麦上（U3 在麦，U5 为观众但前一秒还在麦——已下麦后成为最新观众）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/rooms/R1/members?page=1&limit=20` Bearer TOKEN_U5 | HTTP 200，响应时间 <150ms |
| 2 | `AppServer` | 断言返回字段 | items 长度=20；前 N 项 `mic_slot != null` 按 slot 升序；后续项按 joined_at DESC |
| 3 | `AppServer` | 断言 role | U1.role=`owner`，U2.role=`admin`，其他=`member` |
| 4 | `AppServer` | GET `?page=0&limit=20` | items 为空数组（不抛 500） |
| 5 | `AppServer` | GET `?page=99&limit=20`（超界） | items 为空数组 |
| 6 | `AppServer` | GET `?limit=101` | HTTP 400，code=`LIMIT_EXCEEDED` |
| 7 | `AppServer` | TRANSFER admin 到 U7 后再次 GET | U2.role=`member`，U7.role=`admin` |

**【数据清理】**
- 关闭 R1。

---

## TC-GOVERNANCE-00005：KickUser - 权限矩阵 + 10min 冷却 + 麦上联动
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 房主 U1，管理员 U2，麦上成员 U3（slot=1），观众 U4，普通成员 U5。
2. 清空 Redis `kicked:R1:*` 键。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U5 WS 发 `{"type":"KickUser","payload":{"room_id":"R1","target_user_id":"U4","reason":"spam"}}` | 仅发给 U5：`{"code":40301,"error":"PERMISSION_DENIED"}` |
| 2 | `AppServer` | U2 WS 发 KickUser target=U1 | 仅发给 U2：`code=40310, error=CANNOT_KICK_OWNER` |
| 3 | `AppServer` | U1 WS 发 KickUser target=U3 reason=`abuse` | 房间广播 `MicLeft {slot:1}` → `UserLeft {user_id:U3}`；U3 额外收 `UserKicked {reason:"abuse",remaining_sec:600}` 随后 WS 被服务端关闭 |
| 4 | `DB` | `SELECT reason,operator_user_id FROM room_kick_records WHERE room_id=R1 AND target_user_id=U3` | 1 行，reason=`abuse`，operator_user_id=U1 |
| 5 | `Redis` | `TTL kicked:R1:U3` | 在 595~600 之间 |
| 6 | `AppServer` | U3 立即 WS 重连发 JoinRoom R1 | `code=42911, error=KICKED_COOLDOWN, remaining_sec≈600` |
| 7 | `AppServer` | 模拟 3 个管理员（手动插入多个 admin）同时并发 KickUser target=U4 | 仅 1 次 INSERT room_kick_records，1 次 MicLeft/UserLeft 广播 |
| 8 | `Redis` | 将 `kicked:R1:U3` TTL 手动设为 1s，等待过期 | U3 再次 JoinRoom 成功 |

**【数据清理】**
- DEL `kicked:R1:*`；清理 room_kick_records 本次用例行。

---

## TC-GOVERNANCE-00006：MuteUser - 禁麦/禁言双重拦截 + TTL 自动解除
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 房主 U1，管理员 U2，麦上用户 U3（slot=1）。
2. 清空 `mic_muted:R1:*` `chat_muted:R1:*`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 发 `MuteUser target=U3 type=mic duration_sec=300` | 广播 `UserMuted {type:"mic",duration_sec:300}`；额外广播 `MicLeft {slot:1}`（因在麦强制下麦） |
| 2 | `Redis` | `TTL mic_muted:R1:U3` | 295~300 |
| 3 | `AppServer` | U3 发 `TakeMic slot=2` | 返回 `code=40306, error=MIC_MUTED` |
| 4 | `AppServer` | U3 发 SendGift（向 U1 送礼，余额充足） | 成功（送礼不受禁麦影响）|
| 5 | `AppServer` | U2 发 `MuteUser target=U3 type=chat duration_sec=60` | 广播 `UserMuted {type:"chat",duration_sec:60}` |
| 6 | `AppServer` | U3 发 SendMessage content=`hi` | 返回 `code=40305, error=CHAT_MUTED` |
| 7 | `AppServer` | U5（普通成员）发 UnmuteUser target=U3 type=mic | `code=40301, PERMISSION_DENIED` |
| 8 | `AppServer` | U1 发 UnmuteUser target=U3 type=mic | 广播 `UserMuted {type:"mic",duration_sec:0}`；`mic_muted:R1:U3` 被删除 |
| 9 | `AppServer` | U3 TakeMic slot=2 | 成功 |
| 10 | `AppServer` | duration_sec=59 或 duration_sec=86401 | HTTP/WS 返回 `INVALID_DURATION`（合法 60~86400） |

**【数据清理】**
- DEL `mic_muted:R1:*` `chat_muted:R1:*`；清理 room_mute_records 本用例行。

---

## TC-GOVERNANCE-00007：TransferAdmin + ForceTakeMic / ForceLeaveMic 权限闭环
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 房主 U1，无管理员；U2/U3 为普通成员在房间内；U4 在麦 slot=2。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U2 发 `TransferAdmin action=assign target=U3` | `code=40301, PERMISSION_DENIED`（仅房主） |
| 2 | `AppServer` | U1 发 `TransferAdmin action=assign target=U2` | 广播 `AdminChanged {admin_user_id:U2, previous_admin_id:null}` |
| 3 | `DB` | `SELECT admin_user_id FROM rooms WHERE id=R1` | U2 |
| 4 | `AppServer` | U1 再发 `TransferAdmin action=assign target=U3` | 广播 `AdminChanged {admin_user_id:U3, previous_admin_id:U2}`（旧管理员隐式卸任） |
| 5 | `AppServer` | U3（新管理员）发 `TransferAdmin action=assign target=U5` | `code=40301`（管理员不能再任命） |
| 6 | `AppServer` | U3 发 `ForceLeaveMic target=U4` | 广播 `MicLeft {slot:2, forced_by:U3}` |
| 7 | `AppServer` | U3 发 `ForceLeaveMic target=U4`（已不在麦） | `code=40411, MIC_NOT_FOUND` |
| 8 | `AppServer` | U1 发 `ForceTakeMic target=U5 slot=3`（U5 未禁麦） | 广播 `MicTaken {slot:3, user_id:U5, forced_by:U1}` |
| 9 | `AppServer` | U1 设 `mic_muted:R1:U6`，再 `ForceTakeMic target=U6 slot=4` | `code=40306, MIC_MUTED` |
| 10 | `AppServer` | U3 发 `TransferAdmin action=revoke target=U1` | `code=40312, CANNOT_REVOKE_OWNER`（管理员不能卸任房主，U1 本身也不是 admin） |
| 11 | `AppServer` | U1 发 `TransferAdmin action=revoke target=U3` | 广播 `AdminChanged {admin_user_id:null, previous_admin_id:U3}`；DB admin_user_id=NULL |

**【数据清理】**
- UPDATE rooms SET admin_user_id=NULL WHERE id=R1。

---

## TC-GOVERNANCE-00008：原子性 - TransferAdmin DB 失败不广播不改内存
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. R1 房主 U1，管理员 U2；U3 在房间内。
2. 测试框架可拦截 UPDATE rooms 抛 DB 错误（Mock）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | Mock DB 在 UPDATE rooms 时抛错；U1 发 `TransferAdmin assign target=U3` | 返回错误 `code=50000, INTERNAL_ERROR` |
| 2 | `AppServer` | 房间内 U1/U2/U3 未收到 `AdminChanged` 广播（等待 2s） | 无 AdminChanged 事件 |
| 3 | `DB` | `SELECT admin_user_id FROM rooms WHERE id=R1` | 仍为 U2（未变更） |
| 4 | `AppServer` | 取消 Mock，U1 重发 | 正常成功广播 |

**【数据清理】**
- 回滚 admin_user_id 到 NULL。

---

## TC-GOVERNANCE-00009：Admin 治理日志查询 - 过滤 + 性能 + CSV
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. admin_logs 表写入 1 万条历史治理记录；room_kick_records + room_mute_records 合计 5 万条。
2. super_admin Token `TOKEN_SA` 有效。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/governance/logs?type=kick&from=...&to=...&page=1&limit=50` Bearer TOKEN_SA | HTTP 200，items 全为 type=`kick`，响应时间 <300ms |
| 2 | `AdminServer` | `?type=mute&room_id=R1` | items 全为 mute 且 room_id=R1 |
| 3 | `AdminServer` | `?limit=101` | HTTP 400，code=`LIMIT_EXCEEDED` |
| 4 | `AdminServer` | `?export=csv`（当前筛选 3000 条） | 响应 Content-Type=`text/csv; charset=utf-8`，首 3 字节为 `EF BB BF`（UTF-8 BOM） |
| 5 | `DB` | `SELECT COUNT(*) FROM admin_logs WHERE action='governance_query' AND admin_id=SA` | 有新记录（查询被审计） |
| 6 | `AdminServer` | 普通 operator（无 RoomGovernanceView）调用 | HTTP 403，code=`PERMISSION_DENIED` |
| 7 | `AdminServer` | from=2 月前, to=now（时间窗 >30 天） | HTTP 400，code=`TIME_RANGE_TOO_LARGE` 或实现说明（按产品规约，本接口允许大时间窗时校验为 `<=90d`，严格按实现断言） |

**【数据清理】**
- 清理本次用例生成的 admin_logs 行。
