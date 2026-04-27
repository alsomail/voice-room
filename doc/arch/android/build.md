# Android 构建系统与多环境 productFlavors

**最后更新**: 2026-05-31  
**关联任务**: [T-30050](../tds/android/T-30050.md)（Android productFlavors 与网络安全双锁）  
**模块**: Phase 1.6 E2E 测试基建 - M2 多环境对称

---

## 一、 productFlavors 架构概览

### 1.1 设计目标

让同一台设备能并存三档 APK，每档独立指向对应环境的 API/WS/Analytics 端点：

| 维度 | Local | Staging | Prod |
|-----|-------|---------|------|
| **Product Flavor** | `local` | `staging` | `prod` |
| **applicationIdSuffix** | `.local` | `.stg` | 无 |
| **最终 applicationId** | `com.voice.room.android.local.debug` | `com.voice.room.android.stg.debug` | `com.voice.room.android.debug`（debug）；`com.voice.room.android`（release） |
| **BuildConfig.API_BASE_URL** | `http://10.0.2.2:3000/api` | `https://stg-api.example.com/api` | `https://api.example.com/api` |
| **BuildConfig.WS_URL** | `ws://10.0.2.2:3000/ws` | `wss://stg-api.example.com/ws` | `wss://api.example.com/ws` |
| **BuildConfig.APP_ENVIRONMENT** | `local` | `staging` | `prod` |
| **usesCleartextTraffic** | ✅ true | ❌ false | ❌ false |
| **NetworkSecurityConfig** | `src/local/res/xml/...` | `src/staging/res/xml/...` | `src/prod/res/xml/...` |

### 1.2 关键不变量

1. **flavor 是默认值层**：`local.properties` / 环境变量仍可 override，保证 0 回归与开发体验
2. **NetworkSecurityConfig 是系统级强约束**：staging/prod 即使代码里漏写校验，明文请求也被系统拦截（双锁机制）
3. **APP_ENVIRONMENT 是真值源**：业务代码（如 Sentry tag、日志摘要）依赖此字段而非 applicationId 后缀解析

---

## 二、 实现细节

### 2.1 build.gradle.kts 结构

**文件位置**: `app/android/app/build.gradle.kts`

#### 步骤 1：新增 flavor 维度

```kotlin
// T-30050：环境维度（local / staging / prod 三 flavor）
flavorDimensions += "env"
```

#### 步骤 2：定义 productFlavors 三块

```kotlin
productFlavors {
    // ① Local Flavor（开发调试）
    create("local") {
        dimension = "env"
        isDefault = true                          // 关键：裸 assembleDebug ≡ assembleLocalDebug
        applicationIdSuffix = ".local"
        versionNameSuffix = "-local"
        
        // D-1：local 仍允许 local.properties / ENV 覆盖（保 0 回归）
        val apiBaseUrl = resolveConfigValue(
            localProperties, "voiceRoomApiBaseUrl", "VOICE_ROOM_API_BASE_URL",
            "http://10.0.2.2:3000/api"
        )
        val wsUrl = resolveConfigValue(
            localProperties, "voiceRoomWsUrl", "VOICE_ROOM_WS_URL",
            "ws://10.0.2.2:3000/ws"
        )
        val analyticsEndpoint = resolveConfigValue(
            localProperties, "voiceRoomAnalyticsEndpoint", "VOICE_ROOM_ANALYTICS_ENDPOINT",
            "$apiBaseUrl/v1/events/batch"
        )
        
        buildConfigField("String", "API_BASE_URL",        "\"$apiBaseUrl\"")
        buildConfigField("String", "WS_URL",              "\"$wsUrl\"")
        buildConfigField("String", "ANALYTICS_ENDPOINT",  "\"$analyticsEndpoint\"")
        buildConfigField("String", "APP_ENVIRONMENT",     "\"local\"")
        manifestPlaceholders["usesCleartextTraffic"] = "true"
    }
    
    // ② Staging Flavor（测试环境）
    create("staging") {
        dimension = "env"
        applicationIdSuffix = ".stg"
        versionNameSuffix = "-stg"
        buildConfigField("String", "API_BASE_URL",        "\"https://stg-api.example.com/api\"")
        buildConfigField("String", "WS_URL",              "\"wss://stg-api.example.com/ws\"")
        buildConfigField("String", "ANALYTICS_ENDPOINT",  "\"https://stg-api.example.com/api/v1/events/batch\"")
        buildConfigField("String", "APP_ENVIRONMENT",     "\"staging\"")
        manifestPlaceholders["usesCleartextTraffic"] = "false"
    }
    
    // ③ Prod Flavor（生产环境）
    create("prod") {
        dimension = "env"
        // 无 applicationIdSuffix → 包名为 com.voice.room.android（与商店一致）
        buildConfigField("String", "API_BASE_URL",        "\"https://api.example.com/api\"")
        buildConfigField("String", "WS_URL",              "\"wss://api.example.com/ws\"")
        buildConfigField("String", "ANALYTICS_ENDPOINT",  "\"https://api.example.com/api/v1/events/batch\"")
        buildConfigField("String", "APP_ENVIRONMENT",     "\"prod\"")
        manifestPlaceholders["usesCleartextTraffic"] = "false"
    }
}
```

