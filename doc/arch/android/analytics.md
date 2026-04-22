<!--
[AI 写入规约]
1. 本文件记录 Android Analytics 防腐层（core/analytics/）的架构设计与实现状态。
2. 相关 Task：T-30034（Analytics 防腐层 + Sentry 集成）。
3. 更新时请同步更新 index.md 的能力全景矩阵。
-->

# Android Analytics 防腐层架构

## 一、概述

`core/analytics/` 包是 Android 端的观测性防腐层，封装所有分析/崩溃上报行为，禁止业务层直接 `import io.sentry.*`。
关联：[T-30034 TDS](../../tds/android/T-30034.md) | [phase1_observability](../../product/phase1_observability.md)

---

## 二、目录结构

```
core/analytics/
  AnalyticsPort.kt           // 防腐层接口（含 ConsentMode 枚举）
  AnalyticsModule.kt         // Hilt 模块（预留；MVP 阶段由 AppContainer 手动注入）
  EventKey.kt                // 事件名常量 object
  impl/
    SentryAnalytics.kt       // Sentry 防腐层实现，内含 SentryHub 内部接口（可注入 Fake 测试）
    NoopAnalytics.kt         // 空操作实现（测试 / CrashOnly 回退）
  privacy/
    SensitiveFilter.kt       // 手机号 / JWT 脱敏过滤器（纯 Kotlin，可 JVM 单测）
```

---

## 三、AnalyticsPort 接口设计

```kotlin
// core/analytics/AnalyticsPort.kt
interface AnalyticsPort {
    fun track(event: String, properties: Map<String, Any?> = emptyMap())
    fun setUser(userId: String?, traits: Map<String, Any?> = emptyMap())
    fun captureException(throwable: Throwable, extras: Map<String, Any?> = emptyMap())
    fun setConsent(mode: ConsentMode)
}

enum class ConsentMode {
    /** 全量上报：Crash + 行为事件均上报 */
    FULL,
    /** 合规豁免：Crash 上报，行为事件不上报（用户仅同意 Crash）*/
    CRASH_ONLY,
    /** 全部关闭：不上报任何事件 */
    NONE
}
```

> **注**：TDS 方案中使用 `All/CrashOnly/None`，代码实现改为 `FULL/CRASH_ONLY/NONE`（全大写枚举惯例），含义一致。

---

## 四、SentryAnalytics 实现

### 4.1 SentryHub 内部接口（可测试解耦）

```kotlin
// core/analytics/impl/SentryAnalytics.kt
internal interface SentryHub {
    fun captureException(t: Throwable, extras: Map<String, Any?>)
    fun setUser(userId: String?)
    fun setExtra(key: String, value: String)
}
```

- **DefaultSentryHub**：MVP 阶段为 Stub（使用 `Log.e()` 占位，未添加 `io.sentry:sentry-android` Gradle 依赖）。添加依赖后取消注释即可激活。
- **FakeSentryHub**：测试注入，记录 `captureCount`、`setUserCount` 等计数，供 JVM 单测断言。

### 4.2 ConsentMode 路由

| ConsentMode | `track()` | `captureException()` | `setUser()` |
|-------------|-----------|----------------------|-------------|
| FULL        | 上报       | 上报                  | 写入         |
| CRASH_ONLY  | **跳过**   | 上报                  | 写入         |
| NONE        | **跳过**   | **跳过**              | **跳过（待修复 MEDIUM-02）** |

### 4.3 BuildConfig 注入

```kotlin
// app/build.gradle.kts
buildConfigField("String", "SENTRY_DSN", "\"${project.findProperty("SENTRY_DSN") ?: ""}\"")
```

- `local.properties` 或 CI Secret 提供 dev/prod DSN。
- `opts.environment = BuildConfig.BUILD_TYPE`（dev / staging / prod 区分）。

---

## 五、SensitiveFilter 脱敏策略

