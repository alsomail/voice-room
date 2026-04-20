# Voice Room API 协议文档

> **版本**: v0.9
> **更新日期**: 2026-04-24
> **维护约束**: 新增/修改接口时必须同步更新本文件；前后端联调前必须以本文件为唯一契约源。

---

## 一、通用约定

### 1.1 基础地址

#### App Server (C 端业务后端)

| 环境 | HTTP Base URL | WebSocket URL |
|------|--------------|---------------|
| 本地开发 | `http://localhost:3000/api/v1` | `ws://localhost:3000/ws` |
| 测试环境 | `https://test-api.voiceroom.example/api/v1` | `wss://test-api.voiceroom.example/ws` |
| 生产环境 | `https://api.voiceroom.example/api/v1` | `wss://api.voiceroom.example/ws` |

#### Admin Server (B 端管理后端)

| 环境 | HTTP Base URL |
|------|--------------|
| 本地开发 | `http://localhost:3001/api/v1/admin` |
| 测试环境 | `https://test-admin-api.voiceroom.example/api/v1/admin` |
| 生产环境 | `https://admin-api.voiceroom.example/api/v1/admin` |

> Admin Server 仅 HTTP，不提供 WebSocket。通过 VPN 访问，不对公网暴露。

### 1.2 请求通用头

| Header | 必需 | 说明 |
|--------|------|------|
| `Content-Type` | 是 | `application/json` |
| `Authorization` | 条件 | `Bearer <JWT>`，需要鉴权的接口必传 |
| `X-Request-Id` | 否 | 请求追踪 ID，若不传则 Server 自动生成并在响应头回传 |
| `X-Device-Id` | 否 | 客户端设备标识，用于风控与埋点 |
| `Accept-Language` | 否 | `ar` / `en`，默认 `ar` |

### 1.3 统一响应结构

**成功响应**:
```json
{
  "code": 0,
  "message": "ok",
  "data": { "..." : "..." },
  "request_id": "uuid"
}
```

**错误响应**:
```json
{
  "code": 40001,
  "message": "Invalid phone number format",
  "request_id": "uuid"
}
```

### 1.4 错误码规范

| 范围 | 分类 | 说明 |
|------|------|------|
| `0` | 成功 | 请求成功 |
| `40000-40099` | 参数错误 | 请求参数校验失败 |
| `40100-40199` | 认证错误 | 未登录 / token 无效 / 验证码错误 |
| `40300-40399` | 权限错误 | 无权限操作 |
| `40400-40499` | 资源不存在 | 目标资源不存在 |
| `40900-40999` | 冲突错误 | 重复操作 / 状态冲突 |
| `42900-42999` | 频率限制 | 请求过于频繁 |
| `50000-50099` | 服务端错误 | 内部错误 |

#### 模块1 错误码表

| 错误码 | HTTP Status | 含义 | 触发场景 |
|--------|-------------|------|----------|
| `40001` | 400 | 手机号格式无效 | 发送验证码时手机号不合法 |
| `40002` | 400 | 参数缺失 | 必传字段为空 |
| `40101` | 401 | 未授权 | 无 token 或 token 签名无效 |
| `40102` | 401 | Token 已过期 | JWT 过期 |
| `40103` | 401 | 验证码错误 | 验证码不匹配 |
| `40104` | 401 | 验证码已过期 | 验证码超过 5 分钟有效期 |
| `40105` | 401 | 验证码尝试次数超限 | 同一验证码校验超过 5 次 |
| `40106` | 401 | 管理员账号或密码错误 | Admin 登录凭证无效 |
| `40301` | 403 | 权限不足 | RBAC 角色无权执行该操作 |
| `40302` | 403 | 账号已被禁用 | 管理员账号被 super_admin 停用 |
| `42901` | 429 | 验证码发送过于频繁 | 60 秒内重复发送 |
| `42902` | 429 | 每日发送次数超限 | 同一手机号当日超过 10 次 |

#### 模块2 错误码表（房间模块）

| 错误码 | HTTP Status | 含义 | 触发场景 |
|--------|-------------|------|----------|
| `40003` | 400 | 参数校验失败 | 标题为空 / 超 30 字符、`room_type` 非法枚举、密码房未提供密码 |
| `40900` | 409 | 用户已有活跃房间 | 同一用户尝试创建第二个 `active` 状态房间（DB 唯一偏滤索引兜底） |

### 1.5 分页约定

请求参数：`?page=1&size=20`

