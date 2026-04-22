<!--
[AI 写入规约]
本文件记录 Room HTTP API 模块的架构设计与实现状态（T-00007 ~ T-00027 等）。
仅写架构约定与接口契约，不重复 TDS 中的 TDD 验收用例原文。
-->

# Room HTTP API 架构设计

> 关联 TDS：[T-00025](../../tds/server/T-00025.md) · [T-00026](../../tds/server/T-00026.md) · [T-00027](../../tds/server/T-00027.md)
> 数据库基座：[database.md](./database.md)（T-00024 治理扩字段）

---

## 一、POST /api/v1/rooms — 创建房间（T-00025 升级版）

### 请求体（扩展后）

```json
{
  "title": "中东夜话",
  "room_type": "normal",
  "cover_url": "/assets/covers/desert.png",
  "category": "chat",
  "announcement": "欢迎来到中东夜话~",
  "password": "123456"
}
```

### 字段校验规则

| 字段 | 规则 | 错误码 |
|------|------|--------|
| `cover_url` | 以 `/assets/covers/` 或 CDN 白名单前缀开头；缺省为空串（合法） | 40003 |
| `category` | 枚举：`chat` / `emotion` / `music` / `game` / `matchmaking` / `other` | 40003 |
| `announcement` | ≤ 200 Unicode 字符（含 emoji 计 1 字）；可缺省 | 40003 |
| `password` | `^\d{6}$`，**仅** `room_type=password` 时校验；`normal` 类型时忽略且 `password_hash` 写 NULL | 40003 |

### 密码存储策略

- 密码**明文永不落库**
- `bcrypt(password, cost=12)` → 写 `rooms.password_hash`
- 响应体不返回任何与密码相关的字段

---

## 二、封面 URL 白名单校验（validator.rs）

```rust
const COVER_PREFIX_ALLOW: &[&str] = &[
    "/assets/covers/",
    "https://cdn.voiceroom.example/covers/",
];

fn validate_cover_url(url: &str) -> Result<()> {
    if url.is_empty() { return Ok(()); }
    if COVER_PREFIX_ALLOW.iter().any(|p| url.starts_with(p)) {
        Ok(())
    } else {
        Err(Error::validation("invalid cover_url"))
    }
}
```

MVP 阶段仅允许 8 张预设封面（`/assets/covers/` 前缀）。后续扩展仅需在 `COVER_PREFIX_ALLOW` 中追加 CDN 前缀。

---

## 三、validator.rs — 四个独立验证函数

| 函数 | 职责 |
|------|------|
| `validate_cover_url(url)` | 白名单前缀检查，空串放行 |
| `validate_password(password)` | `^\d{6}$` 正则，6 位纯数字 |
| `validate_announcement(text)` | Unicode 字符计数 ≤ 200 |
| `validate_category(category)` | 6 枚举值之一 |

所有函数均返回 `Result<(), AppError>`，错误码 **40003**。
单元测试覆盖：V-01 ~ V-19（共 19 个）。

---

## 四、PATCH /api/v1/rooms/:id — 房主更新房间信息（T-00025 新增）

**鉴权**：JWT（仅房主），非房主返回 403（40301）。

### 请求体

```json
{
  "title": "新标题",
  "announcement": "新公告，空串表示清空",
  "category": "music"
}
```

- 三个字段均可选，但至少提供一个，否则 400（40003）
- 房间已 `closed` 状态 → 409（40901）

### 处理流程

1. JWT 解析 → 取 `user_id`
2. 查库校验房间存在 + `owner_id == user_id`
3. 校验房间状态为 `active`
4. 对非 NULL 字段分别调用 validator 校验
5. `repository.update_room_fields(room_id, update)` 写库
6. 广播 `RoomInfoUpdated` WS 信令给房间全体成员
7. 返回 200 + 更新后的房间信息

---

## 五、WS 广播 RoomInfoUpdated（S→C）

广播时机：`PATCH /api/v1/rooms/:id` 成功后，向房间内所有 WS 连接推送。

```json
{
  "type": "RoomInfoUpdated",
  "payload": {
    "room_id": "uuid",
    "title": "新标题",
    "announcement": "新公告",
    "category": "music",
    "cover_url": "/assets/covers/desert.png",
    "has_password": true
  },
  "timestamp": 1700000000000
}
```

