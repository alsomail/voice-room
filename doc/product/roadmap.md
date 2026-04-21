# 功能路线图 (Roadmap)

> 来源：原 `doc/product.md` 第一节 §1.3  
> 最后更新：2026-04-20

---

## Phase 0: MVP 基础设施 (当前阶段) — ✅ 基本完成

**Android 端 (C 端)**:
- [x] 手机号验证码一键登录（新用户自动注册）
- [x] 房间大厅（列表/创建/进入）
- [x] 房间内实时语音通信 (RTC 防腐层)
- [x] 麦位管理 (上麦/下麦)
- [x] 文本聊天

**App Server 端 (C 端业务后端)**:
- [x] 用户认证 API（手机号+验证码，JWT）
- [x] 房间管理 API（创建/销毁/列表/详情）
- [x] WebSocket 实时通信服务
- [x] 麦位状态管理与同步
- [x] 消息广播与持久化

**Admin Server 端 (B 端管理后端)**:
- [x] 管理员登录 API（账号密码，JWT）
- [x] 用户管理 API（列表/查询/封禁/编辑）
- [x] 房间管理 API（列表/监控/强制关闭）
- [x] 数据统计 API（DAU/在线人数/房间数）
- [x] 操作日志记录

**Web 端 (后台管理)**:
- [x] 管理员登录页面
- [x] 数据看板（实时在线/DAU/房间数）
- [x] 用户管理（查询/封禁/编辑）
- [x] 房间监控（实时列表/强制关闭）

---

## Phase 0.5: 交互壳体与基础体验 — 🟡 进行中

> 虽然 Phase 0 的代码已完成，但 Android 端缺少完整的用户交互壳（Splash/主页Tab/房间UI整合），Web端缺少部分基础管理能力。

**Android 端**:
- [ ] Splash 启动页（品牌 Logo + 动画）
- [ ] 完整主页框架（三Tab：房间列表 / IM消息 / 我的）
- [ ] 房间交互完整界面（弹幕动画、上麦互动、礼物面板占位）
- [ ] 个人中心页面
- [x] 中东风格视觉主题（黑金配色/RTL）— T-30018 ✅

**Web 端**:
- [ ] 数据看板增强（趋势图完善、告警阈值）
- [ ] 批量操作能力
- [ ] 操作日志增强

---

## Phase 1: 核心营收闭环 (预计 2 个月)

> **执行顺序**：E-07 礼物闭环 + E-07.5 埋点基建 并行→ E-10 房间治理 → E-08 真支付 → E-09 贵族

### E-07 虚拟礼物与钱包闭环 MVP 🟡进行中
详见 [phase1_gift_economy.md](./phase1_gift_economy.md)

**Android**: 钱包页 / 礼物面板 / 接收者选择 / 赠礼客户端 / 三层特效 / 余额不足引导 / 榜单页
**App Server**: 钱包 schema + WS / 礼物配置 + 列表 API / SendGift 强事务 / 榜单
S
**Admin Server**: 手动调整余额 / 礼物 CRUD
**Web**: 余额调整弹窗 + 礼物管理页

### E-07.5 埋点与观测性基建 🟡设计中
详见 [phase1_observability.md](./phase1_observability.md)

**Android**: Sentry 防腐层 + Analytics SDK + 核心事件埋点 + 隐私弹窗
**App Server**: 事件接收 API + WS ReportEvent 信令 + PostgreSQL 分区表
**Admin Server**: 用户行为查询 API
**Web**: 用户详情页“行为流” Tab

> ⚠️ **关键**：E-07 上线前 E-07.5 必须完成，否则营收无数据验证。

### E-10 房间主权与管理员体系 🟡设计中
详见 [phase1_room_governance.md](./phase1_room_governance.md)

**Android**: 创建房间升级 / 密码房进房 / 观众席 BottomSheet / 用户操作菜商 / 打踢禁麦禁言 / 管理员徽章 / 被踢/被禁弹窗 / 公告栏
**App Server**: rooms 表扩字段 + KickUser/MuteUser/TransferAdmin/ForceTakeMic WS 信令 + 审计表
**Admin Server**: 房间治理日志查询
**Web**: 房间治理日志查询页

### E-08 Google Play 真支付 🔴 待开发
**Android**: Billing Library 接入 / 充值页 / 订单状态
**App Server**: 订单表 / 回调验签 / 充值入账
**Admin Server**: 订单查询与对账 / 手动补单
**Web**: 订单查询页 / 财务报表

### E-09 贵族体系 🔴 待开发
**Android**: 贵族购买页 / 特权说明 / 进场特效
**App Server**: 贵族级别配置 / 权限校验 / 过期推送
**Web**: 贵族配置管理

---

## Phase 2: 社交裂变 (预计 3-4 个月)

**Android 端**:
- [ ] 家族/公会系统
- [ ] 好友/关注/粉丝
- [ ] 1v1 私聊房
- [ ] 动态广场
- [ ] 邀请返利

**App Server 端**:
- [ ] 家族/公会管理
- [ ] 社交关系链
- [ ] 1v1 房间匹配
- [ ] 动态发布与审核

**Admin Server 端**:
- [ ] 家族/公会审核管理 API
- [ ] 用户关系图谱查询 API
- [ ] 动态内容审核 API
- [ ] 邀请数据统计 API

**Web 端**:
- [ ] 家族/公会审核管理
- [ ] 用户关系图谱查询
- [ ] 动态内容审核
- [ ] 邀请数据统计

---

## Phase 3: 高级运营 (预计 5-6 个月)

**Android 端**:
- [ ] 官方赛事/PK 系统
- [ ] 守护神系统
- [ ] 房间 NFT 装扮
- [ ] AI 虚拟主播
- [ ] 节日活动页面

**App Server 端**:
- [ ] 赛事/PK 逻辑与排行
- [ ] 守护神绑定与特权
- [ ] NFT 资产管理

**Admin Server 端**:
- [ ] 赛事创建与管理 API
- [ ] 活动配置系统 API
- [ ] 推送消息管理 API
- [ ] AB 测试平台 API

**Web 端**:
- [ ] 赛事创建与管理
- [ ] 活动配置系统（模板化）
- [ ] 推送消息管理
- [ ] 数据分析大盘（用户画像/留存/ARPU）
