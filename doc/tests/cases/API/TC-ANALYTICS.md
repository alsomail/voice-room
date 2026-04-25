# 测试套件：ANALYTICS 埋点与观测性基建（API）

> **需求模糊点 (Ambiguity Notes)**：
> - T-00022 "单个 properties 超 8KB 截断后仍落库还是拒绝"——按 TDS 描述"截断并记日志"断言，即仍返回 200，但字段被截断并打标 `_truncated=true`，若实现为拒绝需反馈。

覆盖 Task：T-00022（events 表 + 分区 + HTTP 批量接收）、T-00023（WS ReportEvent 信令）、T-10015（用户行为查询 API）。

---

## TC-ANALYTICS-00001：HTTP 批量上报 - 登录前 device_id 路径 + 分区命中
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 数据库今日分区 `events_{yyyymmdd}` 已由定时任务建好。
2. 未登录客户端 device_id=`D-001`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/api/v1/events/batch` Body `{"events":[{"event_name":"app_launch","device_id":"D-001","session_id":"S-1","client_ts":1714000000,"properties":{"app_version":"1.0"}}]}` 无 Authorization | HTTP 202（或 200），`data.received=1, data.rejected_indices=[]` |
| 2 | `DB` | `SELECT user_id,event_name,server_ts FROM events_{yyyymmdd} WHERE device_id='D-001'` | 1 行，user_id IS NULL，event_name=`app_launch`，server_ts 为服务端时间 |
| 3 | `AppServer` | POST 100 条事件（批量） | 耗时 <200ms，received=100 |
| 4 | `AppServer` | POST 事件缺 device_id 且无 JWT | HTTP 400，code=`DEVICE_ID_REQUIRED` |
| 5 | `AppServer` | POST 单事件 properties 为 10KB JSON | HTTP 200，DB 写入该条 properties._truncated=true，properties 长度 ≤8KB |
| 6 | `AppServer` | POST Body 超 100 个事件 | HTTP 400，code=`BATCH_TOO_LARGE`（或按 TDS 截断前 100 条，以实现为准） |

**【数据清理】**
- DELETE FROM events_{yyyymmdd} WHERE device_id='D-001'。

---

## TC-ANALYTICS-00002：JWT user_id 覆盖客户端上报 user_id + 不一致告警
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. 用户 U1 登录获得 `TOKEN_U1`（user_id=U1）；device_id=`D-001`。
2. 日志可观测（读取 stdout/Loki）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST `/events/batch` Bearer TOKEN_U1 Body `user_id=U2`（伪造） | HTTP 200 |
| 2 | `DB` | 查该事件 user_id | 为 U1（以 JWT 为准） |
| 3 | `LOG` | 检查服务日志 | 含 WARN `event_user_id_mismatch jwt=U1 client=U2` |
| 4 | `AppServer` | POST 不带 user_id 字段，携带 TOKEN_U1 | DB user_id=U1（JWT 自动回填） |

**【数据清理】**
- 清理本用例事件行。

---

## TC-ANALYTICS-00003：WS ReportEvent - server_ts 覆盖 + 上限 100 + ACK
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 WS 已建立连接。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | U1 WS 发 `{"type":"ReportEvent","payload":{"events":[{"event_name":"click_gift","client_ts":1,"device_id":"D-001"}]}}` | 收到 `{"type":"EventReportAck","payload":{"received":1,"rejected_indices":[]}}` |
| 2 | `DB` | 查该事件 | server_ts >> 1（服务端当前时间，非 client_ts=1） |
| 3 | `AppServer` | WS 发 events 数量 150 | ACK `received=100, rejected_indices=[100,101,...149]`；DB 实际写入 100 行 |
| 4 | `AppServer` | WS 发 events 中某条 properties 非法 JSON | ACK 中该 index 进入 rejected_indices |
| 5 | `AppServer` | WS 发 events 中 user_id=U99 与 JWT U1 不符 | DB 写入 user_id=U1 + WARN 日志 |

**【数据清理】**
- 清理本用例事件行。

---

## TC-ANALYTICS-00004：分区任务补偿 + 跨日边界写入
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. 手动删除明日分区表；让定时任务处于失败态。
2. 可触发手动补偿任务 `cargo run --bin partition_check` 或 HTTP 内部端点。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 手动设置系统时间到明日 00:00:05；POST events | HTTP 500 或 503，写入失败（无目标分区） |
| 2 | `AppServer` | 运行补偿脚本 | 创建 `events_{tomorrow}` 成功 |
| 3 | `AppServer` | 再次 POST 同样事件 | HTTP 202；DB `events_{tomorrow}` 有行 |
| 4 | `AppServer` | 运行补偿脚本二次（幂等） | 不重复创建，无错误 |

**【数据清理】**
- 清理 events_{tomorrow} 中本用例行。

---

## TC-ANALYTICS-00005：Admin 用户行为查询 - 时间窗 + 多 event_name + RBAC
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. 用户 U1 近 24h 内有事件：`login_verify_success×1`、`gift_send_success×5`、`admin_ban_user×0`。
2. super_admin `TOKEN_SA`；普通 operator `TOKEN_OP`（无 analytics 查询权限）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminServer` | GET `/api/v1/admin/users/U1/events?from=now-24h&to=now&limit=100` Bearer TOKEN_SA | HTTP 200，items 6 条，按 server_ts DESC |
| 2 | `AdminServer` | `?event_name=gift_send_success,login_verify_success` | items 仅这两种，共 6 条 |
| 3 | `AdminServer` | `?limit=101` | HTTP 400，`LIMIT_EXCEEDED` |
| 4 | `AdminServer` | `?from=now-31d&to=now` | HTTP 400，`TIME_RANGE_EXCEEDED`（分区命中 30 天限制） |
| 5 | `AdminServer` | Bearer TOKEN_OP 查询 event_name=`admin_ban_user` | HTTP 403，code=`PERMISSION_DENIED`（`admin_*` 仅 super_admin 可查） |
| 6 | `AdminServer` | 响应时间 | 命中分区时 <300ms |
| 7 | `DB` | admin_logs 新增 1 行 action=`analytics_query_user_events` | 存在，detail 含 user_id=U1 |

**【数据清理】**
- 无（读操作）。

---

## TC-ANALYTICS-00006：敏感字段过滤防线（服务侧二次校验）
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Security`
- **回归级别**：`P1`

**【前置条件】**
1. U1 登录，TOKEN_U1 有效。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | POST events properties 含字段 `phone=+9660555`、`jwt=eyJ...` | HTTP 200 |
| 2 | `DB` | 查该 event properties | phone 字段被替换为 `***` 或从 JSON 中删除；jwt 同样脱敏 |
| 3 | `LOG` | 服务端告警日志 | 含 WARN `sensitive_field_stripped keys=phone,jwt` |

**【数据清理】**
- 清理本用例事件。
