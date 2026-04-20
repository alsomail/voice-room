# 10. 外部设施防腐层（Anti-Corruption Layer）

## 10.1 客户端防腐

严禁第三方 SDK 直接耦合业务层与 UI 层。

**Android 示例：**
```kotlin
interface IMediaService {
    fun joinChannel(channelId: String, token: String, uid: Long)
    fun leaveChannel()
    fun muteLocalAudio(muted: Boolean)
    fun observeNetworkState(): Flow<MediaNetworkState>
}
```

```kotlin
interface IAnalyticsService {
    fun trackEvent(name: String, payload: Map<String, Any?>)
    fun setUserProperties(props: Map<String, Any?>)
}
```

```kotlin
interface ICrashReporter {
    fun logBreadcrumb(message: String)
    fun reportError(throwable: Throwable, context: Map<String, Any?> = emptyMap())
}
```

**Web 示例：**
```typescript
export interface IMediaService {
  joinChannel(params: JoinChannelParams): Promise<void>
  leaveChannel(): Promise<void>
  muteLocalAudio(muted: boolean): Promise<void>
  onNetworkStateChange(cb: (state: MediaNetworkState) => void): () => void
}
```

```typescript
export interface IAnalyticsService {
  trackEvent(eventName: string, payload: Record<string, unknown>): void
  setUserProperties(props: Record<string, unknown>): void
}
```

```typescript
export interface ICrashReporter {
  logBreadcrumb(message: string): void
  reportError(error: Error, context?: Record<string, unknown>): void
}
```

## 10.2 服务端防腐

Server 必须通过 `infrastructure/third_party/` 处理第三方服务：
- RTC Token 签发
- Webhook 回调验签与落库
- 审核/推送/短信/支付等 REST API 调用
- 超时、重试、熔断、降级
- 第三方错误码转换为内部错误码

禁止事项：
- Controller 直接调用第三方 SDK
- Service 直接拼第三方 HTTP 请求
- 第三方错误码直接透传客户端
