# 一、通用约定

## 1.1 基础地址

### App Server (C 端业务后端)

| 环境 | HTTP Base URL | WebSocket URL |
|------|--------------|---------------|
| 本地开发 | `http://localhost:3000/api/v1` | `ws://localhost:3000/ws` |
| 测试环境 | `https://test-api.voiceroom.example/api/v1` | `wss://test-api.voiceroom.example/ws` |
| 生产环境 | `https://api.voiceroom.example/api/v1` | `wss://api.voiceroom.example/ws` |

### Admin Server (B 端管理后端)

| 环境 | HTTP Base URL |
|------|--------------|
| 本地开发 | `http://localhost:3001/api/v1/admin` |
| 测试环境 | `https://test-admin-api.voiceroom.example/api/v1/admin` |
| 生产环境 | `https://admin-api.voiceroom.example/api/v1/admin` |

> Admin Server 仅 HTTP，不提供 WebSocket。通过 VPN 访问，不对公网暴露。

## 1.2 请求通用头

| Header | 必需 | 说明 |
|--------|------|------|
| `Content-Type` | 是 | `application/json` |
| `Authorization` | 条件 | `Bearer <JWT>`，需要鉴权的接口必传 |
| `X-Request-Id` | 否 | 请求追踪 ID，若不传则 Server 自动生成并在响应头回传 |
| `X-Device-Id` | 否 | 客户端设备标识，用于风控与埋点 |
| `Accept-Language` | 否 | `ar` / `en`，默认 `ar` |

## 1.3 统一响应结构

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

## 1.4 错误码规范

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

### 模块1 错误码表

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

### 模块2 错误码表（房间模块）

| 错误码 | HTTP Status | 含义 | 触发场景 |
|--------|-------------|------|----------|
| `40003` | 400 | 参数校验失败 | 标题为空 / 超 30 字符、`room_type` 非法枚举、密码房未提供密码 |
| `40900` | 409 | 用户已有活跃房间 | 同一用户尝试创建第二个 `active` 状态房间（DB 唯一偏滤索引兜底） |

## 1.5 分页约定

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

## 1.6 幂等策略

| 接口 | 幂等性 | 策略 |
|------|--------|------|
| 发送验证码 | 冷却期幂等 | 60 秒冷却期内重复请求返回 `42901`；冷却期后重新发送新验证码 |
| 登录 | 可重试+自动注册 | 手机号不存在时自动创建用户；同一验证码有效期内可多次尝试（不超过 5 次） |
| 获取用户信息 | 只读幂等 | GET 请求，天然幂等 |

---

## §4 字段命名铁律 — snake_case 强制

> **强制规定**（不得例外）

所有协议字段（WS 信令 payload、HTTP 请求/响应 body、Redis Pub/Sub 消息）**必须使用 `snake_case`** 命名。

| 规范示例 | 禁止写法 |
|---------|---------|
| `user_id` | `userId`, `UserID` |
| `mic_index` | `micIndex`, `MicIndex` |
| `room_id` | `roomId`, `RoomID` |
| `created_at` | `createdAt`, `CreateAt` |
| `has_password` | `hasPassword`, `HasPwd` |

- `type` 字段的枚举值使用 **PascalCase**（如 `"JoinRoom"`, `"MicTaken"`）——这是信令类型标识符，不是字段名，不受本条约束。
- Redis Pub/Sub 事件的 `type` 值使用 **snake_case**（如 `"ban_user"`, `"close_room"`），与服务端 serde 标签对齐。
- 违反命名约定的字段不得合并入主干，CI audit 脚本会自动检测。

---

## §5 WS payload 嵌套铁律

> **强制规定**

所有 WebSocket 信令的业务字段**必须嵌套在顶层 `payload` 对象内**，不得直接暴露在 envelope 根层级。

**合法 envelope 结构（§5 规定）：**
```json
{
  "type": "TakeMic",
  "msg_id": "uuid-v4",
  "timestamp": 1700000000000,
  "payload": {
    "mic_index": 2
  }
}
```

**禁止的扁平化结构：**
```json
{
  "type": "TakeMic",
  "mic_index": 2
}
```

- `type`、`msg_id`、`timestamp` 是 envelope 保留字段，允许在根层级。
- 所有其他字段**必须**放入 `payload`。
- 对应 JSON Schema 使用 `"additionalProperties": false` 强制此约定。

---

## §6 envelope 双 ID 铁律

> **强制规定**

所有 C→S 信令**必须**携带 `msg_id`（客户端生成的 UUID v4）。服务端 S→C 应答**必须**回显相同的 `msg_id` 并附带服务端 `timestamp`（Unix 毫秒）。

**双 ID 规则：**

| 字段 | 位置 | 格式 | 说明 |
|------|------|------|------|
| `msg_id` | envelope 根层级 | UUID v4 字符串 | 客户端生成，用于请求/应答关联与幂等去重 |
| `timestamp` | envelope 根层级 | int64 Unix ms | 服务端生成时间戳，S→C 消息必须携带 |

- C→S 信令：`msg_id` 必填，`timestamp` 可选。
- S→C 应答：`msg_id` 回显原始请求，`timestamp` 必填。
- S→Room 广播：由服务端分配全局唯一 `msg_id`，`timestamp` 必填。
- 重连补发场景中，客户端通过 `last_msg_id` 游标拉取遗漏消息。

> ✅ **统一声明**（T-00108 完成）：所有信令的 `timestamp` 字段统一为 Unix 毫秒（`timestamp_millis`）。三端已完成全量修正，不再有秒级与毫秒级的混用。

> ⚠️ **待落地说明**：S→Room 广播的 msg_id 字段为设计目标，当前实现（T-00100 冻结阶段）尚未携带。广播 schema 中 msg_id 不在 required 列表中，以反映当前事实。待后续 Task 补齐后修改。
