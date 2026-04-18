# Android Auth 模块架构

**Last Updated:** 2026-04  
**关联 Task:** [T-30001](../../tds/android/T-30001.md) — 登录页 UI (Compose) · [T-30002](../../tds/android/T-30002.md) — 登录 ViewModel  
**Entry Points:** `feature/auth/LoginScreen.kt`, `feature/auth/LoginViewModel.kt`

---

## 一、架构概述

Auth 模块采用 **Jetpack Compose + ViewModel + Repository + DataStore** 的完整 Clean Architecture 分层，
UI 层完全无状态（Stateless Composable），业务逻辑聚合在 `LoginViewModel`，
网络请求通过 `IAuthRepository` 接口隔离，JWT 持久化由 `ITokenManager` 管理，
导航事件通过 `SharedFlow<NavEvent>` 单次发射。

```
┌─────────────────── Presentation (feature/auth) ────────────────────┐
│  LoginScreen (有状态入口)                                            │
│    └─ LoginViewModel          ← viewModelScope, StateFlow/SharedFlow│
│         ├─ uiState: StateFlow<LoginUiState>                         │
│         └─ navEvent: SharedFlow<NavEvent>                           │
│                                                                      │
│  LoginScreenContent (无状态 Composable)                             │
│    ├─ PhoneInput              ← +966 国家码前缀 + 数字输入框          │
│    ├─ CountdownButton         ← 发送验证码 / 60s 倒计时              │
│    ├─ CodeInput               ← 6 位验证码输入框                     │
│    └─ Button (登录)           ← isLoginButtonEnabled 控制可用性     │
└─────────────────────────────────────────────────────────────────────┘
                          │ 依赖（构造注入）
┌─────────────────── Domain (domain/auth, domain/local) ─────────────┐
│  IAuthRepository              ← suspend fun sendCode / login        │
│  ITokenManager                ← suspend fun saveToken / getToken    │
│  LoginResult / SendCodeResult ← 领域结果模型（与 DTO 解耦）          │
└─────────────────────────────────────────────────────────────────────┘
                          │ 实现
┌─────────────────── Data (data/auth, data/local, data/remote) ──────┐
│  RetrofitAuthRepository       ← IAuthRepository Retrofit 实现       │
│    └─ AuthApiService          ← Retrofit 接口 (POST /auth/*)        │
│    └─ ApiException            ← 业务错误码异常类                     │
│  TokenManager                 ← ITokenManager DataStore 实现        │
│    └─ DataStore<Preferences>  ← key="jwt_token" 持久化              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 二、核心数据流

```
── 手机号输入 ──────────────────────────────────────────────────────────
用户输入手机号
  → onPhoneNumberChanged(phone)
  → _uiState.update { copy(phoneNumber = ...) }
  → LoginUiState.isSendButtonEnabled 自动重算（9 位 && 首位 '5' && !isSendingCode）

── 发送验证码（T-30002 真实 API 链路）───────────────────────────────────
点击"发送验证码"
  → onSendCode()
  → isSendingCode = true，清空 error
  → authRepository.sendCode("+966${phoneNumber}")      ← Retrofit POST /auth/send-code
       ┌─ onSuccess(SendCodeResult) → countdownSeconds = result.cooldownSeconds
       │                            → startCountdown()（每秒 -1 协程）
       └─ onFailure(ApiException)   → error = mapSendCodeError(e)

── 输入验证码 ──────────────────────────────────────────────────────────
用户输入验证码
  → onVerificationCodeChanged(code)
  → LoginUiState.isLoginButtonEnabled 自动重算（手机号有效 + 6 位 + !isLoading）

── 登录（T-30002 真实 API 链路）────────────────────────────────────────
点击"登录"
  → onLogin()
  → isLoading = true，清空 error
  → authRepository.login("+966${phoneNumber}", verificationCode)  ← POST /auth/login
       ┌─ onSuccess(LoginResult)
       │      → runCatching { tokenManager.saveToken(result.token) }   ← DataStore I/O
       │             ┌─ onSuccess → isLoginSuccess=true, isNewUser=result.isNew
       │             │             → navEvent.emit(NavEvent.NavigateToHall)
       │             └─ onFailure → error = "登录失败，Token 存储异常，请重试"
       └─ onFailure(ApiException)   → error = mapLoginError(e)
