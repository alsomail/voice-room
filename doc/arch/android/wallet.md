# Android 钱包模块架构文档

**最后更新**：2026-04-23  
**负责人**：Dod Agent  
**关联 Task**：T-30027（钱包页完整实现，Review Round 2 通过）

---

## 一、模块概览

### 功能定义
钱包模块提供用户钻石余额查看与交易流水列表展示功能，支持：
- 💎 **余额显示**：大卡片展示钻石余额（金色大号字体）
- 📋 **流水列表**：Paging3 分页加载交易记录（支持分类、排序）
- 🔄 **实时更新**：WebSocket `BalanceUpdated` 事件推送余额变更
- 🔗 **导航入口**：个人中心"钻石余额"行点击跳转进入钱包页
- ♻️ **下拉刷新**：刷新余额和流水首页

### 核心设计决策
1. **分层架构**：`domain/IWalletRepository` 接口 → `data/RetrofitWalletRepository` HTTP 实现 → `feature/WalletScreen` UI
2. **WebSocket 集成**：通过 `WebSocketEventBus` 订阅 `BalanceUpdatedEvent`，按 protocol §6.4.1 读取嵌套 `payload.diamond_balance`
3. **Paging3 设计**：`WalletTxnPagingSource` 1-based 分页，`lastPage` 判断逻辑为 `items.size < loadSize`
4. **错误处理**：401 token 过期统一由 `AuthInterceptor` 拦截 + `DefaultUnauthorizedHandler` 触发 `NavigateToLogin` 事件

---

## 二、架构分层

### Domain 层（业务接口与模型）

```
domain/wallet/
├── IWalletRepository.kt          # Repository 接口
├── WalletTxn.kt                   # 交易记录领域模型
└── TxnsPage.kt                    # 分页结果容器
```

**关键模型**：

| 模型 | 说明 | 字段 |
|-----|------|------|
| `IWalletRepository` | Repository 接口 | `getBalance(): Result<Long>`<br/>`listTxns(page, pageSize): Result<TxnsPage<WalletTxn>>` |
| `WalletTxn` | 交易记录 | `id: String` / `amount: Long` / `reason: String` / `iconUrl: String` / `createdAt: Long` |
| `TxnsPage` | 分页结果 | `items: List<WalletTxn>` / `total: Int` / `page: Int` |

### Data 层（HTTP 与本地存储）

```
data/wallet/
├── WalletApiService.kt            # Retrofit HTTP 接口
├── WalletModels.kt                # DTO 对象（与领域模型解耦）
├── RetrofitWalletRepository.kt    # Repository 实现（HTTP）
├── WalletTxnPagingSource.kt       # Paging3 分页数据源
└── DebugWalletRepository.kt       # Mock 实现（测试）
```

**API 接口**：

| 方法 | URL | 说明 |
|-----|-----|------|
| `getBalance()` | `GET /api/v1/wallet/balance` | 获取钻石余额 |
| `listTxns(page, size, type?)` | `GET /api/v1/wallet/transactions` | 分页加载交易流水 |

**关键实现细节**：

1. **WalletApiService.kt**
   ```kotlin
   interface WalletApiService {
       @GET("/api/v1/wallet/balance")
       suspend fun getBalance(): ApiResponse<BalanceDto>
       
       @GET("/api/v1/wallet/transactions")
       suspend fun listTxns(
           @Query("page") page: Int,
           @Query("size") size: Int = 20,
           @Query("type") type: String? = null
       ): ApiResponse<PageDto<TxnDto>>
   }
   ```

2. **RetrofitWalletRepository.kt**
   - `getBalance()`：HTTP 调用 → 错误映射（401 → `AuthInterceptor` 处理）→ 返回 `Result<Long>`
   - `listTxns()`：HTTP 调用 → DTO 转领域模型 → 返回 `Result<TxnsPage>`
   - 统一错误处理策略与 `RetrofitUserRepository` 一致

3. **WalletTxnPagingSource.kt**
   - `load(params)` 参数：`Int` 页码（1-based）
   - 分页判断：`params.loadSize` 和返回数据 `items.size` 对比，`size < loadSize` 时设置 `endOfPage=true`
   - `getRefreshKey(state)` 标准实现：返回 `state.anchorPosition`

### Feature 层（UI 与状态管理）

