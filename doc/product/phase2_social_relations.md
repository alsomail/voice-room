# Phase 2 - 好友 / 关注 / 粉丝关系链 (E-12)

> **版本**: v1.0
> **创建日期**: 2026-05-09
> **负责人**: PM Agent
> **对应 Epic**: E-12 好友 / 关注 / 粉丝
> **前置 Epic**: E-04 房间核心（✅ Done） / E-07 钱包（✅ Done） / E-07.5 埋点（✅ Done）
> **可与之并行**: E-11 家族公会
> **状态**: 🟡 设计中

---

## 1. 战略定位

### 1.1 关系链 = 留存核武器
| 平台 | 无关系链次留存 | 有关系链（≥1 好友）次留存 |
|------|--------------|--------------------------|
| Yalla | 22% | **48%** |
| Mico | 18% | **41%** |
| YoHo | 25% | **52%** |

**结论**：建立关系链可让次留存翻倍，是 Phase 2 性价比最高的产品投入。

### 1.2 模型选型：单向关注 + 双向好友混合
| 模型 | 例 | 优点 | 缺点 |
|------|----|----|----|
| 纯单向关注（Twitter / TikTok） | 简单灵活 | 缺乏互动确认 |
| 纯双向好友（QQ / WeChat） | 强信任感 | 添加门槛高，初期增长慢 |
| **单向关注 + 双向好友（本方案）** | Yalla / Mico 验证 | 初期低门槛 + 后期强连接 |

**实现层级**：
- **关注（Follow）**：单向，无需对方同意，建立"我看 TA 动态"通道
- **粉丝（Fan）**：被关注方的反向视图
- **好友（Friend）**：互相关注 → 自动升级为好友（出现在好友列表 + 在线推送 + IM 入口）
- **特别关注（Star）**：限制 50 个，关注好友进房推送

### 1.3 与已有体系的关系
| 已有能力 | E-12 复用方式 |
|----------|--------------|
| WS 心跳与在线检测（T-00041） | 直接复用计算关注好友的"在线状态" |
| Redis 用户在线 Set（T-00046 类似机制） | 新增 `online_users` Set 用于查询 |
| IM 主页 Tab（T-30022） | 改造增"好友/关注"分组 |
| 用户资料卡 | 增"关注/粉丝"按钮 + 关系状态 |

---

## 2. 范围边界

### 2.1 In Scope
| 领域 | 内容 |
|------|------|
| 关注 / 取关 | POST `/follows/:user_id` / DELETE；唯一 `(follower_id, followed_id)` |
| 关注列表 / 粉丝列表 | 分页 + 在线状态 + 关系状态（互关/单关）|
| 好友列表 | 互关用户视图（虚拟，由两条 follow 关系派生）|
| 特别关注 | `is_starred BOOL` + 上限 50；进房推送 |
| 关系状态查询 | 资料卡显示"已关注 / 互相关注 / 未关注" |
| 在线状态 | Redis `online_users` Set + WS connect/disconnect 维护 |
| 好友进房推送 | WS 信令 `StarredFriendOnline / FriendJoinedRoom`（仅互相关注且 is_starred=true）|
| 黑名单 | POST `/users/:id/block`：相互不可关注 / 不可看资料 / 不可发消息 / 不进对方房间推荐 |
| 1v1 私聊 | 仅互关好友可发起；复用 WS（频道 `dm:{minUserId}_{maxUserId}`）+ 持久化 |
| 反骚扰限流 | 单用户日关注上限 200；陌生人 1v1 请求 5/日 |
| Admin | 关系图谱查询 + 异常关系告警（短期大量关注） |
| Web | 用户详情新增"关系" Tab |
| Android | 关注按钮（资料卡 / 用户气泡 / 房间用户菜单）+ 关注/粉丝/好友列表 + 1v1 入口 |

### 2.2 Out of Scope
| 领域 | 延后到 | 原因 |
|------|--------|------|
| 动态广场（关注 Feed） | E-14 | 独立 Epic，强依赖内容生态 |
| 邀请返利 | E-13 | 独立营收 Epic |
| 1v1 视频 | Phase 3 | RTC 视频通道 |
| 群组（非家族） | Phase 3 | 与家族功能重叠 |
| 站内信运营推送 | 后续 | 走推送通道而非 IM |

