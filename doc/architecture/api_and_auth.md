# 7. 接口契约、鉴权与安全

## 7.1 HTTP 统一返回体

```json
{
  "code": 0,
  "message": "ok",
  "data": {},
  "request_id": "01HRX8S8N5R7N7Y3K1X4C0X9D2"
}
```

约定：
- `code = 0` 表示成功。
- `message` 为简短描述。
- `data` 无数据时返回 `null`。
- `request_id` 用于链路追踪与排障。

## 7.2 JWT 鉴权与中间件设计

**HTTP 鉴权中间件必须实现：**
1. 从 `Authorization: Bearer <token>` 读取 Access Token。
2. 校验签名、过期时间、`device_id`、`sid`、`jti`。
3. **强制查询会话状态**，确认未被踢下线、未注销、未封禁。
4. 注入 `AuthContext` 到请求上下文。

## 7.3 WebSocket 鉴权与 Session 绑定

**连接建立流程：**
1. 客户端先通过 HTTP 获取 `join_ticket`。
2. `join_ticket` 包含 `user_id`、`room_id`、`device_id`、`nonce`、`expire_at`。
3. 客户端发起 WS Upgrade，请求头带 Token 与 Ticket。
4. Server 校验 JWT 与 Ticket 有效性。
5. 创建 `WsSession`，绑定 `conn_id`、`user_id`、`room_id`、`device_id`、`sid`、`joined_at`。

**防炸房策略：**
- 单用户单房间单设备唯一连接。
- 同 IP / 同 user / 同 room 建立连接频率限流。
- 未通过鉴权的连接在 Upgrade 前拒绝。
- 房间广播按 `room_id` 隔离，严禁全局广播。
- 入房必须先注册 Session，再允许订阅房间事件。