#### 步骤 3：修改 buildTypes（移除与 flavor 重复的字段）

```kotlin
buildTypes {
    debug {
        applicationIdSuffix = ".debug"
        versionNameSuffix = "-debug"
        // T-30050：移除 manifestPlaceholders["usesCleartextTraffic"]，完全交由 flavor 控制
    }
    release {
        isMinifyEnabled = false
        // T-30050：删除 src/{debug,release}/res/xml/network_security_config.xml 
        // 改为 src/{local,staging,prod}/res/xml/network_security_config.xml
    }
}
```

#### 步骤 4：保留 defaultConfig 兜底（D-2 防 typo）

```kotlin
defaultConfig {
    // ... 其他配置
    
    // D-2：保留 defaultConfig 中的 buildConfigField 作为 typo 兜底
    // 若某个 flavor 块字段名拼写错误，会回落此处的默认值而非编译失败
    buildConfigField("String", "API_BASE_URL",       "\"https://dev-api.example.com/api\"")
    buildConfigField("String", "WS_URL",             "\"wss://dev-api.example.com/ws\"")
    buildConfigField("String", "ANALYTICS_ENDPOINT", "\"https://dev-api.example.com/api/v1/events/batch\"")
    buildConfigField("String", "APP_ENVIRONMENT",    "\"dev\"")
}
```

### 2.2 NetworkSecurityConfig 双锁机制

**文件位置**: `app/android/app/src/{local,staging,prod}/res/xml/network_security_config.xml`

#### Local Flavor（允许明文）

`src/local/res/xml/network_security_config.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<!--
  T-30050 §2.5.1 — local flavor 网络安全配置
  允许明文流量，便于模拟器/真机直连开发服务器
-->
<network-security-config>
    <base-config cleartextTrafficPermitted="true" />
    <domain-config cleartextTrafficPermitted="true">
        <domain includeSubdomains="false">10.0.2.2</domain>
        <domain includeSubdomains="false">127.0.0.1</domain>
        <domain includeSubdomains="false">localhost</domain>
    </domain-config>
</network-security-config>
```

**说明**：
- `base-config cleartextTrafficPermitted="true"` 允许所有 HTTP 流量（含 192.168/10.x 局域网）
- `domain-config` 显式列出常见开发地址（模拟器 `10.0.2.2`、本机 `127.0.0.1`、localhost）
- 物理机调试可通过 `local.properties` 设置 LAN IP

#### Staging / Prod Flavors（强制 HTTPS）

`src/staging/res/xml/network_security_config.xml` 和 `src/prod/res/xml/network_security_config.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<!--
  T-30050 §2.5.2 — staging/prod flavor 网络安全配置
  强制使用加密连接（HTTPS+WSS），防止数据在网络上明文传输
-->
<network-security-config>
    <base-config cleartextTrafficPermitted="false" />
</network-security-config>
```

**说明**：
- `base-config cleartextTrafficPermitted="false"` 系统层强制，任何明文 HTTP 请求都会抛 `IOException`
- 即使开发人员忘记校验环境变量，系统也会在运行时拦截