```

---

## 三、关键模块说明

### 3.1 LoginUiState
**路径：** `feature/auth/LoginUiState.kt`  
**类型：** 纯 Kotlin `data class`，无任何 Android 框架依赖

| 属性 / 计算属性 | 类型 | 来源 Task | 说明 |
|----------------|------|-----------|------|
| `phoneNumber` | `String` | T-30001 | 用户输入的手机号（不含 +966） |
| `verificationCode` | `String` | T-30001 | 用户输入的 6 位验证码 |
| `countdownSeconds` | `Int` | T-30001 | 倒计时剩余秒数，0 = 未倒计时 |
| `defaultCountryCode` | `String` | T-30001 | 固定 `"+966"`（沙特） |
| `isRtlLayout` | `Boolean` | T-30001 | `true` = 强制 RTL（默认开启） |
| `isLoading` | `Boolean` | **T-30002** | 登录接口请求进行中 |
| `isSendingCode` | `Boolean` | **T-30002** | 发送验证码接口请求进行中 |
| `error` | `String?` | **T-30002** | 接口错误信息（`null` = 无错误） |
| `isLoginSuccess` | `Boolean` | **T-30002** | 登录成功（token 已写入 DataStore） |
| `isNewUser` | `Boolean` | **T-30002** | `true` = 首次注册，可展示新手引导 |
| `isSendButtonEnabled` | `Boolean` | T-30001/T-30002 | `isPhoneNumberValid && countdownSeconds == 0 && !isSendingCode` |
| `isCountingDown` | `Boolean` | T-30001 | `countdownSeconds > 0` |
| `countdownLabel` | `String` | T-30001 | 倒计时中显示 `"Xs"`，否则空串 |
| `isLoginButtonEnabled` | `Boolean` | T-30001/T-30002 | 手机号有效 + 验证码恰好 6 位 + `!isLoading` |

**手机号验证规则（T-30002 修复）：** 去除非数字后恰好 9 位 **且首位为 `'5'`**（沙特格式：+966 后的 `5XXXXXXXX`）。
```kotlin
fun isPhoneNumberValid(phone: String): Boolean {
    val digits = phone.filter { it.isDigit() }
    return digits.length == 9 && digits.startsWith("5")
}
```

---

### 3.2 LoginViewModel
**路径：** `feature/auth/LoginViewModel.kt`  
**继承：** `ViewModel()`  
**构造注入：** `IAuthRepository`（默认 NoOp）、`ITokenManager`（默认 NoOp）

| 方法 / 属性 | 来源 Task | 说明 |
|------------|-----------|------|
| `onPhoneNumberChanged(phone: String)` | T-30001 | 更新手机号，触发状态重算 |
| `onVerificationCodeChanged(code: String)` | T-30001 | 更新验证码，触发状态重算 |
| `onSendCode()` | **T-30002** | 调用 `authRepository.sendCode()`，成功后启动 60s 倒计时协程 |
| `onLogin()` | **T-30002** | 调用 `authRepository.login()`，成功后 `saveToken` + 发射导航事件 |
| `uiState: StateFlow<LoginUiState>` | T-30001 | 只读状态流，UI 通过 `collectAsState()` 订阅 |
| `navEvent: SharedFlow<NavEvent>` | **T-30002** | 单次导航事件（replay=0），UI 在 `LaunchedEffect` 中收集 |

**StateFlow / SharedFlow 暴露方式：**
```kotlin
private val _uiState = MutableStateFlow(LoginUiState())
val uiState: StateFlow<LoginUiState> = _uiState.asStateFlow()