- `has_password`：布尔值，客户端据此渲染「密码锁」图标，**不传明文密码**
- ⚠️ **遗留 MEDIUM 项**：当前 `BroadcastEnvelope` 缺少 `msg_id` 字段，违反协议 §6.3 WS 通用格式约定（`BalanceUpdated`/`GiftReceived` 均含 `msg_id`）。后续迭代需补齐：
  - `BroadcastEnvelope` 增加 `msg_id: String`（`Uuid::new_v4().to_string()`）
  - `doc/protocol/websocket_signals.md` §6.6 同步更新

实现文件：`app/server/src/ws/broadcaster.rs`（`broadcast_room_info_updated` + `RoomInfoUpdatedPayload`），含 4 个单元测试（BR-01 ~ BR-04）。

---

## 六、涉及文件清单

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `app/server/src/modules/room/validator.rs` | **新增** | 4 个验证函数 + 19 个单元测试 |
| `app/server/src/ws/broadcaster.rs` | **新增** | `broadcast_room_info_updated` + payload struct |
| `app/server/src/modules/room/dto.rs` | 修改 | `CreateRoomRequest` 新增 4 字段；新增 `PatchRoomRequest`/`PatchRoomResponse`/`RoomFieldsUpdate` |
| `app/server/src/modules/room/repository.rs` | 修改 | trait 新增 `update_room_fields`；`FakeRoomRepository`/`PgRoomRepository` 实现；`create` SQL 含新字段 |
| `app/server/src/modules/room/service.rs` | 修改 | `create_room` 接入验证器；新增 `patch_room` |
| `app/server/src/modules/room/controller.rs` | 修改 | 新增 `patch_room` handler |
| `app/server/src/modules/room/routes.rs` | 修改 | `/api/v1/rooms/{id}` 注册 `.patch(patch_room)` |
| `app/server/src/modules/room/mod.rs` | 修改 | 导出 `validator` 模块 |
| `app/server/src/ws/mod.rs` | 修改 | 导出 `broadcaster` 模块 |

---

## 七、测试覆盖汇总（T-00025）

| 测试集 | 范围 | 数量 |
|--------|------|------|
| `validator.rs` 内联测试 | V-01 ~ V-19 覆盖四个验证函数边界 | 19 |
| `broadcaster.rs` 内联测试 | BR-01 ~ BR-04 广播逻辑与 payload 结构 | 4 |
| `service.rs` T-00025 单元测试 | CR25-01~07（创建）+ PR25-08~12（PATCH） | 12 |
| **全量测试** | 369 个（344 单元 + 23 schema + 2 doc）全部 ✅ | 369 |

---

## 八、POST /api/v1/rooms/:id/verify-password — 密码校验 + 锁定（T-00026）

> 关联 TDS：[T-00026](../../tds/server/T-00026.md)

### 请求体

```json
{ "password": "123456" }
```

### 响应

**成功 200**：
```json
{ "code": 0, "data": { "access_token": "<jwt-60s>" } }
```

JWT Claims（`iss=voiceroom-room-access`，TTL 60s）：
```json
{ "sub": "<user_id>", "room_id": "<room_id>", "iat": 1700000000, "exp": 1700000060, "iss": "voiceroom-room-access" }
```

**错误码**：

| HTTP | code | 说明 |
|------|------|------|
| 400 | 40003 | password 格式非 6 位数字 |
| 404 | 40400 | 房间不存在或已关闭 |
| 400 | 40014 | 非密码房（`room_type != password`） |
| 401 | 40103 | 密码错误（含 `remaining_attempts` payload） |
| 429 | 42910 | 已锁定（含 `locked_remaining_sec` payload） |

---

## 九、Redis Key 策略（T-00026）

| Key | 类型 | TTL | 用途 |
|-----|------|-----|------|
| `pwd_fail:{user_id}:{room_id}` | Int | 1800s | 失败计数 |
| `pwd_lock:{user_id}:{room_id}` | String | 1800s | 锁定标记 |

### 锁定流程