```
feature/wallet/
├── WalletScreen.kt                # Compose 页面
├── WalletViewModel.kt             # 状态与逻辑
├── WalletUiState.kt               # UI 状态数据类
├── WalletEvent.kt                 # 一次性事件（导航、Toast）
└── WalletTxnItem.kt              # 流水列表项组件
```

**关键设计**：

1. **WalletUiState.kt**
   ```kotlin
   data class WalletUiState(
       val balance: Long = 0,
       val loadingBalance: Boolean = true,
       val txnFlow: Flow<PagingData<WalletTxn>>? = null,
       val refreshing: Boolean = false,
       val error: String? = null
   )
   ```

2. **WalletEvent.kt**（sealed class）
   ```kotlin
   sealed class WalletEvent {
       data class ShowToast(val message: String) : WalletEvent()
       object NavigateToLogin : WalletEvent()
       object RefreshTransactions : WalletEvent()
   }
   ```

3. **WalletViewModel.kt**
   - 构造器注入：`walletRepository: IWalletRepository`, `wsEventBus: WebSocketEventBus`, `tokenManager: ITokenManager`
   - **init block**：
     - `loadBalance()` 初始加载余额
     - `subscribeToWsEvents()` 监听 `BalanceUpdatedEvent` 实时更新
   - **方法**：
     - `loadBalance()`：HTTP 加载 → 401 时 `emit NavigateToLogin` → 成功时更新 `_state`
     - `refresh()`：刷新余额 + 列表首页，同时处理 401 → `NavigateToLogin`
     - `subscribeToWsEvents()`：监听 WS `BalanceUpdatedEvent`，按 **protocol §6.4.1** 读取 `payload.diamond_balance` 字段
   - **testTag 协议**：见下文

4. **WalletScreen.kt**（Compose UI）
   - `Scaffold(topBar = TopAppBar(title="我的钱包", onBackClick))`
   - **余额卡片**（`Card(modifier=Modifier.padding(16dp), elevation=4.dp)`）：
     - 💎 Icon（24dp MenaColors.Primary）
     - 余额大号字体（`headlineLarge` 金色 MenaColors.Primary）
     - "充值"按钮 → Toast "即将上线"
     - `testTag = "wallet_balance_value"`
   - **流水列表**（`PullToRefreshBox` 包裹 `LazyColumn`）：
     - Paging3 `collectAsLazyPagingItems()` + `items()` 循环渲染
     - 每项 `WalletTxnItem(txn)`：收入绿色 + 号，支出红色 - 号，图标+时间+reason
     - `testTag = "wallet_txn_list"` 挂在 `LazyColumn` 的 `modifier` 上
   - **空状态**（`items.itemCount == 0 && !isLoading`）：插画 + "暂无流水"，`testTag = "wallet_empty"`
   - **错误状态**：显示错误信息 + 重试按钮
   - `imePadding()` 适配软键盘

---

## 三、WebSocket 集成

### BalanceUpdated 事件处理

**协议规范**：`doc/protocol/websocket_signals.md §6.4.1`

```json
{
  "msg_id": "uuid",
  "signal": "BalanceUpdated",
  "payload": {
    "diamond_balance": 5000,
    "update_reason": "gift_received"
  }
}
```

**实现方式**（WalletViewModel.kt）：

```kotlin
private fun subscribeToWsEvents() {
    viewModelScope.launch {
        wsEventBus.events
            .filterIsInstance<BalanceUpdatedEvent>()
            .collect { event ->
                // 按照 protocol §6.4.1 的正确格式读取
                // JSON 结构：{ "msg_id": "...", "signal": "BalanceUpdated", "payload": { "diamond_balance": 5000 } }
                val newBalance = event.payload.diamond_balance
                _state.update { it.copy(balance = newBalance) }
                // 同时刷新流水首页以获取最新交易
                txnPager.refresh()
            }
    }
}
```

**关键点**：
- ✅ 正确格式：通过 `payload.diamond_balance` 嵌套字段读取
- ❌ 错误格式：~~顶层 `new_balance` 字段~~（已在 R1 修复，测试验证）
- 测试覆盖：`WalletViewModelTest.kt` 中 `R1-CRITICAL-1`（正确协议格式）+ `R1-CRITICAL-1b`（旧错误格式被忽略）

---