```kotlin
// core/analytics/privacy/SensitiveFilter.kt
class SensitiveFilter {
    private val phoneRegex = Regex("""\+?\d{7,15}""")
    private val jwtRegex   = Regex("""eyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+""")

    /** 脱敏 SentryEvent（传入 beforeSend callback） */
    fun scrub(event: SentryEvent) { /* 遍历 message/extras 替换为 *** */ }

    /** ⚠️ MEDIUM-01 待修复：当前将所有异常包装为 RuntimeException，导致 Sentry 异常类型分组失效 */
    fun scrubThrowable(t: Throwable): Throwable = RuntimeException(scrubString(t.message ?: ""))
}
```

**脱敏规则**：
- 手机号：`+?\d{7,15}` → `***`（覆盖国际格式，如 `+966512345678`）
- JWT：`eyJ[...]..[...]` → `***`

---

## 六、NoopAnalytics

```kotlin
// core/analytics/impl/NoopAnalytics.kt
class NoopAnalytics : AnalyticsPort {
    override fun track(event: String, properties: Map<String, Any?>) = Unit
    override fun setUser(userId: String?, traits: Map<String, Any?>) = Unit
    override fun captureException(throwable: Throwable, extras: Map<String, Any?>) = Unit
    override fun setConsent(mode: ConsentMode) = Unit
}
```

用途：
- 单元测试默认注入（不依赖 Sentry SDK）
- `AppContainer.fromBuildConfig()` MVP 阶段默认注入（HIGH-02 已知）
- `ConsentMode.CRASH_ONLY` 非 Crash 事件回退

---

## 七、AppContainer 注入

```kotlin
// common/AppContainer.kt
val analyticsPort: AnalyticsPort = NoopAnalytics()  // MVP 阶段；SDK 激活后切换为 SentryAnalytics
```

**生产激活路径（HIGH-01 / HIGH-02）**：
1. 在 `app/build.gradle.kts` 添加 `implementation("io.sentry:sentry-android:7.x.x")`
2. 取消 `DefaultSentryHub` 中 Sentry SDK 调用注释
3. `AppContainer.fromBuildConfig()` 切换 `analyticsPort = SentryAnalytics(ctx, filter)`

---

## 八、CI 静态约束脚本

```bash
# scripts/check_no_sentry_imports.sh
#!/usr/bin/env bash
# 检查业务层是否直接引用 io.sentry.*（防腐层以外禁止）
! grep -r "io\.sentry\." \
    app/android/app/src/main/java/com/voiceroom \
    --include="*.kt" \
    --exclude-dir="core/analytics/impl"
# 返回非零时 CI fail
```

> **LOW-01 改进建议**：`--exclude-dir="impl"` 当前过宽，建议改为精确路径过滤。

---

## 九、测试覆盖

| 测试文件 | 用例数 | 说明 |
|----------|--------|------|
| `SensitiveFilterTest.kt` | 15 | SF-01~SF-15：手机号/JWT 脱敏边界 |
| `AnalyticsPortBehaviorTest.kt` | 12 | AP-01~AP-12：ConsentMode 路由正确性 |
| `SentryAnalyticsTest.kt` | 12 | SA-01~SA-12：FakeSentryHub 注入，无需真实 Sentry 连接 |
| `BuildConfigAnalyticsTest.kt` | 3 | BC-01~BC-04：BuildConfig 注入 + AppContainer 单例 |
| **合计** | **42** | 全部通过（failures=0, errors=0） |

---

## 十、已知问题与待修复项

### MEDIUM-01：scrubThrowable 类型丢失
- **现状**：`SensitiveFilter.scrubThrowable()` 将所有异常包装为 `RuntimeException`，导致 Sentry 异常类型分组失效。
- **建议修复**：改为在 `beforeSend` callback 内脱敏 `event.message`，保留原始异常类型。
- **计划**：T-30035 或独立 Task 中修复。