```
1. 检查 pwd_lock 是否存在
   └─ 存在 → 返回 42910 + get_ttl(pwd_lock) 作为 locked_remaining_sec

2. bcrypt 验证 password
   ├─ 成功 → DEL pwd_fail → 签发 room_access token → 200
   └─ 失败 → INCR pwd_fail (原子)
             若 count >= 5 → SET NX EX 1800 pwd_lock (原子防重复写)
             返回 40103 + remaining_attempts = 5 - count
```

**并发安全**：`SET NX EX` 单条原子命令，并发多个第 5 次失败请求仅一条创建锁定 Key。

> ⚠️ **遗留 MEDIUM 项**：`incr_with_ttl` 生产实现中 `INCR` + `EXPIRE` 为两条命令，非原子。极端情况下进程崩溃于两命令之间将导致 `pwd_fail` Key 无 TTL 永久存在（仅影响该用户该房间的失败计数自动清除）。后续迭代可用 Lua 脚本合并为单条原子操作消除崩溃窗口。

---

## 十、WS JoinRoom 密码房校验（T-00026）

`JoinRoom` payload 新增可选字段 `access_token?: string`。

| 场景 | 返回错误码 |
|------|-----------|
| 密码房无 `access_token` | `PASSWORD_REQUIRED (40104)` |
| token 已过期（> 60s） | `TOKEN_EXPIRED (40105)` |
| token `room_id` 与目标房间不匹配 | `INVALID_TOKEN (40106)` |
| token `iss` ≠ `voiceroom-room-access` | `INVALID_TOKEN (40106)` |
| token `sub` ≠ 当前 `user_id` | `INVALID_TOKEN (40106)` |

**双重校验顺序**：先校验 `iss` → 再校验 `sub == user_id` → 再校验 `room_id` 匹配 → 最后检查未过期。

---

## 十一、涉及文件清单（T-00026 新增 / 修改）

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `app/shared/src/auth/room_access.rs` | **新增** | JWT encode/decode（6 个单元测试 RA01~RA06） |
| `app/shared/src/auth/mod.rs` | **新增** | 导出 room_access 模块 |
| `app/shared/src/lib.rs` | 修改 | 导出 auth 模块 |
| `app/shared/src/error/code.rs` | 修改 | 新增 `NotPasswordRoom=40014`, `PasswordRoomLocked=42910` |
| `app/server/src/common/error.rs` | 修改 | 新增 `AppError::NotPasswordRoom` |
| `app/server/src/modules/room/password.rs` | **新增** | `RoomPasswordRedis` trait + Fake/Real 实现 + `verify_password` 函数（8 单元测试） |
| `app/server/src/modules/room/dto.rs` | 修改 | 新增 `VerifyPasswordRequest/Response/WrongPasswordData/LockedData` |
| `app/server/src/modules/room/service.rs` | 修改 | 新增 `get_active_room_model` 方法 |
| `app/server/src/modules/room/controller.rs` | 修改 | 新增 `verify_password_handler` |
| `app/server/src/modules/room/routes.rs` | 修改 | 注册 `POST /verify-password` 路由 |
| `app/server/src/modules/room/mod.rs` | 修改 | 导出 `password` 模块 |
| `app/server/src/room/handler.rs` | 修改 | `JoinRoomDeps` 新增 `jwt_secret`；`handle_join_room` 加入 access_token 校验（4 单元测试） |
| `app/server/src/ws/connection.rs` | 修改 | `SocketDeps`/`handle_socket` 新增 `jwt_secret` 参数 |
| `app/server/src/ws/handler.rs` | 修改 | 传递 `jwt_secret` 到 `handle_socket` |
| `app/server/src/bootstrap/mod.rs` | 修改 | `AppState` 新增 `room_password_redis` 字段 + builder |
| `app/server/src/main.rs` | 修改 | 构建 `RealRoomPasswordRedis` 并注入 `AppState` |
| `app/server/tests/password_room_test.rs` | **新增** | PR26-01 ~ PR26-12 全部 12 个集成测试 |
| `app/server/Cargo.toml` | 修改 | 注册 password_room_test 测试文件 |

---

## 十二、测试覆盖汇总（T-00026）