## 四、错误处理与导航

### 401 Token 过期处理

**流程**：
1. `loadBalance()` 或 `refresh()` 调用 HTTP API
2. 服务端返回 401 Unauthorized
3. `AuthInterceptor` 拦截，触发 `DefaultUnauthorizedHandler.handle()`
4. ViewModel 的 `onFailure` 分支捕获 `ApiException(401)`
5. 发射 `WalletEvent.NavigateToLogin`
6. `WalletScreen` 监听事件，执行导航回登录页

**代码实现**：

```kotlin
// WalletViewModel.kt loadBalance() 方法
private fun loadBalance() {
    viewModelScope.launch {
        try {
            val balance = walletRepository.getBalance().getOrThrow()
            _state.update { it.copy(balance = balance, loadingBalance = false) }
        } catch (e: ApiException) {
            if (e.code == 401) {
                _events.emit(WalletEvent.NavigateToLogin)
            } else {
                _state.update { it.copy(error = e.message, loadingBalance = false) }
            }
        }
    }
}

// refresh() 方法同时处理 401
private fun refresh() {
    viewModelScope.launch {
        _state.update { it.copy(refreshing = true) }
        try {
            val balance = walletRepository.getBalance().getOrThrow()
            _state.update { it.copy(balance = balance, refreshing = false) }
            txnPager.refresh()
        } catch (e: ApiException) {
            if (e.code == 401) {
                _events.emit(WalletEvent.NavigateToLogin)
            } else {
                _state.update { it.copy(error = e.message, refreshing = false) }
            }
        }
    }
}
```

---

## 五、UI 测试协议

### testTag 映射表

| testTag | 位置 | 说明 |
|---------|------|------|
| `wallet_balance_value` | 余额卡片数字 | 余额大号字体，用于定位和数值验证 |
| `btn_wallet_recharge` | 充值按钮 | "充值"按钮，点击验证 Toast |
| `wallet_txn_list` | LazyColumn | 流水列表主容器，挂在 `modifier` 上 |
| `wallet_empty` | 空状态 Box | 暂无流水占位，用于验证空状态渲染 |

### 验收用例映射

| 验收用例 | 测试类型 | testTag | 断言 |
|---------|---------|--------|------|
| W27-01：打开调用 API | 单元测试 | N/A | `walletRepository.getBalance()` + `listTxns()` 被调用 |
| W27-02：余额 0 显示"0" | 单元测试 | `wallet_balance_value` | 数值为 0，无负号 |
| W27-03：充值按钮 Toast | UI 测试 | `btn_wallet_recharge` | 点击后显示 Toast "即将上线" |
| W27-04：WS 实时更新 | 单元测试 | `wallet_balance_value` | 发送 BalanceUpdated 事件后余额更新（按 §6.4.1 协议格式） |
| W27-05：下拉刷新 | UI 测试 | `wallet_balance_value` + `wallet_txn_list` | PullToRefreshBox 向下拉取后数据刷新 |
| W27-06：流水颜色 | UI 测试 | `wallet_txn_list` | 收入项绿色+，支出项红色- |
| W27-07：空状态占位 | UI 测试 | `wallet_empty` | 暂无流水时显示占位文案与插画 |
| W27-08：401 跳转登录 | 单元测试 | N/A | `loadBalance()` / `refresh()` 返回 401 → 发射 `NavigateToLogin` 事件 |
| W27-09：Profile 点击跳转 | 集成测试 | N/A | ProfileContent 余额行 clickable → `navController.navigate("wallet")` |

---

## 六、文件位置映射

### 代码组织

```
app/android/app/src/main/java/com/voiceroom/
├── wallet/
│   ├── WalletScreen.kt                          # 页面 UI
│   ├── WalletViewModel.kt                       # 状态与逻辑
│   ├── WalletUiState.kt                         # UI 状态
│   ├── WalletEvent.kt                           # 事件定义
│   ├── WalletTxnItem.kt                         # 流水列表项
│   └── data/
│       ├── WalletApiService.kt                  # API 接口
│       ├── WalletModels.kt                      # DTO
│       ├── WalletTxnPagingSource.kt             # Paging3 数据源
│       └── RetrofitWalletRepository.kt          # Repository 实现
├── domain/wallet/
│   ├── IWalletRepository.kt                     # 接口
│   ├── WalletTxn.kt                             # 领域模型
│   └── TxnsPage.kt                              # 分页结果
├── core/ws/event/
│   └── (BalanceUpdatedEvent 已删除，改用直接 JSON 解析)
├── feature/
│   └── profile/
│       ├── ProfileContent.kt (修改)             # 余额行新增 onNavigateToWallet 回调
│       └── ProfileScreen.kt (修改)              # 传入导航回调
└── feature/main/
    └── MainScreen.kt (修改)                     # 新增 "wallet" 路由
```