private val _navEvent = MutableSharedFlow<NavEvent>()          // replay = 0
val navEvent: SharedFlow<NavEvent> = _navEvent.asSharedFlow()
```

**错误码映射（T-30002）：**

| ApiException.code | 来源接口 | 用户可见文案 |
|-------------------|---------|------------|
| `40103` | login | `"验证码错误"` |
| `40104` | login | `"验证码已过期"` |
| `40105` | login | `"验证码尝试次数超限"` |
| `40001` | sendCode | `"手机号格式无效"` |
| `42901` | sendCode | `"发送过于频繁，请稍后再试"` |
| `42902` | sendCode | `"今日发送次数已超限"` |
| 其他 | 任意 | `"网络异常，请稍后重试"` |

**Factory（生产环境 DI）：**
```kotlin
LoginViewModel.Factory(authRepository = RetrofitAuthRepository(apiService),
                       tokenManager   = TokenManager(context.authDataStore))
```

---

### 3.3 LoginScreen / LoginScreenContent
**路径：** `feature/auth/LoginScreen.kt`

- `LoginScreen`：有状态入口，通过 `viewModel()` 工厂获取 ViewModel，`collectAsState()` 订阅 StateFlow。
- `LoginScreenContent`：纯 Stateless Composable，所有事件通过 lambda 回调向上传递，便于 Preview 与测试。

**RTL 实现方式：**
```kotlin
val layoutDirection = if (uiState.isRtlLayout) LayoutDirection.Rtl else LayoutDirection.Ltr
CompositionLocalProvider(LocalLayoutDirection provides layoutDirection) {
    // 所有子组件自动遵循 RTL 排列
}
```
不修改 `AndroidManifest`，支持运行时按需切换，无需重启 Activity。

---

### 3.4 PhoneInput 组件
**路径：** `feature/auth/components/PhoneInput.kt`

```
Row
 ├─ Surface（国家码标签）   → 显示 "+966"，固定不可编辑
 ├─ Spacer(8.dp)
 └─ OutlinedTextField       → 仅数字键盘，filter.take(9) 限制 9 位
      placeholder: "5XXXXXXXX"
      label: "رقم الهاتف"（阿拉伯语"手机号"）
```

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `phoneNumber` | `String` | — | 受控输入值 |
| `onPhoneNumberChanged` | `(String) -> Unit` | — | 过滤后的纯数字（≤9位）回调 |
| `countryCode` | `String` | `"+966"` | 国家码前缀 |
| `enabled` | `Boolean` | `true` | 是否可交互 |

---

### 3.5 CodeInput 组件
**路径：** `feature/auth/components/CodeInput.kt`

- 数字密码键盘（`KeyboardType.NumberPassword`）
- `filter.take(6)` 强制最多 6 位
- label：`"رمز التحقق"`（阿拉伯语"验证码"）

---

### 3.6 CountdownButton 组件
**路径：** `feature/auth/components/CountdownButton.kt`

| 状态 | 按钮文案 | 按钮可用性 |
|------|---------|-----------|
| 未倒计时（可发送） | `إرسال رمز التحقق`（发送验证码） | ✅ 可点击 |
| 倒计时中 | `Xs`（剩余秒数） | ❌ 禁用 |

---

### 3.7 IAuthRepository（Domain 层接口）
**路径：** `domain/auth/IAuthRepository.kt`

```kotlin
interface IAuthRepository {
    /** POST /auth/send-code — 发送短信验证码 */
    suspend fun sendCode(phone: String): Result<SendCodeResult>

