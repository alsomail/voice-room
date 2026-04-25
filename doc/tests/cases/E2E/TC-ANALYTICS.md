# 测试套件：ANALYTICS 埋点上报与后台查看跨端闭环（E2E）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖闭环：Android 业务事件触发 → EventReportClient 上报 → AppServer 落库 → AdminServer 查询 → Web 行为流 Tab 可见。

---

## TC-ANALYTICS-00001：送礼全链路埋点 - Android → DB → Web 行为流
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. Android：U1 已登录、full 同意模式、WS 在线、钻石余额充足。
2. Android：U2 在房间 R1 麦位 1（接收者）。
3. Web：super_admin 登录，进入 U1 详情页 → "行为流"Tab。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | U1 点击房间内 🎁 → 选 520 礼物 → 点"送出" | 客户端 `track('click_gift_panel_open')` + `track('gift_send_click', {gift_id,count})` |
| 2 | `AppServer` | 服务端收到 SendGift 成功后 | U1 客户端额外 `track('gift_send_success', {gift_id,count,room_id})` |
| 3 | `Android` | 节流器 8 条或 2min 内 flush | WS `ReportEvent` 批量上报，收到 `EventReportAck {received:N, rejected_indices:[]}` |
| 4 | `DB` | `SELECT event_name FROM events_{today} WHERE user_id=U1 AND server_ts > 60s 前` | 含 `click_gift_panel_open`、`gift_send_click`、`gift_send_success` |
| 5 | `Web` | 行为流 Tab 刷新（或等待轮询） | 10s 内新增 3 条事件出现在列表顶部 |
| 6 | `Web` | 展开 `gift_send_success` JSON | properties 含 `gift_id=520礼物id`、`count=1`、`room_id=R1`，server_ts 与客户端相近 |
| 7 | `Web` | 检查 properties 敏感字段 | 不含 `phone`、`jwt` |

**【数据清理】**
- 清本用例事件。

---

## TC-ANALYTICS-00002：离线补传 + 进程重启持久化
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. Android：U1 已登录、full 模式。
2. Web：admin 可访问 U1 行为流页面。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 打开 Airplane Mode | WS 断开 |
| 2 | `Android` | 快速点击 App 20 个可追踪按钮 | 20 个事件进入 Room `event_queue` 表（断网暂存） |
| 3 | `Android` | 强制 kill 进程并冷启动 | 队列保留，队列长度仍为 20 |
| 4 | `Android` | 关闭 Airplane Mode 等待 WS 重连 | 30s 内队列清空，全部 20 条成功上报（优先 WS 通道） |
| 5 | `DB` | 统计 U1 最近 5 分钟事件数量 | = 20（恰好） |
| 6 | `Web` | 行为流刷新 | 新增的 20 条事件全部可见，按 server_ts DESC |

**【数据清理】**
- 无。

---

## TC-ANALYTICS-00003：隐私同意切换对上报流的端到端影响
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. Android：全新安装；U1 已登录。
2. Web：super_admin。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | Splash 后弹隐私弹窗 → 点"仅 Crash" | 进入主页 |
| 2 | `Android` | 在 App 内进行 30 次交互 2 分钟 | 无事件上报 |
| 3 | `DB` | U1 最近 2min 事件 | 0 行 |
| 4 | `Android` | 触发一次模拟 Crash 后重启 | Sentry 收到 Crash 事件（豁免同意） |
| 5 | `DB` | events 表 | 仍为 0 行（crash 上报到 Sentry，不走 events） |
| 6 | `Android` | 设置页切换到"开启完整分析" | consent_mode=full |
| 7 | `Android` | 继续 30 次交互触发事件 | 事件正常入队并上报 |
| 8 | `Web` | 5 分钟后查询 U1 行为流 | 仅 step 7 后的事件可见，step 2 的事件无 |

**【数据清理】**
- 清 App 数据；清本用例事件。
