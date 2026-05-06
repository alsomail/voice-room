# 八、Provider 配置模型

## 8.1 SMS Provider

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

## 8.2 RTC Provider（预留）

```toml
[rtc]
provider = "agora"            # agora | zego | mock
app_id = "${RTC_APP_ID}"
app_certificate = "${RTC_APP_CERTIFICATE}"
```

- Provider 实现在 `infrastructure/third_party/rtc/` 目录下
- `mock` 模式返回固定 token 字符串，供开发调试

---

## 8.3 Redis Pub/Sub — admin:events 频道

服务端使用 Redis Pub/Sub 在多节点间广播管理员操作事件。

**频道名称**: `admin:events`

**消息格式**（统一 envelope）：
```json
{
  "type": "<event_type>",
  "payload": { ... },
  "admin_id": "<uuid>",
  "ts": 1700000000000
}
```

### 事件类型索引

| type 值 | 标题 | Schema | 描述 |
|---------|------|--------|------|
| `ban_user` | BanUser | [schemas/pubsub/BanUser.schema.json](schemas/pubsub/BanUser.schema.json) | 封禁指定用户 |
| `unban_user` | UnbanUser | [schemas/pubsub/UnbanUser.schema.json](schemas/pubsub/UnbanUser.schema.json) | 解除封禁 |
| `close_room` | CloseRoom | [schemas/pubsub/CloseRoom.schema.json](schemas/pubsub/CloseRoom.schema.json) | 强制关闭房间 |
| `broadcast_notice` | BroadcastNotice | [schemas/pubsub/BroadcastNotice.schema.json](schemas/pubsub/BroadcastNotice.schema.json) | 全局公告广播 |

### BanUser

**payload**: `{ "user_id": "<uuid>" }`

触发来源: Admin REST `POST /api/admin/users/:id/ban`
服务端实现: `app/server/src/events/admin_event.rs::AdminEvent::BanUser`

### UnbanUser

**payload**: `{ "user_id": "<uuid>" }`

触发来源: Admin REST `DELETE /api/admin/users/:id/ban`
服务端实现: `app/server/src/events/admin_event.rs::AdminEvent::UnbanUser`

### CloseRoom

**payload**: `{ "room_id": "<uuid>" }`

触发来源: Admin REST `POST /api/admin/rooms/:id/close`
服务端实现: `app/server/src/events/admin_event.rs::AdminEvent::CloseRoom`

### BroadcastNotice

**payload**: `{ "message": "<string>" }`

触发来源: Admin REST `POST /api/admin/broadcast`
服务端实现: `app/server/src/events/admin_event.rs::AdminEvent::BroadcastNotice`

---

## §8.4 协议路径绑定与双端实现

**发布方实现**：[adminServer 协议入口索引](../arch/adminServer/index.md#redis-pubsub)  
**消费方实现**：[server 协议入口索引](../arch/server/index.md#redis-pubsub)