    /** POST /auth/login — 手机号 + 验证码一步登录（不存在时自动注册） */
    suspend fun login(phone: String, code: String): Result<LoginResult>
}
```

**领域结果模型（`domain/auth/AuthDomainModels.kt`）：**

| 类 | 字段 | 说明 |
|----|------|------|
| `LoginResult` | `token: String` | JWT 字符串，须持久化到 DataStore |
| | `userId: String` | Server 分配的用户 UUID |
| | `isNew: Boolean` | 首次注册时为 true，可用于新手引导 |
| `SendCodeResult` | `cooldownSeconds: Int` | 服务端冷却时长（协议约定 60s） |

---

### 3.8 RetrofitAuthRepository（Data 层实现）
**路径：** `data/auth/RetrofitAuthRepository.kt`

```
RetrofitAuthRepository
  └─ sendCode(phone) → apiService.sendCode(SendCodeRequest(phone))
  └─ login(phone, code) → apiService.login(LoginRequest(phone, code))
       ↓ parseBody<T>(response)
       ├─ HTTP 2xx + code==0 → 返回 data（映射为领域对象）
       ├─ HTTP 2xx + code≠0  → throw ApiException(code, message)
       └─ HTTP 4xx/5xx       → 解析 errorBody JSON → throw ApiException
