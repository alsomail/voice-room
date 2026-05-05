# 三、房间模块 (Room)

## 3.1 POST /api/v1/rooms

创建房间。**需要 JWT 认证**。

**Request Headers**: `Authorization: Bearer <token>`

**Request Body**:
```json
{
  "title": "我的语音房",
  "room_type": "normal",
  "password": null
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `title` | `string` | ✅ | 房间标题，1–30 个 Unicode 字符 |
| `room_type` | `string` | ✅ | 枚举：`normal` / `password` / `paid` |
| `password` | `string` | 条件 | `room_type=password` 时必填；`normal` / `paid` 类型忽略此字段 |

**Success Response (201)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "room_id": "550e8400-e29b-41d4-a716-446655440001",
    "title": "我的语音房",
    "room_type": "normal",
    "created_at": "2026-04-18T00:00:00Z"
  },
  "request_id": "uuid"
}
```

**Error Scenarios**:

| 场景 | HTTP | 错误码 | message |
|------|------|--------|---------|
| title 为空 / 超过 30 字符 / room_type 非法 / 密码房未提供密码 | 400 | `40003` | Validation error: \<detail\> |
| 无 token / 签名无效 | 401 | `40101` | Unauthorized |
| Token 已过期 | 401 | `40102` | Token expired |
| 用户已有活跃房间 | 409 | `40900` | User already has an active room |

**业务规则**:
- `title` 长度按 Unicode 字符数（`chars().count()`）计算，1 个中文字符 = 1 个字符
- `room_type=password` 时服务端对 `password` 做 bcrypt 哈希存储，绝不明文保存
- `room_type=normal` 或 `paid` 时即使请求体携带 `password` 字段也会被忽略（`password_hash` 存 `NULL`）
- 同一用户同时只能拥有 1 个 `active` 房间；若已有则返回 409（DB 层由唯一偏滤索引 `idx_rooms_owner_active` 兜底，并发安全）
- 成功返回 **HTTP 201**（Created），而非 200

---

## 3.2 GET /api/v1/rooms

获取活跃房间列表。**无需鉴权（公开接口）**。按热度（在线人数降序）排序，过滤已关闭房间。

**Query Parameters**:

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `page` | `integer` | 否 | `1` | 页码，最小值 `1` |
| `size` | `integer` | 否 | `20` | 每页条数，范围 `1–100` |

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "total": 42,
    "page": 1,
    "size": 20,
    "items": [
      {
        "room_id": "550e8400-e29b-41d4-a716-446655440001",
        "title": "欢迎来我的语音房",
        "room_type": "normal",
        "member_count": 18,
        "max_members": 50,
        "owner_id": "550e8400-e29b-41d4-a716-446655440000",
        "owner_nickname": "User_a1b2",
        "owner_avatar": "https://cdn.example.com/avatars/xxx.jpg",
        "created_at": "2026-04-18T00:00:00Z"
      }
    ]
  },
  "request_id": "uuid"
}
```

**items 字段说明**:

| 字段 | 类型 | 说明 |
|------|------|------|
| `room_id` | `string (UUID)` | 房间唯一 ID |
| `title` | `string` | 房间标题，1–30 字符 |
| `room_type` | `string` | 枚举：`normal` / `password` / `paid` |
| `member_count` | `integer` | 当前在线人数（排序依据，降序） |
| `max_members` | `integer` | 房间人数上限（默认 50） |
| `owner_id` | `string (UUID)` | 房主用户 ID |
| `owner_nickname` | `string` | 房主昵称（JOIN users 表） |
| `owner_avatar` | `string \| null` | 房主头像 URL，无头像时为 `null` |
| `created_at` | `string (ISO 8601)` | 房间创建时间 |

**Error Scenarios**:

| 场景 | HTTP | 错误码 | message |
|------|------|--------|---------|
| `page < 1` | 400 | `40003` | Validation error: page must be >= 1 |
| `size < 1` | 400 | `40003` | Validation error: size must be >= 1 |
| `size > 100` | 400 | `40003` | Validation error: size must be <= 100, got {size} |

**排序与过滤规则**:
- 固定按 `member_count DESC, created_at DESC` 双字段排序（热度优先，同热度下按创建时间降序）
- 仅返回 `status = 'active' AND deleted_at IS NULL` 的房间
- `page` 超出总页数时 `items` 返回空数组，`total` 仍为真实总数

---

## 3.3 获取房间详情

**接口**：`GET /api/v1/rooms/:id`  
**认证**：公开，无需 JWT  
**描述**：获取单个 active 房间的详细信息，包括房主信息和麦位列表（MVP 为空）

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| id | UUID | 房间 ID |

**响应 200 OK**：
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "room_id": "uuid",
    "title": "string",
    "room_type": "normal | password | paid",
    "member_count": 0,
    "max_members": 50,
    "owner": {
      "user_id": "uuid",
      "nickname": "string",
      "avatar": "string | null"
    },
    "mic_slots": [],
    "created_at": "RFC3339"
  }
}
```

**错误码**：
| HTTP | code | 说明 |
|------|------|------|
| 404 | 40400 | 房间不存在或已关闭 |
| 400 | 40003 | room_id 格式非法（非 UUID） |

---

## 3.4 关闭房间

**接口**：`DELETE /api/v1/rooms/:id`  
**认证**：需要 JWT（Bearer Token），仅房主可操作  
**描述**：将 active 状态的房间改为 closed。MVP 阶段不广播 WebSocket 事件（待 T-00011 接入）。

**路径参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| id | UUID | 要关闭的房间 ID |

**响应 200 OK**：
```json
{
  "code": 0,
  "message": "ok",
  "data": null,
  "request_id": "uuid"
}
```