| 测试集 | 范围 | 数量 |
|--------|------|------|
| `password.rs` 内联测试 | 锁定/计数/成功/非密码房边界 | 8 |
| `room_access.rs` 内联测试 | RA01~RA06 JWT encode/decode | 6 |
| `room/handler.rs` 内联测试 | PR26-02/03/04/12 WS 校验层 | 4 |
| 集成测试 `password_room_test.rs` | PR26-01 ~ PR26-12 端到端 | 12 |
| **全量测试** | 382+ 个全部 ✅ | 382+ |

---

## 十三、GET /api/v1/rooms/:id/members — 观众席列表 API（T-00027）

> 关联 TDS：[T-00027](../../tds/server/T-00027.md)

### 接口定义

**GET /api/v1/rooms/:id/members?page=1&limit=20**（需 JWT，仅连接中成员可调）

#### 响应格式

```json
{
  "code": 0,
  "data": {
    "total": 87,
    "page": 1,
    "limit": 20,
    "items": [
      {
        "user_id": "uuid",
        "nickname": "...",
        "avatar": "...",
        "role": "owner|admin|member",
        "mic_slot": 0,
        "joined_at": "2026-04-23T10:00:00Z",
        "muted_mic": false,
        "muted_chat": false
      }
    ]
  }
}
```

- `mic_slot`：整数（0 = 主麦），`null` 表示观众席
- `muted_mic` / `muted_chat`：仅对连接中的房间成员可见（管理员视角）；非成员请求返回 403（40301）

---

### 排序规则

1. **麦上用户置顶**：`mic_slot IS NOT NULL ORDER BY slot ASC`（主麦 slot=0 排最前）
2. **观众按 `joined_at DESC`**：最新进房者在前

---

### 角色计算优先级

```
user_id == room.owner_id       → role = "owner"
user_id == room.admin_user_id  → role = "admin"
else                            → role = "member"
```

优先级：**owner > admin > member**（同一用户若既是 owner 又是 admin_user_id，取 owner）

---

### 性能设计

| 步骤 | 操作 | 复杂度 |
|------|------|--------|
| 1 | `RoomManager.list_members(room_id)` 内存读取 `Vec<MemberSnapshot>` | O(n) 无 DB |
| 2 | 批量 `SELECT id, nickname, avatar FROM users WHERE id = ANY($1) AND deleted_at IS NULL` | **1 次 SQL** |
| 3 | `muted_mic` / `muted_chat` 从 Redis Key `mic_muted:{room_id}:{user_id}` / `chat_muted:{room_id}:{user_id}` 批量读取 | O(n) 内存/Redis |
| 4 | 复合排序 + 分页 slice | O(n log n) |

100 人房间 p95 目标 **< 150ms**。

---

### MemberSnapshot 结构体

`RoomManager` 内存中维护的每位成员快照：

```rust
pub struct MemberSnapshot {
    pub user_id:   Uuid,
    pub joined_at: DateTime<Utc>,   // 进房时间（单一数据源，不再使用 member_join_times DashMap）
    pub mic_slot:  Option<u8>,      // None = 观众；Some(n) = n 号麦位
}
```

> **单一数据源**：`joined_at` 统一从 `MemberInfo.joined_at` 读取；`RoomState.member_join_times DashMap` 已于 T-00027 Round 2 修复中完整移除（7 处引用全部删除）。

---

### 权限与错误码

| 场景 | HTTP | code |
|------|------|------|
| 非连接中用户（路人 HTTP 请求） | 403 | 40301 |
| `page < 1` | 400 | 40003 |
| 房间不存在 | 404 | 40400 |
| `page` 超界 | 200 | `items: []`，`total` 返回真实总数 |

---

## 十四、涉及文件清单（T-00027 新增 / 修改）

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `app/server/src/modules/room/members_handler.rs` | **新增** | list members handler + `AuthServiceUserAdapter`（单次批量调用） |
| `app/server/src/modules/room/members_service.rs` | **新增** | `MembersPort` trait + 业务组装逻辑 |
| `app/server/src/modules/auth/repository.rs` | 修改 | `UserRepository` trait 新增 `find_by_ids`；`PgUserRepository` `WHERE id = ANY($1)` 单次 SQL；R01~R05 单元测试 |
| `app/server/src/modules/auth/service.rs` | 修改 | `AuthService` 新增 `get_users_by_ids` 方法；S01~S03 单元测试 |
| `app/server/src/room/manager.rs` | 修改 | `list_members(room_id)` 直接读 `MemberInfo.joined_at`，移除 fallback |
| `app/server/src/room/state.rs` | 修改 | 删除 `member_join_times DashMap`（单一数据源） |
| `app/server/src/room/handler.rs` | 修改 | 删除 `member_join_times.insert` / `.remove` 调用 |
| `app/server/src/bootstrap/router.rs` | 修改 | 挂载 `GET /api/v1/rooms/{id}/members` 路由 |
| `app/server/tests/members_list_test.rs` | **新增** | M27-01 ~ M27-08 集成测试 |

