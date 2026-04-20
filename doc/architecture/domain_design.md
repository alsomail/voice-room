# 4. 业务域拆分与扩展策略

## 4.1 核心 bounded context

| 领域 | 职责 | 典型实体 |
| --- | --- | --- |
| **Auth** | 登录、JWT、刷新、会话绑定、设备校验 | UserSession, AccessToken |
| **User** | 用户资料、等级、头像、在线态 | UserProfile, UserStatus |
| **Room** | 房间生命周期、房间配置、成员管理 | Room, RoomMember |
| **Seat** | 麦位状态、申请上麦、抱下麦、锁麦 | Seat, SeatAssignment |
| **RTC** | RTC Token、频道映射、媒体状态 | RtcChannel, RtcSession |
| **Wallet** | 金币余额、冻结、扣费、加款 | Wallet, WalletLedger |
| **Gift** | 礼物定义、送礼、房间广播 | GiftOrder, GiftCatalog |
| **Billing** | 收益、分成、对账、流水 | Bill, IncomeStatement |
| **Moderation** | 敏感词、封禁、踢人、风控 | BanRecord, RiskDecision |
| **Notification** | 系统通知、站内消息、Push | NotificationTask |
| **Admin** | 后台运营、房间巡检、配置发布 | AdminAction |

## 4.2 可横向扩展业务模块

以下模块必须作为独立业务域演进，不得直接耦合进 Room/Gift/User 的实现：

- Family
- CP
- VIP / Noble
- Backpack
- Mini Game

**扩展原则：**
1. 新模块拥有自己的 controller/service/repository/domain/dto。
2. 跨模块交互只通过 Application Service、Facade 或 Domain Events。
3. 禁止跨模块直接读写彼此私有表结构。
4. 新模块接入房间时，只暴露最小接口，如 `FamilyRoomFacade`、`GameRoomFacade`。

## 4.3 推荐的服务端模块结构

以 `gift` 模块为例：

```text
modules/gift/
├── controller.rs     # HTTP 路由与请求处理
├── ws_handler.rs     # 礼物相关 WS 帧处理
├── service.rs        # 业务逻辑与事务编排
├── repository.rs     # DB 读写与持久化
├── domain.rs         # 核心实体与业务规则
├── dto.rs            # 请求/响应结构体
├── event.rs          # 领域事件
├── mapper.rs         # DTO 与 Entity 映射
└── mod.rs            # 模块导出
```

## 4.4 Server 端 Rust 分层规范

- **Controller / WS Handler**
  - 接收请求，校验 DTO。
  - 提取 AuthContext，校验 WS `msg_id`。
  - 调用 Service，输出统一返回体。
- **Service**
  - 执行业务用例，开启数据库事务。
  - 编排多个 Repository。
  - 触发广播和领域事件。
- **Repository**
  - SQLx 数据访问，状态仓储读写。
  - 不承担核心业务决策。
  - **强制约束：必须使用 `cargo sqlx prepare` 生成 `.sqlx/` 离线数据供 CI 编译检查，禁止在未提供离线模式下使用宏 `query!`。**
- **Domain**
  - 规则判断：是否可上麦、是否允许送礼、是否命中风控。
  - 值对象封装：RoomId、UserId、SeatNo、Money、MsgId。