### MEDIUM-02：setUser 缺 ConsentMode.None 守卫
- **现状**：`SentryAnalytics.setUser()` 在 `ConsentMode.NONE` 时仍写入用户身份。
- **建议修复**：加守卫 `if (currentConsent == ConsentMode.NONE) return`。
- **计划**：T-30035 或下一个独立 Task 补充。

### HIGH-01：DefaultSentryHub 为 Stub
- **现状**：`io.sentry:sentry-android` 未添加为 Gradle 依赖，所有 SDK 调用均为 `Log.e()` 占位。
- **激活步骤**：见第七节"生产激活路径"。

### HIGH-02：AppContainer 注入 NoopAnalytics
- **现状**：`AppContainer.fromBuildConfig()` 默认注入 `NoopAnalytics()` 而非 `SentryAnalytics`。
- **激活步骤**：同 HIGH-01，SDK 依赖添加后同步切换。

---

## 十一、TDD 验收结果（T-30034）

| 用例 | 状态 | 说明 |
|------|------|------|
| A34-01 业务层 `grep io.sentry` = 0 | ✅ | `check_no_sentry_imports.sh` 验证通过 |
| A34-02 captureException → FakeSentryHub.captureCount 递增 | ✅ | SA-01, SA-11 |
| A34-03 BuildConfig.SENTRY_DSN 编译时注入 | ✅ | BC-01, BC-02 |
| A34-04 CrashOnly 模式 track() 跳过、captureException 仍工作 | ✅ | SA-05, SA-06 |
| A34-05 `+966512345678` 脱敏为 `***` | ✅ | SF-01, SA-02, SA-10 |
| A34-06 JWT extras 中被脱敏 | ✅ | SF-03, SA-03 |
| A34-07 ANR 检测（`opts.isEnableAnrDetection`） | ℹ️ | MVP 阶段 SDK 未添加依赖，注释保留 |
| A34-08 AppContainer.analyticsPort 可从容器获取 | ✅ | BC-04 |

---

## 十二、EventReportClient 主链路（T-30035）

关联：[T-30035 TDS](../../tds/android/T-30035.md)

### 12.1 目录结构

```
core/analytics/
  EventReportClient.kt              // 主入口（track 门控 + flush 调度）
  queue/
    EventQueueEntity.kt             // 队列实体（接口化设计，无 Room 注解）
    EventQueueDao.kt                // 队列 DAO 接口
    InMemoryEventQueueDao.kt        // 内存实现（测试/MVP 阶段）
  throttle/
    Throttler.kt                    // flush 触发器
  transport/
    Transport.kt                    // 传输接口 + SendOutcome
    WsTransport.kt                  // WebSocket 传输
    HttpTransport.kt                // HTTP fallback 传输
  session/
    SessionManager.kt               // session UUID 管理
  context/
    CommonPropsProvider.kt          // 公共属性注入

core/consent/
  ConsentStore.kt                   // 持久化存储接口 + InMemory 实现
  DataStoreConsentStore.kt          // Java Properties 文件持久化（生产实现）
  ConsentRepository.kt              // 同意状态管理
  PrivacyConsentDialog.kt           // Compose 隐私弹窗
```

### 12.2 EventReportClient 主入口

```kotlin
class EventReportClient @Inject constructor(
    private val queueDao: EventQueueDao,
    private val throttler: Throttler,
    private val wsTransport: WsTransport,
    private val httpTransport: HttpTransport,
    private val consent: ConsentRepository,
    private val commonProps: CommonPropsProvider,
    private val sessionManager: SessionManager,
    private val analyticsPort: AnalyticsPort     // 错误时 captureException
) {
    fun track(event: String, properties: Map<String, Any?> = emptyMap()) {
        if (consent.mode != ConsentMode.FULL) return  // 非 FULL 模式直接丢弃（ConsentMode 门控）
        val enriched = commonProps.enrich(event, properties, sessionManager.currentId)
        queueDao.insert(enriched.toEntity())
        throttler.notify(queueDao.size())
    }
    suspend fun flush() { /* 取至多 100 条 → WS/HTTP → 成功删/失败保留，指数退避 */ }
}
```