### 2.3 local.properties override 链路

**文件位置**: `app/android/local.properties.example`（开发者参考）

```properties
# T-30050 — Android local.properties override 模板（按需复制为 local.properties）
#
# 优先级：local.properties (本文件键) > 环境变量 > flavor.buildConfigField 默认值 > defaultConfig 兜底
#
# 适用场景：
#   1. 物理机调试需把 10.0.2.2 替换为 LAN IP（如 192.168.1.100）
#   2. 本地复现 staging/prod 行为（不改 flavor 默认值）
#   3. 接入 Sentry 上报（dev 阶段允许空字符串）

# ── voice-room 业务 override 键 ───────────────────────────────────────────
# 仅在需要覆盖 flavor 默认值时取消注释。
# voiceRoomApiBaseUrl=http://192.168.1.100:3000/api
# voiceRoomWsUrl=ws://192.168.1.100:3000/ws
# voiceRoomAnalyticsEndpoint=http://192.168.1.100:3000/api/v1/events/batch
# voiceRoomEnvironment=local

# Sentry DSN（dev 阶段可留空；release 上线前必须填写）
# SENTRY_DSN=https://<key>@o<org>.ingest.sentry.io/<project>
```

**优先级解析**（从高到低）：

1. **local.properties 文件内的键**（最高优先）→ 开发者本地覆盖
2. **环境变量**（如 `VOICE_ROOM_API_BASE_URL`）→ CI 自动注入
3. **flavor buildConfigField 默认值** → 不同 flavor 各自默认值
4. **defaultConfig buildConfigField 值**（最低优先）→ D-2 typo 兜底

核心实现函数：

```kotlin
fun resolveConfigValue(
    localProperties: Properties,
    propertyName: String,           // 本地文件键名
    envName: String,                // 环境变量名
    defaultValue: String            // 无环境变量时的默认值
): String = localProperties.getProperty(propertyName)
    ?: System.getenv(envName)
    ?: defaultValue
```

---

## 三、 构建命令

### 3.1 按 flavor 构建

```bash
# Local flavor（开发调试）
./gradlew assembleLocalDebug      # app-local-debug.apk
./gradlew assembleLocalRelease    # app-local-release.apk（需签名配置）

# Staging flavor（测试环境）
./gradlew assembleStagingDebug    # app-staging-debug.apk
./gradlew assembleStagingRelease  # app-staging-release.apk（需签名配置）

# Prod flavor（生产环境）
./gradlew assembleProdDebug       # app-prod-debug.apk
./gradlew assembleProdRelease     # app-prod-release.apk（需签名配置）

# 默认 flavor（与 local 等价，因 isDefault=true）
./gradlew assembleDebug           # 等价 assembleLocalDebug
./gradlew assembleRelease         # 等价 assembleLocalRelease
```

### 3.2 按 flavor 运行单元测试

```bash
# Local flavor
./gradlew testLocalDebugUnitTest

# Staging flavor
./gradlew testStagingDebugUnitTest

# Prod flavor
./gradlew testProdDebugUnitTest

# 所有 flavor
./gradlew testDebugUnitTest
```

---

## 四、 Flavor-Specific Test Sourceset

### 4.1 文件结构

```
app/android/app/src/
├── main/                    # 共享代码
├── test/                    # 共享单元测试
├── testLocal/               # 仅在 local flavor 下运行
│   └── java/com/voice/room/android/.../BuildConfigFlavorTest.kt
├── testStaging/             # 仅在 staging flavor 下运行
│   └── java/com/voice/room/android/.../BuildConfigStagingTest.kt
└── testProd/                # 仅在 prod flavor 下运行
    └── java/com/voice/room/android/.../BuildConfigProdTest.kt
```

### 4.2 原因与好处

**原因**：

- BuildConfig 是 flavor-specific 的编译产物
- 在共享 `test/` 目录中，同一个 `@Test` 方法会在所有 flavor 上运行
- 若测试断言 `BuildConfig.API_BASE_URL == "http://10.0.2.2:3000/api"`，这在 staging/prod flavor 下会失败