响应结构：
```json
{
  "total": 100,
  "page": 1,
  "size": 20,
  "items": []
}
```

### 1.6 幂等策略

| 接口 | 幂等性 | 策略 |
|------|--------|------|
| 发送验证码 | 冷却期幂等 | 60 秒冷却期内重复请求返回 `42901`；冷却期后重新发送新验证码 |
| 登录 | 可重试+自动注册 | 手机号不存在时自动创建用户；同一验证码有效期内可多次尝试（不超过 5 次） |
| 获取用户信息 | 只读幂等 | GET 请求，天然幂等 |

---

## 二、认证模块 (Auth)

### 2.1 POST /api/v1/auth/verification-codes

发送短信验证码。无需鉴权。

**Request Body**:
```json
{
  "phone": "+966512345678"
}
```

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "expires_in": 300,
    "cooldown": 60
  },
  "request_id": "uuid"
}
```

**Error Scenarios**:

| 场景 | HTTP | 错误码 | message |
|------|------|--------|---------|
| 手机号格式无效 | 400 | `40001` | Invalid phone number format |
| 60 秒内重复发送 | 429 | `42901` | Verification code sent too frequently |
| 当日发送次数超限 | 429 | `42902` | Daily verification code limit exceeded |

**业务规则**:
- 验证码为 6 位数字
- 有效期 5 分钟 (300 秒)
- 同一手机号 60 秒冷却期
- 同一手机号每日最多发送 10 次
- 验证码由 Server 生成后通过 `SmsProvider` 防腐层发送
- 冷却期内重复请求不会生成新验证码

---

### 2.2 POST /api/v1/auth/login

使用手机号 + 验证码登录。无需鉴权。**一步登录**：手机号不存在时自动创建用户并登录。

**Request Body**:
```json
{
  "phone": "+966512345678",
  "code": "123456"
}
```

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 2592000,
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "phone": "+966512345678",
      "nickname": "User_a1b2",
      "avatar": null,
      "coin_balance": 0,
      "vip_level": 0,
      "is_new": true,
      "created_at": "2026-04-17T00:00:00Z"
    }
  },
  "request_id": "uuid"
}
```

> `is_new`: `true` 表示本次登录为首次注册（前端可据此展示新手引导）

**Error Scenarios**:

| 场景 | HTTP | 错误码 | message |
|------|------|--------|---------|
| 验证码错误 | 401 | `40103` | Invalid verification code |
| 验证码已过期 | 401 | `40104` | Verification code expired |
| 验证码尝试次数超限 | 401 | `40105` | Verification code max attempts exceeded |

**JWT Claims 结构**:
```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "iat": 1713312000,
  "exp": 1715904000,
  "iss": "voiceroom"
}
```

| 字段 | 说明 |
|------|------|
| `sub` | user_id (UUID) |
| `iat` | 签发时间 (Unix timestamp) |
| `exp` | 过期时间，签发后 30 天 |
| `iss` | 固定值 `voiceroom` |

- 签名算法：HS256（MVP），后续可升级为 RS256
- Secret 从环境变量 `JWT_SECRET` 读取，禁止硬编码

---

### 2.3 GET /api/v1/users/me

获取当前登录用户信息。**需要 JWT 认证**。

**Request Headers**: `Authorization: Bearer <token>`

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "phone": "+966512345678",
    "nickname": "User_a1b2",
    "avatar": "https://cdn.example.com/avatars/xxx.jpg",
    "coin_balance": 1000,
    "vip_level": 2,
    "created_at": "2026-04-17T00:00:00Z"
  },
  "request_id": "uuid"
}
```

**Error Scenarios**:

| 场景 | HTTP | 错误码 | message |
|------|------|--------|---------|
| 无 token / 签名无效 | 401 | `40101` | Unauthorized |
| Token 已过期 | 401 | `40102` | Token expired |

---

## 三、房间模块 (Room)

### 3.1 POST /api/v1/rooms

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

### 3.2 GET /api/v1/rooms

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

### 3.3 获取房间详情

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
    "room_type": "public | password | paid",
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

### 3.4 关闭房间

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

## 四、Admin 认证模块 (Admin Auth)

> Admin Server 独立部署，使用独立的管理员账号体系。

### 4.1 POST /api/v1/admin/login

管理员使用账号密码登录。无需鉴权。

**Request Body**:
```json
{
  "username": "admin_operator",
  "password": "hashed_not_plain"
}
```

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 604800,
    "admin": {
      "id": "uuid",
      "username": "admin_operator",
      "role": "operator",
      "display_name": "运营小王",
      "last_login_at": "2026-04-17T00:00:00Z"
    }
  },
  "request_id": "uuid"
}
```

