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