**好处**：

- 物理隔离每个 flavor 的验证逻辑
- `./gradlew testLocalDebugUnitTest` 只跑 local 特定断言
- 避免虚假失败和复杂的条件判断

### 4.3 示例测试

**src/testLocal/java/.../BuildConfigFlavorTest.kt**:

```kotlin
class BuildConfigFlavorTest {
    @Test
    fun apiBaseUrlIsLocal() {
        assertEquals("http://10.0.2.2:3000/api", BuildConfig.API_BASE_URL)
    }
    
    @Test
    fun wsUrlIsLocal() {
        assertEquals("ws://10.0.2.2:3000/ws", BuildConfig.WS_URL)
    }
    
    @Test
    fun appEnvironmentIsLocal() {
        assertEquals("local", BuildConfig.APP_ENVIRONMENT)
    }
    
    @Test
    fun cleartextIsAllowed() {
        assertTrue(cleartextAllowedFor("10.0.2.2"))  // network_security_config 校验
    }
}
```

**src/testStaging/java/.../BuildConfigStagingTest.kt**:

```kotlin
class BuildConfigStagingTest {
    @Test
    fun apiBaseUrlIsStaging() {
        assertEquals("https://stg-api.example.com/api", BuildConfig.API_BASE_URL)
    }
    
    @Test
    fun wsUrlIsSecure() {
        assertTrue(BuildConfig.WS_URL.startsWith("wss://"))
    }
    
    @Test
    fun appEnvironmentIsStaging() {
        assertEquals("staging", BuildConfig.APP_ENVIRONMENT)
    }
}
```

**src/testProd/java/.../BuildConfigProdTest.kt**:

```kotlin
class BuildConfigProdTest {
    @Test
    fun apiBaseUrlIsProd() {
        assertEquals("https://api.example.com/api", BuildConfig.API_BASE_URL)
    }
    
    @Test
    fun wsUrlIsSecure() {
        assertTrue(BuildConfig.WS_URL.startsWith("wss://"))
    }
    
    @Test
    fun appEnvironmentIsProd() {
        assertEquals("prod", BuildConfig.APP_ENVIRONMENT)
    }
}
```

---

## 五、 同设备三包并存

### 5.1 applicationId 冲突检查

由于三个 flavor 的 applicationIdSuffix 严格不同，同一台设备可同时安装三档 APK：

```bash
# 检查已安装包
adb shell pm list packages | grep voice

# 输出示例
package:com.voice.room.android.local.debug  (local-debug)
package:com.voice.room.android.stg.debug    (staging-debug)
package:com.voice.room.android.debug        (prod-debug)
```

### 5.2 安装三个版本

```bash
# 构建三个 APK
./gradlew assembleLocalDebug assembleStagingDebug assembleProdDebug

# 一次性安装三个
adb install -r build/outputs/apk/local/debug/app-local-debug.apk
adb install -r build/outputs/apk/staging/debug/app-staging-debug.apk
adb install -r build/outputs/apk/prod/debug/app-prod-debug.apk
```

### 5.3 测试明文流量禁用

```bash
# 在 staging-debug APK 上尝试 HTTP 请求（会失败）
adb logcat | grep "Cleartext HTTP traffic"

# 预期输出
E Cleartext HTTP traffic to stg-api.example.com is not permitted
```

---

## 六、 与其他端对称性

### 6.1 对称表（AppServer / AdminServer / Web / Android）

| 端 | 配置形式 | 默认值位置 | Override 链 | 关键文件 |
|----|--------|--------|------------|--------|
| **AppServer** | TOML profile | `config/default.toml` | `local.properties` > ENV > TOML 值 | `app/server/config/{default,dev,test,staging,prod}.toml` |
| **AdminServer** | TOML profile | `config/default.toml` | ENV > TOML 值 > TOML default | `app/adminServer/config/{default,dev,test,staging,prod}.toml` |
| **Web** | Vite mode | `.env.development` 等 | ENV > `.env.{mode}` 值 | `.env.{development,test,staging,production}` |
| **Android** | productFlavor | `defaultConfig` | `local.properties` > ENV > flavor 值 > defaultConfig | `app/android/app/build.gradle.kts` |