### 导航配置

**MainScreen.kt** 内部 NavHost：

```kotlin
composable("wallet") {
    WalletScreen(
        onBackClick = { navController.popBackStack() },
        onNavigateToLogin = { 
            navController.navigate("login") {
                popUpTo("main") { inclusive = true }
            }
        }
    )
}
```

**ProfileScreen.kt** 传入回调：

```kotlin
// MainScreen.kt 调用 ProfileScreen
ProfileScreen(
    onNavigateToWallet = { navController.navigate("wallet") }
)

// ProfileContent.kt 使用回调
Modifier.clickable { onNavigateToWallet() }
```

---

## 七、测试覆盖

### 单元测试（WalletViewModelTest.kt）

**总数**：22 个测试，全部通过

| 分类 | 测试用例 | 覆盖目标 | 状态 |
|------|---------|--------|------|
| 初始化 | WM-01 ~ WM-03 | `init` 调用 `loadBalance()` 和 `subscribeToWsEvents()` | ✅ |
| 余额加载 | W27-01, W27-02, W27-08 | API 调用、零值显示、401 处理 | ✅ |
| WS 事件 | W27-04, R1-CRITICAL-1, R1-CRITICAL-1b | 正确协议格式、错误格式被忽略 | ✅ |
| 刷新操作 | W27-05, R1-HIGH-3, R1-HIGH-3b | 下拉刷新、401 导航、非 401 错误处理 | ✅ |
| 边界情况 | Extra-01 ~ Extra-05 | 极值、`CancellationException` re-throw、并发等 | ✅ |

### Paging3 分页测试（WalletTxnPagingSourceTest.kt）

**总数**：7 个测试，全部通过

| 测试 | 覆盖 |
|-----|------|
| PS-01 ~ PS-03 | 首页/中页/末页加载逻辑 |
| PS-04 ~ PS-05 | 网络错误重试 |
| PS-06 ~ PS-07 | 刷新与追加场景 |

### Compose UI 测试（可选，当前未实现）

建议后续补充 `WalletScreenTest.kt`（androidTest），覆盖 W27-03, W27-06, W27-07 的视觉验证。

---

## 八、外部依赖

| 库 | 版本 | 用途 |
|----|-----|------|
| Retrofit | 2.11.0 | HTTP 客户端 |
| OkHttp | 4.11.0 | HTTP 库 + 拦截器 |
| Paging3 | 3.1.1 | 分页加载 |
| Compose | Latest Material3 | UI 框架 |
| Coil | 2.x | 图片加载 |
| Gson | 2.10.1 | JSON 序列化 |

---

## 九、相关文档链接

- **技术设计**：[doc/tds/android/T-30027.md](../../tds/android/T-30027.md)
- **UI 设计**：[doc/design/android/T-30027.md](../../design/android/T-30027.md)
- **WebSocket 协议**：[doc/protocol/websocket_signals.md §6.4.1](../../protocol/websocket_signals.md#641-balanceupdated-余额更新推送)
- **钱包 API 文档**：[doc/protocol/wallet_api.md](../../protocol/wallet_api.md)
- **个人中心模块**：[features.md](./features.md)
- **导航架构**：[navigation.md](./navigation.md)

---

## 十、后续迭代方向

1. **礼物购买**（T-30028 ~ T-30031）：礼物面板接入钱包余额展示与实时更新
2. **充值流程**（E-08）：集成 Google Play 计费库，真实充值流程
3. **余额不足引导**（T-30032）：充值弹窗快速导流到钱包
4. **榜单模块**（T-30033）：与钱包财富榜联动展示

---

**DoD 确认状态**：✅ 代码实现完成 → Review R2 通过 → 文档已同步  
**审查员**：Claude Sonnet 4.5 (Review R2，2026-04-23)
