# Phase 2 - 家族 / 公会系统 (E-11)

> **版本**: v1.0
> **创建日期**: 2026-05-09
> **负责人**: PM Agent
> **对应 Epic**: E-11 家族 / 公会系统
> **前置 Epic**: E-07 钱包闭环（✅ Done） / E-10 房间治理（✅ Done） / E-08 真支付（设计中）
> **状态**: 🟡 设计中（Phase 2 启动）

---

## 1. 战略定位

### 1.1 为什么是 Phase 2 第一个 Epic
经过 Phase 1 的营收闭环（礼物 + 真支付 + 贵族），用户消费心智已建立，但**留存与组织化运营**问题暴露：
- **Yalla 家族贡献占总营收 40%+**（详见 [competitors.md](./competitors.md)）
- **Ahlan 沙特用户家族归属感最强**：女性家族长比例 50%+，家族即"线上沙龙"
- **公会长 = 中型主播经纪人**：他们带来"批量主播"，是平台扩张的杠杆点

### 1.2 家族 vs 公会
本 Epic 统称"家族"（Family）— 在 MENA 地区"家族"概念远比"公会"具有情感号召力。
但产品形态融合两者特性：
- **家族**：情感归属（Yalla / Ahlan 模型）→ 默认形态
- **公会**：经纪人结构（YoHo / Mico 模型）→ 通过"贡献分润"特性体现

### 1.3 核心收益模型
| 角色 | 价值 |
|------|------|
| **家族长（Owner）** | 获得家族总贡献分润（10-20%）+ 专属徽章 + 房间装扮折扣 |
| **管理员（最多 5 位）** | 协助治理 + 小额分润 |
| **核心成员（贡献榜 Top 10）** | 月度专属奖励（钻石/贵族体验卡） |
| **普通成员** | 家族归属感 + 家族房免门槛 + 家族聊天 |
| **平台** | 留存提升 / GMV 提升 / 新用户裂变（家族邀请） |

### 1.4 与 E-10 治理的关系
- 房间治理（E-10）= "**单房间内**"的房主权
- 家族（E-11）= "**跨房间持久组织**"
- 两者**正交**：家族长不自动获得房间管理权；家族成员仍需在房间走 E-10 治理流程
- **特例**：家族房（room.family_id 关联）→ 家族管理员自动继承房间管理权

---

## 2. 范围边界

### 2.1 In Scope（MVP）
| 领域 | 内容 |
|------|------|
| 创建家族 | 钻石门槛（10万）+ 等级 ≥ 10 + 唯一名称（5-15 字） + 家徽 9 选 1 |
| 加入流程 | 申请 → 家族长 / 管理员审批；同时只能持有一个家族；退出 30 天冷却 |
| 家族信息 | 名称 / 家徽 / 公告 / 等级（基于成员总贡献升级）/ 上限人数（按等级）/ 标签 |
| 角色 | Owner（1）/ Admin（≤5）/ Elite（贡献榜 Top10 自动）/ Member |
| 家族贡献 | Member 在任意房间送礼自动累计到家族总贡献（Redis ZSet 周/月榜） |
| 家族成员管理 | 转让 Owner / 任免 Admin / 踢出 Member / 解散家族 |
| 家族 IM | 家族群聊（复用 WS，频道 `family:{id}`）+ 公告推送 |
| 家族房 | 创建房间时可绑定 family_id；家族房成员折扣 / 优先入房 |
| 家族贡献榜 | 家族内月榜（Top10 家族获奖） + 全服家族 Top100 |
| Admin | 家族审核（可疑名称黄条）/ 强制解散 / 数据查询 |
| Web | 家族审核页 + 家族数据看板 |
| Android | 家族 Tab（首页第 4 Tab 或集成到 IM）+ 列表 + 详情 + 创建 + 申请 + 聊天 + 贡献榜 |

### 2.2 Out of Scope（延后）
| 领域 | 延后到 | 原因 |
|------|--------|------|
| 家族 PK / 联赛 | Phase 3 (E-19) | 复杂赛事系统 |
| 跨家族联盟 | Phase 3 | 业务复杂度高 |
| 家族钱包共池 | 后续专项 | 财务合规风险高 |
| 家族应援 / 守护神 | Phase 3 | 依赖更多关系链 |
| 邀请返利 | E-13（独立 Epic）| 涉及佣金体系 |

---

## 3. 等级与成员上限