```

**`ApiException`（`data/auth/ApiException.kt`）：**
```kotlin
class ApiException(val code: Int, message: String) : Exception(message)
// code 对应 protocol.md §1.4 错误码，如 40103 = 验证码错误
```

> ⚠️ **已知代码味道（低优先级）**：`parseBody` 中 `runCatching { throw ApiException(...) }` 模式反直觉，建议后续 cleanup sprint 改为标准 `try-catch`（Review M02，不阻塞功能）。

---

### 3.9 TokenManager（DataStore JWT 持久化）
**路径：** `data/local/TokenManager.kt`  
**接口：** `domain/local/ITokenManager.kt`

```kotlin
interface ITokenManager {
    suspend fun saveToken(token: String)   // DataStore edit, key = "jwt_token"
    suspend fun getToken(): String?        // DataStore firstOrNull()
    suspend fun clearToken()               // DataStore remove key
}
```

**线程安全：** `DataStore` 保证并发写安全，`saveToken` 在协程中调用无竞争风险。  
**初始化（Application 级别）：**
```kotlin
val Context.authDataStore by preferencesDataStore(name = "auth")
// 注入：TokenManager(context.authDataStore)
```

---

### 3.10 NavEvent（单次导航事件）
**路径：** `feature/auth/NavEvent.kt`

```kotlin
sealed class NavEvent {
    /** 登录成功 → 跳转大厅（在 saveToken 成功后发射） */
    object NavigateToHall : NavEvent()
}
```

**消费方式（LoginScreen.kt）：**
```kotlin
LaunchedEffect(Unit) {
    viewModel.navEvent.collect { event ->
        when (event) {
            NavEvent.NavigateToHall -> navController.navigate("hall") { popUpTo("login") }
        }
    }
}
```

**关键约束：** `NavigateToHall` 仅在 `saveToken` **成功**后发射；`saveToken` 抛异常时仅更新 `error`，不导航，避免用户在 token 未保存状态下进入大厅。

---

## 四、RTL 布局支持

沙特阿拉伯市场需要阿拉伯语从右到左布局，实现方案：

| 方案 | 选用 | 说明 |
|------|------|------|
| `CompositionLocalProvider(LocalLayoutDirection)` | ✅ | 局部覆盖，无需修改 Manifest |
| `android:supportsRtl="true"` + 系统语言 | ❌ | 全局生效，无法按页面切换 |

`LoginUiState.isRtlLayout` 默认为 `true`（沙特市场首选），后续可扩展为从用户设置或系统语言动态读取。

---

## 五、依赖项

| 依赖 | 版本 | 来源 Task | 用途 |
|------|------|-----------|------|
| `androidx.compose.bom` | Compose BOM | T-30001 | Compose 版本统一管理 |
| `androidx.lifecycle:lifecycle-viewmodel-compose` | BOM 对齐 | T-30001 | `viewModel()` 工厂 |
| `org.jetbrains.kotlinx:kotlinx-coroutines-android` | — | T-30001 | `viewModelScope` 协程 |
| `androidx.compose.material3` | BOM 对齐 | T-30001 | Material3 主题与组件 |
| `com.squareup.retrofit2:retrofit` | **2.11.0** | **T-30002** | HTTP 客户端，调用 `/auth/*` 接口 |
| `com.squareup.retrofit2:converter-gson` | **2.11.0** | **T-30002** | JSON 序列化（ApiResponse / DTO） |
| `androidx.datastore:datastore-preferences` | **1.1.1** | **T-30002** | JWT Token 本地持久化 |

---

## 六、已知问题 / 后续迭代

### T-30001 遗留项（T-30002 已解决）

| 严重级别 | 问题 | 状态 |
|---------|------|------|
| **HIGH** | `onSendCode()` 协程路径未被测试 | ✅ T-30002 已补充 `runTest { vm.onSendCode() }` 等 19 个用例 |
| **MEDIUM** | `isPhoneNumberValid` 未校验首位必须为 `'5'` | ✅ T-30002 已修复：`digits.startsWith("5")` |
| **HIGH** | `tokenManager.saveToken` 未包裹错误处理，UI 可永久 Loading | ✅ T-30002 已修复：`runCatching { saveToken }` 双路径处理 |

### 当前遗留项

| 严重级别 | 问题 | 建议修复 Task |
|---------|------|-------------|
| **MEDIUM** | 使用 `collectAsState()` 而非 `collectAsStateWithLifecycle()`，后台时仍收集 Flow | T-30003 或单独 cleanup |
| **MEDIUM** | `RetrofitAuthRepository.parseBody` 中 `runCatching { throw }` 代码味道 | cleanup sprint |
| **LOW** | `LoginViewModel` 中 `"+966"` 前缀硬编码（两处），建议读 `_uiState.value.defaultCountryCode` | 后续迭代 |
| **LOW** | UI 文字（阿拉伯语）硬编码，未使用 `strings.xml` | 后续 i18n 迭代 |
| **LOW** | `IAuthService.kt` / `DebugAuthService.kt` 与登录流程无关联，建议清理或注明用途 | cleanup sprint |

---

## 七、T-30003 JWT 拦截器

**关联 Task:** [T-30003](../../tds/android/T-30003.md) — JWT 拦截器  
**状态:** ✅ Done  
**Last Updated:** 2026-05

---

### 7.1 新增文件列表（`core/network/`）

| 文件路径（`com.voice.room.android.` 下） | 类型 | 职责 |
|----------------------------------------|------|------|
| `core/network/AuthInterceptor.kt` | OkHttp Interceptor | 读取 token → 注入 `Authorization: Bearer` header；401 响应时触发未授权处理 |
| `core/network/UnauthorizedHandler.kt` | Interface | 定义 `onUnauthorized()` + `resetUnauthorized()` 两个行为契约 |
| `core/network/DefaultUnauthorizedHandler.kt` | Impl | `AtomicBoolean.compareAndSet` 保证并发 401 只处理一次；`resetUnauthorized()` 重置标志位 |

`AppHttpClientFactory.kt` 新增 `authInterceptor` 参数，将拦截器接入 OkHttp 调用链。  
`LoginViewModel.kt` 在 `saveToken` 成功后调用 `unauthorizedHandler.resetUnauthorized()`。

---

### 7.2 AuthInterceptor 工作流程

```
── 请求阶段 ──────────────────────────────────────────────────────────
Request 进入拦截器
  → tokenManager.getToken()
      ├─ token != null  → 克隆 Request，添加 "Authorization: Bearer <token>"
      └─ token == null  → 原样透传（未登录请求，如 /auth/login）
  → chain.proceed(request)

── 响应阶段 ──────────────────────────────────────────────────────────
Response 返回
  → response.code == 401?
      ├─ 否 → 直接返回 Response
      └─ 是 → unauthorizedHandler.onUnauthorized()
                  → AtomicBoolean.compareAndSet(false, true)
                      ├─ true  → 首次触发：执行登出 / 跳转登录页逻辑
                      └─ false → 并发重复 401，静默忽略
             → 返回 Response（由调用方感知 401）
```

---

### 7.3 AtomicBoolean 并发保护

高并发场景下（如同时发起多个 API 请求），多个请求可能几乎同时收到 401 响应。
`DefaultUnauthorizedHandler` 通过 `AtomicBoolean` 的 CAS（Compare-And-Set）操作保证：

```kotlin
private val isHandling = AtomicBoolean(false)

override fun onUnauthorized() {
    if (isHandling.compareAndSet(false, true)) {
        // 仅第一个到达的 401 会执行此块，其余被原子性地跳过
        // 执行：清除本地 Token、发送登出广播、跳转登录页
    }
}

override fun resetUnauthorized() {
    isHandling.set(false)   // 登录成功后恢复，允许下次 401 重新触发
}
```

**保证：** 无论多少个线程并发调用 `onUnauthorized()`，用户只会看到一次"登录已过期"提示，不会出现重复弹窗或多次跳转登录页。

---

### 7.4 resetUnauthorized() 调用时机

`resetUnauthorized()` 必须在**登录成功、token 已成功写入 DataStore 之后**调用，防止在新 token 还未持久化的窗口期内，下一次 401 被静默忽略。

```
LoginViewModel.onLogin()
  → authRepository.login(...)
      └─ onSuccess(LoginResult)
           → tokenManager.saveToken(result.token)   ← DataStore I/O 成功
                └─ onSuccess
                     → unauthorizedHandler.resetUnauthorized()  ← ✅ 此处调用
                     → navEvent.emit(NavEvent.NavigateToHall)
```

**错误路径：** `saveToken` 抛异常时**不**调用 `resetUnauthorized()`，保持拦截器处于"已触发"状态，避免在 token 未存储时放行后续请求。

---

### 7.5 UnauthorizedHandler 接口解耦的价值

`UnauthorizedHandler` 定义在 `core/network` 层，是一个纯 Kotlin 接口，**零 Android UI 依赖**：

| 特性 | 说明 |
|------|------|
| 无 `Context` / `Activity` 依赖 | `AuthInterceptor` 本身可在纯 JVM 环境构造和测试 |
| 可 Mock / Fake | 单元测试中注入 `FakeUnauthorizedHandler`，无需 Robolectric |
| 生产实现可替换 | `DefaultUnauthorizedHandler` 通过 DI 注入，不影响 `AuthInterceptor` 源码 |
| 测试覆盖简洁 | `onUnauthorized()` 调用次数断言 + `AtomicBoolean` 状态断言，纯 JVM 即可运行 |

---

## 八、相关文档

- [Android 架构总索引](./index.md)
- [业务骨架与测试现状](./features.md)
- [TDS T-30001 登录页 UI](../../tds/android/T-30001.md)
- [TDS T-30002 登录 ViewModel](../../tds/android/T-30002.md)
- [TDS T-30003 JWT 拦截器](../../tds/android/T-30003.md)

## 九、T-30002 新增文件速查

| 文件路径（`com.voice.room.android.` 下） | 类型 | 说明 |
|----------------------------------------|------|------|
| `domain/auth/IAuthRepository.kt` | Interface | 认证仓库 Domain 契约 |
| `domain/auth/AuthDomainModels.kt` | Data class | `LoginResult` / `SendCodeResult` |
| `domain/local/ITokenManager.kt` | Interface | Token 存储 Domain 契约 |
| `data/remote/api/AuthApiService.kt` | Retrofit | `POST /auth/send-code` + `POST /auth/login` |
| `data/remote/model/ApiResponse.kt` | DTO | 统一响应包装体 `ApiResponse<T>` |
| `data/remote/model/LoginModels.kt` | DTO | `LoginRequest` / `LoginResponseData` |
| `data/remote/model/SendCodeModels.kt` | DTO | `SendCodeRequest` / `SendCodeResponseData` |
| `data/auth/ApiException.kt` | Exception | 业务错误码异常（`code: Int`） |
| `data/auth/RetrofitAuthRepository.kt` | Impl | `IAuthRepository` Retrofit 实现 |
| `data/local/TokenManager.kt` | Impl | `ITokenManager` DataStore 实现 |
| `feature/auth/NavEvent.kt` | Sealed | `NavigateToHall` 导航事件 |