---

## 十五、测试覆盖汇总（T-00027）

| 测试集 | 范围 | 数量 |
|--------|------|------|
| `repository.rs` 内联测试 | R01~R05 批量查询 `find_by_ids` | 5 |
| `service.rs` 内联测试 | S01~S03 `get_users_by_ids` | 3 |
| 集成测试 `members_list_test.rs` | M27-01 ~ M27-08 端到端 | 8 |
| **全量测试** | 398 个全部 ✅ | 398 |

---

## 十六、WS KickUser 信令格式（T-00028）

> 关联 TDS：[T-00028](../../tds/server/T-00028.md)

### C→S `KickUser`（请求）

```json
{
  "type": "KickUser",
  "msg_id": "uuid",
  "payload": {
    "room_id": "uuid",
    "target_user_id": "uuid",
    "reason": "harassment"
  }
}
```

### S→C `KickUserResult`（响应）

```json
{ "type": "KickUserResult", "msg_id": "uuid", "code": 0 }
```

**错误码**：

| code | 含义 |
|------|------|
| 40301 | `PERMISSION_DENIED`（操作者非 owner/admin） |
| 40302 | `CANNOT_KICK_OWNER`（不可踢房主） |
| 40400 | 房间不存在或 target 不在房间 |
| 40003 | reason 缺失或格式非法 |

### S→目标 `UserKicked`（广播，仅目标用户）

```json
{
  "type": "UserKicked",
  "payload": {
    "room_id": "uuid",
    "reason": "harassment",
    "cooldown_sec": 600,
    "operator_nickname": "..."
  }
}
```

### S→房间其他人 `UserLeft`（广播，排除被踢者）

扩展 `reason` 字段标识踢出来源：

```json
{
  "type": "UserLeft",
  "payload": {
    "user_id": "uuid",
    "reason": "kicked_by_admin",
    "operator_id": "uuid"
  }
}
```

### S→房间全体 `MicLeft`（仅被踢者在麦时额外广播）

```json
{
  "type": "MicLeft",
  "payload": {
    "slot": 1,
    "user_id": "uuid",
    "forced": true
  }
}
```

---

## 十七、KickUser 处理流程（7 步）

```
1. 权限校验：ctx.user_id ∈ {owner, admin}
   └─ 不在 → 返回 40301 PERMISSION_DENIED
   └─ target == room.owner_id → 返回 40302 CANNOT_KICK_OWNER

2. target 存在性校验：room_manager.is_member(room_id, target_user_id)
   └─ 不在房间 → 返回 40400

3. Redis SETEX kicked:{room_id}:{target_user_id} 600 reason（冷却写入）

4. DB INSERT room_kick_records（审计落库）

5. RoomManager 移除 + 若在麦自动下麦
   └─ 若 mic_slot_of(room_id, target) = Some(slot)
       ├─ room_manager.leave_mic(room_id, slot)
       └─ 广播 MicLeft { slot, user_id: target, forced: true }
   └─ room_manager.remove_member(room_id, target)（DashMap.remove() 原子）

6. 广播
   ├─ UserKicked 仅发给被踢者所有连接
   └─ UserLeft(reason="kicked_by_admin") 广播给房间其余所有人

7. 主动关闭被踢者 WS 连接（conn.close(Reason::Kicked)）
```

---

## 十八、权限校验规则

| 操作者角色 | 可踢目标 | 不可踢目标 |
|-----------|---------|----------|
| owner | admin、member | — |
| admin | member | owner、其他 admin |
| member | — | 任何人（40301） |

**优先级铁律**：`owner > admin > member`，任何人均不可踢 owner（40302）。

---

## 十九、Redis 冷却 Key 与 JoinRoom 拦截

### 冷却 Key