**门控逻辑**：仅 `ConsentMode.FULL` 时入队，`CRASH_ONLY` / `NONE` 均立即返回。

### 12.3 队列策略

- **存储**：`EventQueueEntity`（`id/event_name/properties_json/session_id/client_ts/created_at`），`InMemoryEventQueueDao`（MVP，可替换为 Room 实现）
- **容量上限**：`size > 1000` → 删除最旧的（`deleteOldest(LIMIT 1 ORDER BY created_at`）
- **批次大小**：每次 flush 最多取 100 条

### 12.4 Throttler 触发条件

| 条件 | 触发时机 |
|------|---------|
| 队列 `size >= 8` | 立即 flush |
| 距上次 flush `>= 2min` | 定时 flush |
| App 进后台 `onStop` | 生命周期 flush |
| WS 重连成功 `onWsReconnected` | 补报 flush |

### 12.5 Transport 选择策略

```kotlin
suspend fun send(batch: List<Event>): Result<SendOutcome> {
    return if (wsClient.isConnected) wsTransport.send(batch)  // WS 优先
           else httpTransport.send(batch)                      // HTTP fallback
}
```

- **成功**：`queueDao.deleteByIds(outcome.acceptedIds)`
- **失败（网络错/超时）**：保留队列，指数退避（`2s → 4s → 8s → 16s → max 60s`）
- **HttpTransport**：`send()` 包装 `withContext(Dispatchers.IO) { execute() }`（Round 1 修复，防止阻塞协程线程池）

### 12.6 SessionManager

- 首次进前台生成 UUID 作为 `session_id`
- App 退后台 **30s 后**回前台 → 新建 `session_id`
- `Clock` 抽象接口便于单测注入虚拟时钟

### 12.7 CommonPropsProvider（6 个公共字段）

| 字段 | 来源 |
|------|------|
| `device_id` | 安装时生成，DataStore 持久化 |
| `app_version` | `BuildConfig.VERSION_NAME` |
| `os_version` | `Build.VERSION.RELEASE` |
| `locale` | `Locale.getDefault().toLanguageTag()` |
| `network_type` | `ConnectivityManager`（`() -> String` 函数注入，动态取值）|
| `session_id` | `SessionManager.currentId`（每次 enrich 时注入）|

> `networkTypeProvider: () -> String` 设计为构造函数参数，便于测试注入 `{ "WIFI" }` 等 fake。

### 12.8 ConsentRepository + DataStoreConsentStore

- **`ConsentStore` 接口**：`save(mode)` / `load(): ConsentMode`
- **`InMemoryConsentStore`**：内存实现，用于测试
- **`DataStoreConsentStore`**（Round 1 新增）：基于 Java Properties 文件持久化，冷重启后同意状态不丢失；`save()` 自动创建父目录；`load()` 文件损坏安全降级为 `ConsentMode.NONE`
- **`ConsentRepository`**：封装读写，`mode` 属性惰性加载，`setMode(mode)` 调用 `analyticsPort.setConsent(mode)` + `eventReportClient` 立即生效

### 12.9 PrivacyConsentDialog（Compose）

- **触发时机**：Splash 成功后 + DataStore 中 consent 未设置时显示
- **UI 结构**：
  - Title："数据收集说明" / "جمع البيانات"（双语）
  - Body：简述匿名行为 + Crash 收集，**不含手机号**
  - 两按钮：[仅 Crash] [同意全部]
- **testTag 键名**（Compose `Key()`）：
  - `Key("privacy_consent_dialog")` — 整体弹窗容器
  - `Key("btn_privacy_crash_only")` — 仅 Crash 按钮
  - `Key("btn_privacy_agree")` — 同意全部按钮

