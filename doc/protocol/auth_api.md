# 二、认证模块 (Auth)

## 2.1 POST /api/v1/auth/verification-codes

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

## 2.2 POST /api/v1/auth/login

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

## 2.3 GET /api/v1/users/me

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