| Key 格式 | 类型 | TTL | 存储内容 |
|---------|------|-----|---------|
| `kicked:{room_id}:{user_id}` | String | 600s | kick reason（如 `harassment`） |

写入时机：踢人流程第 3 步，`SETEX` 单条原子命令。

### JoinRoom 前置拦截

`JoinRoom` 信令处理前增加冷却检查：

```rust
// handler.rs — handle_join_room 冷却前置
if let Some(remaining_sec) = kick_redis.get_ttl(room_id, user_id).await? {
    return Err(AppError::KickedCooldown { remaining_sec });
}
// 错误码 42911，payload: { remaining_sec }
```

错误码 `42911 KICKED_COOLDOWN` 含 `remaining_sec` 字段，客户端据此展示倒计时。

---

## 二十、并发保护

多个管理员同时踢同一人场景：

- **`DashMap.remove()` 原子性**：`remove_member` 返回 `Option`；仅第一次返回 `Some` 的请求走完完整流程（广播 + 关闭连接），后续返回 `None` 的请求提前退出。
- **Redis SETEX 覆盖无副作用**：多次 SETEX 相同 key 仅覆盖值（TTL 重置），最终冷却仍正确。
- **`room_kick_records` 允许多条**：并发踢出可能插入多行（每位管理员操作均有审计记录），符合治理日志完整性要求。

---

## 二十一、遗留问题（T-00028）

| 级别 | 问题描述 | 建议修复时机 |
|------|---------|------------|
| **MEDIUM** | 当前实现先广播 `UserLeft`（步骤14）再广播 `MicLeft`（步骤15），与 TDS §二 约定顺序（先 MicLeft 后 UserLeft）相反。功能正确，但客户端可能出现瞬时 UI 异常（麦位已清空但用户列表未更新）。 | 下一迭代调整广播顺序 |
| **LOW** | `RealKickRedis.get_ttl` 当 Redis 返回 `-1`（key 存在无 TTL）时当前代码归入 `_ => Ok(None)` 按无冷却处理。保守方案建议返回满冷却秒数（600s）以防永久 key 绕过冷却。 | 下一 Redis 工具类迭代修复 |

---

## 二十二、涉及文件清单（T-00028 新增 / 修改）

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `app/server/src/modules/governance/mod.rs` | **新增** | governance 模块入口 |
| `app/server/src/modules/governance/kick.rs` | **新增** | 核心踢人逻辑 + `KickRedis`/`KickAuditDb` trait + Fake/Real 实现 |
| `app/server/src/modules/mod.rs` | 修改 | 注册 governance 子模块 |
| `app/server/src/room/manager.rs` | 修改 | 新增 `remove_member()` 返回 `Option`；新增 `is_member()` |
| `app/server/src/room/handler.rs` | 修改 | `JoinRoomDeps` 新增 `kick_redis` 字段；JoinRoom 冷却前置检查；`broadcast_mic_left` 增加 `forced: bool` 参数 |
| `app/server/src/ws/connection.rs` | 修改 | 新增 `KickUser` 分支；`SocketDeps` 新增 `kick_redis`/`kick_audit_db` 字段 |
| `app/server/src/ws/handler.rs` | 修改 | 传递 `kick_redis`/`kick_audit_db` 到 `handle_socket` |
| `app/server/src/bootstrap/mod.rs` | 修改 | `AppState` 新增 `kick_redis`/`kick_audit_db` + `with_kick_redis`/`with_kick_audit_db` builder |
| `app/server/tests/kick_user_test.rs` | **新增** | K28-01 ~ K28-12 集成测试（12 个） |
| `app/server/tests/password_room_test.rs` | 修改 | `JoinRoomDeps` 补充 `kick_redis: None` |
| `app/server/tests/send_gift_test.rs` | 修改 | `MemberInfo` 补全 `joined_at` 字段 |
| `app/server/Cargo.toml` | 修改 | 新增 `kick_user_test` 测试入口 |

---

## 二十三、测试覆盖汇总（T-00028）

| 测试集 | 范围 | 数量 |
|--------|------|------|
| 集成测试 `kick_user_test.rs` | K28-01 ~ K28-12 端到端 | 12 |
| **全量测试** | 366+ 个全部 ✅，零回归 | 366+ |