### 12.10 核心事件埋点清单（20+ 事件）

| 页面/逻辑 | 事件名称 | 触发点 |
|----------|---------|--------|
| SplashActivity | `app_launch` | `onCreate` |
| LoginViewModel | `login_request` | API 调用前 |
| LoginViewModel | `login_success` | API 成功返回 |
| LoginViewModel | `login_fail` | API 失败返回 |
| HallScreen | `hall_view` | 页面进入 |
| HallScreen | `room_card_click` | 房间卡片点击 |
| HallScreen | `create_room_click` | 创建房间按钮点击 |
| CreateRoom | `create_room_success` | API 成功 |
| CreateRoom | `create_room_fail` | API 失败 |
| RoomScreen | `room_enter` | WS JoinRoom 成功 |
| RoomScreen | `room_leave` | WS LeaveRoom |
| MicSlot | `mic_take` | WS MicTaken 结果 |
| MicSlot | `mic_leave` | WS MicLeft 结果 |
| ChatInput | `chat_send` | 发送消息后 |
| GiftPanel | `gift_panel_open` | 礼物面板打开 |
| GiftPanel | `gift_select` | 礼物选择 |
| GiftPanel | `gift_send_click` | 点击发送按钮 |
| SendFlow | `gift_send_success` | 送礼成功 |
| SendFlow | `gift_send_fail` | 送礼失败 |
| SendFlow | `insufficient_balance_dialog_shown` | 余额不足弹窗展示 |
| WalletScreen | `wallet_view` | 钱包页进入 |
| WalletScreen | `recharge_click` | 充值按钮点击 |
| RankingScreen | `ranking_view` | 榜单页进入 |
| RankingScreen | `ranking_tab_switch` | Tab 切换 |
| Profile | `profile_view` | 个人中心进入 |
| Profile | `logout_click` | 退出登录点击 |

---

## 十三、TDD 验收结果（T-30035）

**测试总数：42 个，全部通过（failures=0, errors=0）**

| 验收用例 | 状态 | 测试文件 |
|---------|------|---------|
| E35-01 `track("x")` 写入队列 | ✅ | `EventReportClientTest` |
| E35-02 队列 ≥8 条立即 flush | ✅ | `EventReportClientTest` |
| E35-03 队列 3 条 + 2min → flush | ✅ | `ThrottlerSessionTest` |
| E35-04 队列 >1000 条淘汰最旧 | ✅ | `EventReportClientTest` |
| E35-05 WS 在线 → WsTransport | ✅ | `EventReportClientTest` |
| E35-06 WS 离线 → HttpTransport | ✅ | `EventReportClientTest` |
| E35-07 断网恢复后 100% 补报 | ✅ | `EventReportClientTest` |
| E35-08 CrashOnly 模式 track 不入队 | ✅ | `EventReportClientTest` |
| E35-09 弹窗选择 DataStore 持久化 | ✅ | `ConsentRepositoryTest` + `DataStoreConsentStoreTest` |
| E35-10 核心事件集成测试 | ✅ | `EventReportClientTest` |
| E35-11 手机号/JWT 被 SensitiveFilter 过滤 | ✅ | `EventReportClientTest` |
| E35-12 session_id 30s 后刷新 | ✅ | `ThrottlerSessionTest` |

### 模块覆盖率（JVM 单元测试）

| 模块 | LINE | BRANCH | METHOD |
|------|------|--------|--------|
| `core/analytics/throttle` | 100% | 100% | 100% |
| `core/analytics/context` | 100% | — | 100% |
| `core/analytics/queue` | 79% | 100% | 70% |
| `core/analytics/session` | 75% | 100% | 67% |
| `core/analytics` (EventReportClient) | 79% | 58% | 56% |
| `core/consent` | 85%* | 80%* | 90% |

> \* `PrivacyConsentDialog.kt` 为 Compose UI，需 Instrumented Test 覆盖，JVM 阶段无法执行。
