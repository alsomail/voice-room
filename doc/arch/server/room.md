<!--
[AI 写入规约]
本文件记录 Room HTTP API 模块的架构设计与实现状态（T-00007 ~ T-00025 等）。
仅写架构约定与接口契约，不重复 TDS 中的 TDD 验收用例原文。
-->

# Room HTTP API 架构设计

> 关联 TDS：[T-00025](../../tds/server/T-00025.md)
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
