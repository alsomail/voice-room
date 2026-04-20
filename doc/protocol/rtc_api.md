# 五、RTC Token（预留）

> ⚠️ 本节为预留设计，将在模块3（T-00012 及后续任务）实现时正式落地。

## 5.1 POST /api/v1/rtc/token

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

## 5.2 RTC Provider 三端抽象边界

| 端 | 抽象层 | 职责 | 不做的事 |
|----|--------|------|----------|
| **Server** | `RtcTokenProvider` trait | 签发 token、映射 uid、配置频道 | 不管客户端推拉流 |
| **Web** | `RtcClientAdapter` interface | `join/leave/publish/unpublish/renewToken` | 不直接引用 Agora/ZEGO JS SDK |
| **Android** | `IMediaService` interface | `joinChannel/leaveChannel/publishAudio/muteAudio/renewToken` | 不在 feature 层直接依赖 SDK |
