# 六、WebSocket 信令（预留）

> 将在模块3 WebSocket 连接管理（T-00012）实现时正式定义。以下为设计预留。

## 6.1 连接建立

```
ws://host/ws?token=<JWT>
```

## 6.2 心跳

- 客户端每 15 秒发送 `{"type":"ping"}`
- 服务端回复 `{"type":"pong"}`
- 30 秒无心跳自动断开

## 6.3 消息通用格式

```json
{
  "type": "EventType",
  "msg_id": "uuid",
  "payload": {},
  "timestamp": 1713312000
}
```
