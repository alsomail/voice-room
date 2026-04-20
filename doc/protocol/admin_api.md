# 四、Admin 认证模块 (Admin Auth)

> Admin Server 独立部署，使用独立的管理员账号体系。

## 4.1 POST /api/v1/admin/login

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

---

## 4.2 GET /api/v1/admin/me

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

---

## 4.3 RBAC 权限矩阵

| 角色 | 用户管理 | 房间管理 | 数据统计 | 财务操作 | 系统管理 |
|------|---------|---------|---------|---------|---------|
| `super_admin` | ✅ 读写 | ✅ 读写 | ✅ | ✅ | ✅ |
| `operator` | ✅ 读写 | ✅ 读写 | ✅ | ❌ | ❌ |
| `cs` | 只读 | 只读 | ❌ | ❌ | ❌ |
| `finance` | ❌ | ❌ | ✅ | ✅ | ❌ |

---

## 4.4 GET /api/v1/admin/rooms — 查询房间列表（后台）

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

---

## 4.5 获取房间详情（后台）

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

## 4.6 强制关闭房间

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