| 家族等级 | 累计贡献（钻石） | 成员上限 | 管理员上限 | 解锁能力 |
|:--:|:--:|:--:|:--:|:--|
| LV1 | 0 | 30 | 1 | 创建即默认 |
| LV2 | 100,000 | 50 | 2 | 自定义家徽 |
| LV3 | 500,000 | 80 | 3 | 家族公告推送 |
| LV4 | 2,000,000 | 120 | 4 | 家族房免密邀请链接 |
| LV5 | 10,000,000 | 200 | 5 | 家族专属皮肤 + 进场广播 |

> **降级规则**：MVP 不降级（只增不减），降级模型 Phase 3 评估。

---

## 4. 业务流程

### 4.1 创建家族
```
Android 用户 → 家族 Tab → "创建家族" → 表单（名称/家徽/标签/初始公告）
  → POST /api/v1/families { name, badge_id, tags, announcement }
  → Server 校验：钻石余额 ≥ 100000 + 用户等级 ≥ 10 + 名称唯一
  → 强事务：扣 100000 钻 + INSERT families + INSERT family_members(role=owner) + 写流水
  → 返回 family_id → Android 跳家族详情页
```

### 4.2 申请加入与审批
```
Android 用户在家族详情点"申请加入" → POST /api/v1/families/:id/applications { message }
  → Server 校验：未持有家族 + 不在退出冷却 + 未被该家族拉黑
  → 写 family_applications(state=PENDING) + WS 推送家族管理员
  → 家族管理员审批 PUT /applications/:id { action: approve|reject }
  → approve 时事务：申请→APPROVED + INSERT family_members(role=member) + 推送 FamilyChanged
```

### 4.3 家族贡献累计
```
任意用户在房间送礼（T-00020 SendGift）成功 → Server 检查 sender.family_id != null
  → Redis ZINCRBY family:contrib:{period}:{family_id} +diamond_amount
  → Redis ZINCRBY family:member_contrib:{period}:{family_id} {user_id} +diamond_amount
  → 累计达到下一等级阈值 → 触发家族升级事件 + 全家族 WS 通知
```

### 4.4 家族 IM
```
用户进入家族详情 → 自动 WS Subscribe family:{id}
  → 发消息：WS SendFamilyMessage { msg_id, family_id, content }
  → Server 校验成员资格 + 写 family_messages 表 + 广播 family:{id} 频道
  → 在线成员收到 FamilyMessageReceived
  → 离线成员通过历史 API 拉取
```

### 4.5 关键异常

| 场景 | 处理 |
|------|------|
| 钻石不足创建 | INSUFFICIENT_BALANCE → 跳充值 |
| 名称重复 | 40920 NAME_TAKEN → Toast |
| 名称违禁词 | 40921 NAME_FORBIDDEN → 高亮提示 |
| 申请被拒后短期重申请 | 黑名单期 7 天 |
| 家族解散时仍有未结算贡献 | 当期归并存入 family_archive 表 + 成员 history 留档 |
| 家族长账号被封 | super_admin 强制转让 Owner 给最早的 Admin |
| 转让 Owner 后立即被踢 | 禁止：转让操作必须连续 24h 才能再被新 Owner 操作（防恶意） |

---

## 5. 关键技术约束

1. **强事务**（红线 #2）：家族创建 / 转让 / 申请审批所有写多表操作必须事务化
2. **单一事实源**（红线 #1）：客户端家族成员列表严禁本地推断；管理员审批后由 `FamilyMemberChanged` WS 广播驱动
3. **WS 频道复用**：复用现有 connection 多频道订阅机制；避免为家族新建独立 WS 进程
4. **贡献榜 Redis 隔离**：家族贡献 Key 与个人榜（T-00021）独立 namespace 避免冲突
5. **审核与合规**：家族名称走"违禁词词库 + Server 端 reject"，避免 App Store 投诉
6. **配置化**：等级阈值、人数上限、创建门槛走 `config/business.toml`，运营可调

---

## 6. 验收指标

| 类别 | 指标 | 目标 |
|------|------|------|
| 功能 | 创建/加入/退出/转让/解散全流程跑通 | 100% |
| 性能 | 家族列表分页 P95 | < 200ms |
| 性能 | 家族贡献写入延迟 | < 100ms |
| 业务 | Phase 2 上线 30 天家族数 | ≥ 500 |
| 业务 | 家族成员 7 日留存比无家族用户 | +25% |
| 业务 | 家族贡献占总送礼 GMV | ≥ 30% |

---

## 7. 关联文档

- [Tasks 模块 12 - E-11 家族公会](../tasks/模块12-家族公会%20(E-11).md)
- [Android 设计稿 - T-30083 家族详情页](../design/android/T-30083.md)
- [phase1_room_governance.md](./phase1_room_governance.md) — E-10 治理对照
- [competitors.md §家族贡献](./competitors.md)