**Error Scenarios**:

| 场景 | HTTP | 错误码 | message |
|------|------|--------|---------|
| 账号或密码错误 | 401 | `40106` | Invalid admin credentials |
| 账号已被禁用 | 403 | `40302` | Admin account disabled |

**Admin JWT Claims 结构**:
```json
{
  "sub": "admin_uuid",
  "role": "operator",
  "iat": 1713312000,
  "exp": 1713916800,
  "iss": "voiceroom-admin"
}
```

| 字段 | 说明 |
|------|------|
| `sub` | admin_id (UUID) |
| `role` | 管理员角色: `super_admin` / `operator` / `cs` / `finance` |
| `iat` | 签发时间 |
| `exp` | 过期时间，签发后 7 天 |
| `iss` | 固定值 `voiceroom-admin`（区分 C 端 JWT） |

- 签名算法：HS256
- Secret 从环境变量 `JWT_SECRET` 读取（与 App Server 共享，通过 shared crate）

### 4.2 GET /api/v1/admin/me

获取当前管理员信息。**需要 Admin JWT 认证**。

**Request Headers**: `Authorization: Bearer <admin_token>`

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "id": "uuid",
    "username": "admin_operator",
    "role": "operator",
    "display_name": "运营小王",
    "permissions": ["user:read", "user:ban", "room:read", "room:close"],
    "last_login_at": "2026-04-17T00:00:00Z"
  },
  "request_id": "uuid"
}
```

### 4.3 RBAC 权限矩阵

| 角色 | 用户管理 | 房间管理 | 数据统计 | 财务操作 | 系统管理 |
|------|---------|---------|---------|---------|---------|
| `super_admin` | ✅ 读写 | ✅ 读写 | ✅ | ✅ | ✅ |
| `operator` | ✅ 读写 | ✅ 读写 | ✅ | ❌ | ❌ |
| `cs` | 只读 | ✅ 读写 | ❌ | ❌ | ❌ |
| `finance` | ❌ | ❌ | ✅ | ✅ | ❌ |

### 4.4 GET /api/v1/admin/rooms — 查询房间列表（后台）

**认证**：需要 Admin JWT（Bearer Token），`finance` 角色无 `RoomRead` 权限（403）。  
**描述**：管理员查看全状态房间列表，支持分页、状态过滤、关键词搜索。与 C 端接口的区别：可见 `closed` 房间，不按热度排序。

**查询参数**：

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `page` | integer | 否 | 1 | ≥1，否则 400 |
| `page_size` | integer | 否 | 20 | 1–100，否则 400 |
| `status` | string | 否 | 全部 | `active` / `closed`，其他值 400 |
| `keyword` | string | 否 | — | 按房间标题模糊搜索 |

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "total": 150,
    "page": 1,
    "page_size": 20,
    "items": [
      {
        "room_id": "uuid",
        "title": "string",
        "room_type": "normal | password | paid",
        "member_count": 0,
        "max_members": 50,
        "status": "active | closed",
        "owner_id": "uuid",
        "owner_nickname": "string",
        "owner_avatar": "string | null",
        "created_at": "RFC3339"
      }
    ]
  },
  "request_id": "uuid"
}
```

**错误码**：

| HTTP | code | 说明 |
|------|------|------|
| 401 | 40101 | 未提供 Token、签名无效或 C 端 JWT（iss 不匹配） |
| 401 | 40102 | Token 已过期 |
| 403 | 40301 | `finance` 角色无 `RoomRead` 权限 |
| 400 | 40003 | 参数校验失败（`page` / `page_size` / `status` 非法） |

### 4.5 获取房间详情（后台）

**接口**：`GET /api/v1/admin/rooms/:id`  
**认证**：需要 Admin JWT（Bearer Token），finance 角色无权限  
**描述**：管理员查看指定房间的完整信息，可见 active 和 closed 状态房间，软删除房间返回 404。