### 6.2 T-30050 与 T-00040/T-10020/T-20020 的对应

- **T-00040** (AppServer)：config 目录 + TOML profile
- **T-10020** (AdminServer)：config 目录 + TOML profile
- **T-20020** (Web)：.env 多文件 + Vite mode
- **T-30050** (Android)：productFlavor + NetworkSecurityConfig

都通过三层配置链（local override > ENV > 默认值）实现多环境切换，M2 多环境对称完整闭合。

---

## 七、 常见问题与故障排查

### 7.1 Q：为什么需要 NetworkSecurityConfig 的三个文件？

**A**：避免 flavor × buildType 复合时资源被错误合并。

- 若仅在 manifest 中通过 `usesCleartextTraffic` placeholder，staging + debug 复合时 buildType 的 xml 可能被合并，导致误开明文
- 使用 flavor-specific 源集确保编译期就选中正确的 xml

### 7.2 Q：local flavor 为什么仍需 resolveConfigValue？

**A**：保证开发体验不退化。

- 物理机调试时需要把 `10.0.2.2` 改为 LAN IP（如 `192.168.1.100`）
- D-1 设计保留 local.properties / ENV override 链，开发者可灵活切换

### 7.3 Q：applicationIdSuffix 与 Firebase 会冲突吗？

**A**：有潜在冲突（R4）。

- 当前 Firebase 未接入，故无影响
- 接入推送时需在 google-services.json 中按 flavor 拆分，详见 [doc/tds/android/T-30050.md §六 R4](../tds/android/T-30050.md)

### 7.4 Q：默认 flavor 是 local，这对 CI/CD 有影响吗？

**A**：无影响（由 `isDefault=true` + U5.1 保护）。

- 裸 `./gradlew assembleDebug` 等价 `assembleLocalDebug`
- CI 现有 pipeline 可继续使用裸命令，或显式改为 `assembleLocalDebug` 更清晰
- 默认值保护了迁移的后向兼容性

---

## 八、 相关文档与链接

- **主设计**: [T-0000E 多环境分层与切换器](../tds/infra/T-0000E.md)
- **本任务 TDS**: [T-30050 Android productFlavors](../tds/android/T-30050.md)
- **AppServer 对标**: [T-00040 AppServer config](../tds/server/T-00040.md)
- **AdminServer 对标**: [T-10020 AdminServer config](../tds/adminServer/T-10020.md)
- **Web 对标**: [T-20020 Web 多 profile env](../tds/web/T-20020.md)
- **模块 9 概览**: [doc/tasks/模块9-E2E 测试基建](../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)

---

## 九、 关键决策记录（Decision Log）

| 决策 | 原因 | 影响 |
|-----|------|------|
| `local.isDefault = true` | 保护 §2.11.3 不变量，裸 `assembleDebug` ≡ `assembleLocalDebug` | CI/开发流程零破坏，后向兼容 |
| local 仍走 `resolveConfigValue` | DX 不退化，物理机切 LAN IP 仍走 local.properties | 保留灵活性，开发舒适度 |
| staging/prod 字段写死占位 URL | 让两环境有可重现的默认行为，CI 通过 ENV override 真域名 | 本地不会意外走错环境，reduce 人为错误 |
| NetworkSecurityConfig 走 flavor sourceset | 避免 staging+debug 复合时 buildType xml 资源被合并误开 cleartext（R1） | 编译期物理隔离，系统层强约束，防御深 |
| 保留 defaultConfig buildConfigField | flavor 字段名拼写错误时回落 defaultConfig 而非崩溃（R3 防御） | 兜底保险，提升 DX |
| staging/prod 暂不配置 release 签名 | O2 超出本任务范围，上架配置留后续任务 | 本任务以 unsigned/debug-signed APK 通过验收 |
| 真实域名占位 `*.example.com` | O1 由 SRE 提供后通过 T-0000L RUNBOOK 联动 PR 替换 | 避免本 PR 跨边界，责任清晰 |

---

**维护方**: Android Team  
**状态**: ✅ 完成  
**审查**: Review Round 1 通过 (commit f00ed60)
