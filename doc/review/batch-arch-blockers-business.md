# 全局代码审查报告：架构阻塞修复批次（业务侧 · T-00041 + T-00042 + T-00043 + T-00044）

> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [0/10]

---

## 0. 流转规则

- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由 [GlobalReview] 进行全局代码审查。
- [GlobalReview] 审查通过 → 修改负责人 [-] 状态 [✅ Passed]。
- [GlobalReview] 审查未通过 → 修改负责人 [TDD] 状态 [❌ Failed]，并将审查意见追加到文档下方。
- [TDD] 修复并自测后 → 状态改为负责人 [GlobalReview] 状态 [⏳ In Review]，触发下一轮复审。

---

## 1. 审查上下文

- **批次定位**：QA 战报反向拆出的 6 个架构阻塞 Task 中，**业务侧（AppServer）4 个**的合并审查批次。基建侧 2 个（T-0000P / T-0000Q）已在 [batch-arch-blockers-infra.md](./batch-arch-blockers-infra.md) 闭环 ✅ Passed，本批次不重复审。
- **包含任务**：
  - **T-00041**（模块3）：WS 心跳 30s 超时主动断开（ping/pong）。客户端 15s ping 保活不断开，35s 静默后服务端 Close(1000)；并发 1000 连接稳定性不回归。
  - **T-00042**（模块8）：Admin 强制断连广播事件（user_banned / room_closed）。Redis pub/sub 解耦，事件失败不影响 Admin 主流程；封禁 2s 内 WS Close(4003)，房间关闭 3s 内所有成员断开。
  - **T-00043**（模块3）：Chat 消息持久化 + REST 历史查询接口。新建 `chat_messages` 表（迁移须遵循 T-0000M 双迁移表约定 → `app/server/migrations/` + `_sqlx_app_migrations`）；`GET /rooms/:id/messages` 时间倒序分页。
  - **T-00044**（模块6）：礼物 REST 端点 `POST /api/v1/gifts/send`，复用 WS SendGift 事务；幂等性、错误码（40290/40402/40403）与 WS 一致。
- **关联 TDS**：
  - [T-00041](../tds/server/T-00041.md)（§5 已记录单 Task Round 1 GlobalReview 🟢）
  - [T-00042](../tds/server/T-00042.md)（§5 已记录 Round 2 GlobalReview ✅）
  - [T-00043](../tds/server/T-00043.md)（§5 已记录 Round 2 GlobalReview ✅）
  - [T-00044](../tds/server/T-00044.md)（§5 已记录 Round 3 GlobalReview ✅）
- **开始时间**：2026-04-29

---

## 2. 审查关切（架构级）

本批次核心是「QA 战报反向拆出的 4 个 AppServer 业务阻塞，是否在补齐时遵守既有架构契约 / 协议契约 / 数据契约」。

### 关切 ① — T-00041：WS 心跳超时窗口与并发稳定性
- 30s 超时阈值是否契合协议（客户端 15s ping → 服务端 30s 阈值留足 2 个 ping 窗口）？
- Close code=1000（Normal Closure）是否符合「服务端主动超时」语义（vs 1011/4xxx）？相关 frame reason 字段是否落库 / 落日志可追溯？
- last_heartbeat 写入与扫描 task 的并发竞态（spawn 漏挂、tick 抖动、锁中毒、shutdown drain）是否有测试兜底？1000 连接压测是否真跑了，不是 mock？

### 关切 ② — T-00042：Redis Pub/Sub 事件失败的故障域隔离
- Admin Server 发布 `user_banned` / `room_closed` 后，App Server 订阅失败（连接抖动、JSON 解析失败、用户离线）是否会反向阻塞 Admin 主流程（封禁 API、关房 API）？
- Close code=4003（私有协议）+ reason 字段是否在协议文档（doc/protocol）显式登记？多连接（同一用户多设备）是否全部断？
- 事件 payload 是否带 msg_id / timestamp，幂等去重 / 重复消费防护是否到位？

### 关切 ③ — T-00043：chat_messages 表迁移契约（强制 T-0000M）
- 迁移文件**必须**位于 `app/server/migrations/` 目录（不是 `db/migrations` 或共享目录），并通过 AppServer 的 `_sqlx_app_migrations` 跟踪表执行（不是默认 `_sqlx_migrations`）。这是 T-0000M ADR-0001 已闭环的强制契约。
- 索引设计：`(room_id, created_at DESC)` 复合索引是否到位以支撑分页倒序查询？外键 ON DELETE 策略（CASCADE / SET NULL / RESTRICT）是否与房间软删 / 用户注销策略一致？
- `GET /rooms/:id/messages` 的鉴权（必须是房间在线成员或历史成员？）、分页（cursor vs offset）、limit 软上限、`COUNT(*) OVER()` 性能是否在 Round 2 已收敛？
- 断线重连「拉全量历史」的边界（无上限？最近 N 条？基于 last_msg_id？）—— 是否会演变为大表全扫描？

### 关切 ④ — T-00044：HTTP / WS 双入口的事务/幂等/错误码一致性
- HTTP `POST /api/v1/gifts/send` 是否真复用了 WS `SendGift` 的事务函数（同一份 service 层），还是各自一份并发 / 锁 / 流水写入逻辑？后者是定时炸弹。
- 幂等键策略：是 `Idempotency-Key` header（HTTP 习惯）还是 `msg_id`（WS 习惯）？两边是否互通（HTTP 提交后用同一 msg_id 走 WS 是否命中同一行 gift_records）？
- 错误码：40290 / 40402 / 40403 在 HTTP 与 WS 是否完全等价（同义词、同 HTTP 状态码映射、同 ErrorCode enum）？
- 弱网场景的「降级可用」具体含义：是否在 HTTP 层做了 retry-safe 设计（POST 幂等 + 5xx 客户端可重试）？rollback / spawn 异步副作用是否有原子性保证？

### 关切 ⑤ — 跨 Task 一致性
- 4 个 Task 都涉及 WS 连接生命周期（T-00041 主动断、T-00042 被动断、T-00043 重连拉历史、T-00044 经 WS 推 GiftReceived）。这些路径是否有冲突？例如：
  - T-00042 强制断连（Close 4003）vs T-00041 心跳超时（Close 1000）—— 同一 task / channel 是否有竞态？
  - T-00043 重连拉历史的入口是否会被 T-00042 的封禁状态正确拦截（封禁用户不应能拉历史）？
- 配套文档（doc/arch/server/{ws,gift,chat,governance}/index、doc/arch/database、doc/product/index）是否都已在 DoD 阶段同步更新？

### 关切 ⑥ — 安全 / 测试覆盖红线
- 4 个 Task 是否都跑通真 DB / 真 Redis 集成测试（不是纯 mock）？
- 是否引入硬编码凭据 / 真实测试用户密码 / 高熵 secret？
- 测试覆盖是否对得起「DoD」标签（不是 happy path only）？

---

## 2. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】

**@GlobalReview**：等待 global-code-reviewer 子代理执行架构级审查并填写本节。

---