与 C 端 `GET /api/v1/rooms/:id` 的区别：
- 可见 closed 状态房间（C 端仅 active）
- 响应多出 `status`、`updated_at` 字段
- 需要 Admin JWT

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
    "status": "active | closed",
    "room_type": "normal | password | paid",
    "member_count": 0,
    "max_members": 50,
    "owner": {
      "user_id": "uuid",
      "nickname": "string",
      "avatar": "string | null"
    },
    "mic_slots": [],
    "created_at": "RFC3339",
    "updated_at": "RFC3339"
  },
  "request_id": "uuid"
}
```

**错误码**：

| HTTP | code | 说明 |
|------|------|------|
| 400 | 40003 | room_id 格式非法（非 UUID） |
| 401 | 40101 | 未提供 Token、签名无效或 C 端 JWT |
| 401 | 40102 | Token 已过期 |
| 403 | 40301 | finance 角色无 RoomRead 权限 |
| 404 | 40400 | 房间不存在或已软删除 |

---

### 4.6 强制关闭房间

**接口**：`DELETE /api/v1/admin/rooms/:id`  
**认证**：需要 Admin JWT（Bearer Token），仅 super_admin 和 operator 角色有 RoomForceClose 权限  
**描述**：管理员强制关闭任意 active 房间，无需是房主。与 C 端 `DELETE /api/v1/rooms/:id` 的核心区别：无 owner 检查，管控范围为全部房间。MVP 阶段不广播 WebSocket 事件。

**路径参数**：

| 参数 | 类型 | 说明 |
|------|------|------|
| id | UUID | 要强制关闭的房间 ID |

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
| 401 | 40101 | 未提供 Token、签名无效或 C 端 JWT |
| 401 | 40102 | Token 已过期 |
| 403 | 40301 | 角色无 RoomForceClose 权限（finance、cs 角色） |
| 404 | 40400 | 房间不存在或已软删除 |
| 409 | 40901 | 房间已处于 closed 状态 |

---

## 五、RTC Token（预留）

> ⚠️ 本节为预留设计，将在模块3（T-00012 及后续任务）实现时正式落地。

### 5.1 POST /api/v1/rtc/token

获取 RTC 频道 token。**需要 JWT 认证**。

**Request Body**:
```json
{
  "channel_id": "room_uuid",
  "role": "publisher"
}
```

**Success Response (200)**:
```json
{
  "code": 0,
  "message": "ok",
  "data": {
    "rtc_token": "006xxx...",
    "channel_id": "room_uuid",
    "uid": 12345,
    "expires_in": 3600
  },
  "request_id": "uuid"
}
```

**设计要点**:
- `role`: `publisher`（上麦用户）/ `subscriber`（听众）
- `uid`: RTC 频道内的数字 ID，由 Server 从 user_id 映射
- `expires_in`: token 有效期（秒），客户端应在过期前主动续签
- Server 通过 `RtcTokenProvider` 防腐层签发，不暴露具体 SDK（Agora/ZEGO 等）术语
- 客户端收到 RTC SDK 的 `onTokenPrivilegeWillExpire` 回调时，重新调用此接口

### 5.2 RTC Provider 三端抽象边界

| 端 | 抽象层 | 职责 | 不做的事 |
|----|--------|------|----------|
| **Server** | `RtcTokenProvider` trait | 签发 token、映射 uid、配置频道 | 不管客户端推拉流 |
| **Web** | `RtcClientAdapter` interface | `join/leave/publish/unpublish/renewToken` | 不直接引用 Agora/ZEGO JS SDK |
| **Android** | `IMediaService` interface | `joinChannel/leaveChannel/publishAudio/muteAudio/renewToken` | 不在 feature 层直接依赖 SDK |

---

## 六、WebSocket 信令（预留）

> 将在模块3 WebSocket 连接管理（T-00012）实现时正式定义。以下为设计预留。

### 6.1 连接建立

```
ws://host/ws?token=<JWT>
```

### 6.2 心跳

- 客户端每 15 秒发送 `{"type":"ping"}`
- 服务端回复 `{"type":"pong"}`
- 30 秒无心跳自动断开

### 6.3 消息通用格式

```json
{
  "type": "EventType",
  "msg_id": "uuid",
  "payload": {},
  "timestamp": 1713312000
}
```

---

## 七、数据模型（模块1相关）

### 7.1 users 表

```sql
CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    phone       VARCHAR(20) NOT NULL UNIQUE,
    nickname    VARCHAR(50) NOT NULL,
    avatar      TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    coin_balance BIGINT NOT NULL DEFAULT 0,
    vip_level    SMALLINT NOT NULL DEFAULT 0,
    deleted_at  TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_users_phone ON users(phone) WHERE deleted_at IS NULL;