**错误码**：
| HTTP | code | 说明 |
|------|------|------|
| 400 | 40003 | room_id 格式非法（非 UUID） |
| 401 | 40101 | 未提供 Token 或签名无效 |
| 401 | 40102 | Token 已过期 |
| 403 | 40301 | 当前用户不是房主 |
| 404 | 40400 | 房间不存在或已软删除 |
| 409 | 40901 | 房间已处于 closed 状态 |

---

## 3.5 Phase 1.5 (E-10) 新增接口索引

> 详细契约参见各 TDS 文档。

| 接口 | 方法与路径 | 关联 Task | 详细契约 |
|------|-----------|-----------|---------|
| 创建房间升级（封面/分类/公告/密码） | `POST /api/v1/rooms`（扩展字段） | T-00025 | [tds/server/T-00025.md](../tds/server/T-00025.md) |
| 编辑房间信息 | `PATCH /api/v1/rooms/:id` | T-00025 | 同上 |
| 密码房密码验证 | `POST /api/v1/rooms/:id/verify-password` | T-00026 | [tds/server/T-00026.md](../tds/server/T-00026.md) |
| 房间成员列表（观众席） | `GET /api/v1/rooms/:id/members` | T-00027 | [tds/server/T-00027.md](../tds/server/T-00027.md) |

### 3.5.1 POST /api/v1/rooms 扩展字段（向下兼容）
```json
{
  "title": "...",
  "room_type": "normal|password",
  "cover_url": "/assets/covers/desert.webp",
  "category": "chat|emotion|music|game|matchmaking|other",
  "announcement": "≤200 字",
  "password": "6 位数字（room_type=password 必填）"
}
```

### 3.5.2 POST /api/v1/rooms/:id/verify-password
- Body: `{ "password":"123456" }`
- 成功返回短期 access_token（TTL 60s），供 WS JoinRoom 使用
- 5 次错误后锁定 30min（`code=42910`）

### 3.5.3 GET /api/v1/rooms/:id/members
- Query: `page`, `limit` (max 100)
- 麦上用户置顶；观众按 `joined_at DESC`
- 返回 `role` (`owner|admin|member`) + `mic_slot`

---

## 3.6 Chat 文本消息写入与历史（T-00043 / T-00045 / T-00047）

> **协议路径定位**：本节为 chat 写入的 **REST 备路径**；客户端**主路径** ⭐ 走 WS `SendMessage`（[websocket_signals.md §6.8](./websocket_signals.md)）。REST 路径定位为运营 / 后端兜底场景。

### 3.6.1 POST /api/v1/chat-messages（写入 + 广播）

发送一条房间文本消息，**INSERT DB 后**通过既有 WS 广播器向房间内所有连接广播 `RoomMessage`（与 WS `SendMessage` 路径形态等价，详见 [websocket_signals.md §6.8.3](./websocket_signals.md)）。**需要 JWT 认证**。

**Request Headers**: `Authorization: Bearer <token>`, `Content-Type: application/json`

**Request Body**:
```json
{
  "room_id": "550e8400-e29b-41d4-a716-446655440001",
  "content": "مرحبا 大家好"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `room_id` | `string (UUID)` | ✅ | 目标房间 ID |
| `content` | `string` | ✅ | 文本内容；按 Unicode `chars().count()` 计长，1–500；空串 / 全空白拒绝 |

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": { "msg_id": "<chat_messages.id, UUID v4>" },
  "request_id": "uuid"
}
```

**Error Scenarios**:

| 场景 | HTTP | 错误码 | message |
|------|------|--------|---------|
| `room_id` 非合法 UUID | 400 | `40003` | Validation error: invalid room id format |
| `content` 缺失 / 空字符串 | 400 | `40003` | Validation error: content is required |
| `content` 超过 500 字符 | 400 | `40003` | Validation error: message exceeds 500 characters |
| 无 token / 签名无效 | 401 | `40101` | Unauthorized |
| Token 已过期 | 401 | `40102` | Token expired |
| DB 写入失败 | 500 | `50000` | DB persist failed |

**业务规则**:
- 广播在 DB 事务**提交后**触发；DB 失败 → 不广播 → 返回 5xx。
- 广播 envelope 形态见 [websocket_signals.md §6.8.2](./websocket_signals.md)；`payload.msg_id` = 本响应 `data.msg_id`（DB 行主键）。
- 房间未在内存（无 JoinRoom 历史）：降级 `broadcast_to_room_no_state`，**不**写 `recent_broadcasts`，DB 仍正常落库，HTTP 200。
- 单连接广播失败容忍（`let _ = sender.send(...)`），不影响 HTTP 响应。
- **敏感词过滤**：T-00047 起 REST 路径必须调用 `filter_content(content)` 后再写 DB + 广播，与 WS 路径 `payload.content` 完全等价（PROTO-2）。
- **muted 检查 / 幂等去重**：REST 路径**不**做 `muted_users` / `processed_msg_ids` 校验；`Idempotency-Key` 头由后续 Task 接入。

### 3.6.2 GET /api/v1/rooms/:room_id/messages（历史分页查询，T-00043）

按 `created_at DESC` 分页查询房间历史消息。**需要 JWT 认证**。

**Query Parameters**: `before?: string (UUID)`（游标）、`limit?: int`（默认 50，max 100）

**Success Response (200)**: 标准分页 `{ items: [{ id, room_id, user_id, content, created_at }, ...], next_cursor }`

> 详细字段定义参见 [tds/server/T-00043.md](../tds/server/T-00043.md)。本节仅声明路径与定位；**`items[].id` 与 §3.6.1 / §6.8.2 的 `payload.msg_id` 严格相等**，供客户端去重 / 锚定使用。