---

## 3. 业务流程

### 3.1 关注流程
```
Android 用户在资料卡点"关注" → POST /api/v1/follows/:targetId
  → Server 校验：① 不能关注自己 ② 不在黑名单 ③ 日关注上限 < 200
  → 强事务：INSERT follows + 推 follow_count 缓存
  → 检查反向关系（targetId follow me?）→ 是 → 双方升级为好友 → 推送 FriendCreated
  → 推送 followed 事件给 target（红点）
  → Android：按钮"已关注 / 互相关注"
```

### 3.2 在线状态维护
```
WS connect → SADD online_users {user_id}
WS disconnect / 心跳超时 → SREM online_users {user_id}
查询朋友在线 → SMEMBERS 取交集 follows × online_users
```

### 3.3 好友进房推送
```
用户 JoinRoom → Server 查询 starred_followers (反向关注且 is_starred=true)
  → 对每个在线 starred_follower 推送 WS FriendJoinedRoom { friend_id, room_id, room_title }
  → Android 收到后：聊天/IM Tab 顶部红点 + 通知（如允许）
```

### 3.4 1v1 私聊
```
用户在好友列表点 IM → 跳 1v1 聊天页（频道 dm:{a}_{b}）
  → 首条消息时校验：必须互关 OR 进过同房间最近 24h（陌生人首条限流 5/日）
  → 写 dm_messages 表（user_a / user_b / sender_id / msg_id / content / created_at）
  → 广播仅给在线对端 + 自己其它会话
```

### 3.5 关键异常

| 场景 | 处理 |
|------|------|
| 自关注 | 40930 SELF_FOLLOW |
| 已关注重复请求 | 幂等返同一关系 |
| 日关注上限 | 40931 FOLLOW_LIMIT |
| 拉黑后取关再关注 | 40932 BLOCKED |
| 陌生人首条 1v1 超限 | 40933 STRANGER_DM_LIMIT |
| 网络不稳定关注按钮多次点击 | 客户端去抖 1s + Server 幂等兜底 |
| 大 R 1 小时被关注 1000 次（机器人攻击） | 触发 follow 限流 + 风控告警 |

---

## 4. 关键技术约束

1. **关系派生模型**（红线 #1）：好友 = 互关，**不**单独存表，避免双写不一致；查询通过两次 follows JOIN
2. **强事务**：follow + 双向检查 + friend 升级广播在同一事务（避免双方一边显示好友一边没有）
3. **配置化限流**：日上限/陌生人 DM 上限走 `config/business.toml`
4. **WS 频道复用**：1v1 频道 `dm:{minUserId}_{maxUserId}` 排序避免双方建立两个频道
5. **观测性**：所有关系变更埋 Analytics（`follow_create / follow_remove / friend_created / star_toggle / block / dm_send`）
6. **隐私默认**：`hide_follow_list` 字段（Phase 2.5），用户可隐藏自己的关注列表

---

## 5. 验收指标

| 类别 | 指标 | 目标 |
|------|------|------|
| 功能 | 关注/取关/拉黑/1v1 全跑通 | 100% |
| 性能 | 关注/粉丝列表 100 人 P95 | < 200ms |
| 性能 | 在线状态查询 1000 用户 | < 100ms |
| 业务 | Phase 2 上线 30 天人均关注数（活跃用户） | ≥ 8 |
| 业务 | 互关好友比例（活跃用户中至少 1 好友） | ≥ 35% |
| 业务 | 次留存（无关系 vs 有 ≥1 好友） | 提升 ≥ 80% |

---

## 6. 关联文档

- [Tasks 模块 13 - E-12 关系链](../tasks/模块13-好友关注关系链%20(E-12).md)
- [Android 设计稿 - T-30090 用户资料卡](../design/android/T-30090.md)
- [phase2_family_guild.md](./phase2_family_guild.md) — E-11 家族对照
- [competitors.md §关系链留存](./competitors.md)