```

### 7.2 验证码存储 (Redis)

> 验证码使用 Redis 存储，不使用 PostgreSQL 表。

**Redis Key 设计**:

| Key 模式 | 类型 | TTL | 说明 |
|----------|------|-----|------|
| `sms:code:{phone}` | Hash | 300s | 验证码内容 + 尝试次数 |
| `sms:cooldown:{phone}` | String | 60s | 发送冷却标记 |
| `sms:daily:{phone}:{date}` | String (INCR) | 86400s | 每日发送计数 |

**`sms:code:{phone}` Hash 结构**:
```
HSET sms:code:+966512345678 code "123456" attempts 0 max_attempts 5
EXPIRE sms:code:+966512345678 300
```

**验证流程**:
1. 发送验证码前：检查 `sms:cooldown:{phone}` 是否存在（冷却中）；检查 `sms:daily:{phone}:{date}` 是否超限
2. 发送成功后：写入 `sms:code:{phone}` (TTL 300s) + `sms:cooldown:{phone}` (TTL 60s) + INCR `sms:daily:{phone}:{date}`
3. 登录校验时：HGET `sms:code:{phone}` 取 code 比对，HINCRBY attempts 1，超过 max_attempts 返回 40105
4. 校验成功后：DEL `sms:code:{phone}` 使验证码一次性作废

### 7.3 admins 表

```sql
CREATE TABLE admins (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username     VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(200) NOT NULL,
    role         VARCHAR(20) NOT NULL DEFAULT 'operator',
    display_name VARCHAR(100),
    is_active    BOOLEAN NOT NULL DEFAULT true,
    last_login_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 角色枚举约束
ALTER TABLE admins ADD CONSTRAINT chk_admin_role
    CHECK (role IN ('super_admin', 'operator', 'cs', 'finance'));
```

**初始数据**: 部署时通过 migration seed 插入默认 super_admin 账号。

### 7.4 admin_logs 表

```sql
CREATE TABLE admin_logs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id    UUID NOT NULL REFERENCES admins(id),
    action      VARCHAR(50) NOT NULL,
    target_type VARCHAR(20),
    target_id   UUID,
    detail      JSONB,
    ip_address  INET,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_admin_logs_admin_id ON admin_logs(admin_id, created_at DESC);
CREATE INDEX idx_admin_logs_action ON admin_logs(action, created_at DESC);
```

**action 枚举**: `admin_login`, `ban_user`, `unban_user`, `close_room`, `broadcast_notice`, `create_admin`, `update_admin`

---

## 八、Provider 配置模型

### 8.1 SMS Provider

通过环境变量 / `config/*.toml` 注入：

```toml
[sms]
provider = "twilio"           # twilio | aws_sns | mock
# Twilio 专属
twilio_account_sid = "${TWILIO_ACCOUNT_SID}"
twilio_auth_token = "${TWILIO_AUTH_TOKEN}"
twilio_from_number = "${TWILIO_FROM_NUMBER}"
```

- `mock` 模式在本地开发/测试环境使用，不实际发送短信，验证码固定为 `000000`
- Provider 实现在 `infrastructure/third_party/sms/` 目录下

### 8.2 RTC Provider（预留）

```toml
[rtc]
provider = "agora"            # agora | zego | mock
app_id = "${RTC_APP_ID}"
app_certificate = "${RTC_APP_CERTIFICATE}"
```

- Provider 实现在 `infrastructure/third_party/rtc/` 目录下
- `mock` 模式返回固定 token 字符串，供开发调试

---

**文档变更历史**:
- 2026-04-17: 初始版本，定义模块1认证契约 + RTC/WS 预留
- 2026-04-17: v0.2 — 删除 register 端点改为一步登录；验证码存储从 PG 改 Redis；新增 Admin Server 认证契约（§四）；新增 admins/admin_logs 表；users 表增加 coin_balance/vip_level
- 2026-04-19: v0.4 — 新增 §三 3.2 `GET /api/v1/rooms` 接口定义：查询参数（page/size）、items 字段说明、排序过滤规则（T-00008）
- 2026-04-20: v0.5 — 新增 §3.3 获取房间详情（T-00009）
- 2026-04-21: v0.6 — 新增 §3.4 关闭房间（T-00010），新增错误码 40301/40901
- 2026-04-22: v0.7 — 新增 §4.4 Admin 房间列表接口（T-10004）
- 2026-04-23: v0.8 — 新增 §4.5 Admin 房间详情接口（T-10005）
- 2026-04-24: v0.9 — 新增 §4.6 Admin 强制关闭房间（T-10006）
