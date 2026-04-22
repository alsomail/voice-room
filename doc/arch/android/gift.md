# Android 礼物模块架构文档

**最后更新**：2026-04-25  
**负责人**：Dod Agent  
**关联 Task**：T-30028（礼物面板 BottomSheet 完整实现），T-30029（接收者选择器），T-30030（SendGift 客户端+幂等）

---

## 一、模块概览

### 功能定义
礼物模块提供虚拟礼物面板展示、接收者选择与发送功能，支持：
- 🎁 **礼物列表**：HTTP API 获取礼物列表（60s Mutex 防竞态缓存），4列网格展示
- 🔖 **分类 Tab**：热门/全部礼物分类切换（按后端 tier 字段筛选）
- 🔢 **数量选择**：6 档位吉祥数选择器（1/10/66/520/786/1314）
- 💎 **余额实时更新**：WebSocket `BalanceUpdated` 事件推送余额变更
- 🚨 **错误重试**：网络失败时展示重试按钮，绑定 `giftViewModel.retryLoad()`
- 🎯 **接收者选择器**：LazyRow 横向滚动麦位头像条，支持选中高亮（T-30029 已完成）

### 核心设计决策
1. **分层架构**：`domain/IGiftRepository` 接口 → `data/RetrofitGiftRepository` HTTP 实现 → `feature/GiftPanelViewModel` 状态管理 → `feature/GiftPanelBottomSheet` UI
2. **Mutex 缓存**：`RetrofitGiftRepository` 采用 Kotlin Coroutines `Mutex.withLock` 保护整个"读缓存→判断过期→发请求→写缓存"复合操作，消除 TOCTOU 竞态（R1-MEDIUM 修复）
3. **多语言支持**：`listGifts(locale)` 通过 `Accept-Language` Header 按 locale 参数请求多语言礼物名称
4. **错误恢复**：UI 层展示错误状态 + 重试按钮，调用方传入 `onRetry = { giftViewModel.retryLoad() }` 绑定重试逻辑（R1-HIGH 修复）
5. **接收者选择器**：GiftPanelBottomSheet 顶部嵌入 `RecipientSelector` 组件，支持 LazyRow 横向滚动、主麦默认选中、选中项金色边框高亮、空麦时按钮禁用（T-30029 完成）

---

## 二、架构分层

### Domain 层（业务接口与模型）

```
domain/gift/
├── IGiftRepository.kt          # Repository 接口
├── GiftVO.kt                   # 礼物值对象
└── MicUserVO.kt                # 麦位用户值对象（接收者槽）
```

**关键模型**：

| 模型 | 说明 | 字段 |
|-----|------|------|
| `IGiftRepository` | Repository 接口 | `listGifts(locale: String): Result<List<GiftVO>>` |
| `GiftVO` | 礼物值对象 | `id: String` / `code: String` / `name: String` / `iconUrl: String` / `price: Long` / `sortOrder: Int` / `tier: Int` |
| `MicUserVO` | 麦位用户值对象 | `micIndex: Int` / `userId: String` / `nickname: String` / `avatarUrl: String` |

### Data 层（HTTP 与缓存）

```
data/gift/
├── GiftApiService.kt           # Retrofit HTTP 接口
├── GiftModels.kt               # DTO 对象（GiftDto、GiftResponse）
├── RetrofitGiftRepository.kt   # Repository 实现（HTTP + Mutex 缓存）
└── DebugGiftRepository.kt      # Mock 实现（测试）
```

**API 接口**：

| 方法 | URL | 说明 | Header |
|-----|-----|------|--------|
| `listGifts()` | `GET /api/v1/gifts/list` | 获取礼物列表 | `Accept-Language: {locale}` |

**缓存机制**：

```kotlin
private val cacheMutex = Mutex()
private var cachedGifts: List<GiftVO>? = null
private var cacheTimestamp: Long = 0L
private val cacheDurationMs: Long = 60_000L  // 60s，可通过构造参数注入（测试友好）

override suspend fun listGifts(locale: String): Result<List<GiftVO>> = runCatching {
    cacheMutex.withLock {
        val now = System.currentTimeMillis()
        val cached = cachedGifts
        if (cached != null && (now - cacheTimestamp) < cacheDurationMs) {
            return@runCatching cached  // 缓存命中，直接返回
        }
        // 缓存过期或首次调用，发起 HTTP 请求
        val gifts = parseBody(apiService.listGifts(acceptLanguage = locale)).map { it.toDomain() }
        cachedGifts = gifts
        cacheTimestamp = System.currentTimeMillis()
        gifts
    }
}
```

**为什么用 Mutex？**
- `@Volatile` 仅保证单次读写可见性，无法保护复合操作
- 两个协程并发调用 `listGifts()` 时，都可能通过过期检查，各自发起 HTTP 请求（**TOCTOU 竞态**）
- `Mutex.withLock` 保证整个 check-then-act 块的原子性，只有一个协程能进入 `withLock`，其他协程阻塞等待

### Presentation 层（状态管理与 UI）

```
feature/gift/
├── GiftPanelViewModel.kt       # 状态管理 ViewModel
├── GiftPanelUiState.kt         # UI 状态数据类
├── GiftPanelEvent.kt           # 一次性事件定义
├── GiftPanelBottomSheet.kt     # ModalBottomSheet UI
└── components/
    ├── GiftCard.kt             # 礼物卡片组件
    ├── CountSelector.kt        # 数量选择器
    ├── RecipientSelector.kt    # 接收者选择器（T-30029 ✅）
    └── BalanceBar.kt           # 顶部余额条
```

**UiState 设计**：

```kotlin
data class GiftPanelUiState(
    val gifts: List<GiftVO> = emptyList(),
    val loading: Boolean = true,
    val error: String? = null,
    val selectedGiftId: String? = null,
    val selectedCount: Int = 1,
    val balance: Long = 0,
    val recipients: List<MicUserVO> = emptyList(),      // 来自房间状态
    val selectedRecipientId: String? = null,            // 默认主麦
    val activeTab: GiftTab = GiftTab.Hot,
) {
    val selectedGift get() = gifts.firstOrNull { it.id == selectedGiftId }
    val totalPrice get() = (selectedGift?.price ?: 0L) * selectedCount
    val canSend get() = selectedGift != null && selectedRecipientId != null
                         && balance >= totalPrice && recipients.isNotEmpty()
    val isBalanceInsufficient get() = selectedGift != null && balance < totalPrice
    val displayGifts get() = if (activeTab == GiftTab.Hot) 
        gifts.filter { it.tier in 2..3 } 
    else 
        gifts
}

enum class GiftTab { Hot, All, Backpack /* Phase2 占位 */ }
```

**ViewModel 关键方法**：

| 方法 | 说明 | 依赖 |
|-----|------|------|
| `loadGifts()` | 异步加载礼物列表，更新 UiState | `IGiftRepository.listGifts(locale)` |
| `selectGift(giftId)` | 选中礼物，更新 `selectedGiftId` | 内存状态 |
| `selectCount(count)` | 选择数量档位，更新 `selectedCount` | 内存状态 |
| `selectRecipient(userId)` | 选择接收者，更新 `selectedRecipientId` | 内存状态 |
| `updateRecipients(users)` | 更新房间麦位用户列表 | RoomViewModel 推送 |
| `selectTab(tab)` | 切换分类 Tab（热门/全部） | 内存状态 |
| `dismiss()` | 关闭面板，清除 `selectedGiftId` | 内存状态 |
| `retryLoad()` | 重试加载（缓存重置后重新调用 `loadGifts()`） | `IGiftRepository` |
| `updateBalance(newBalance)` | WS `BalanceUpdated` 触发，更新余额 | WebSocket 事件 |

**Event 设计**：

```kotlin
sealed class GiftPanelEvent {
    data class ShowRechargeHint(val currentBalance: Long, val requiredBalance: Long) : GiftPanelEvent()
    data class ShowToast(val message: String) : GiftPanelEvent()
}
```

---

## 三、UI 组件层级

### GiftPanelBottomSheet（总容器）

```
ModalBottomSheet(height=55% screen)
├── BalanceBar（顶部余额条）
│   ├── 💎 balance 金色大号显示
│   └── "充值" 按钮（占位 Toast"即将上线"）
├── TabRow（分类切换）
│   ├── "热门" Tab
│   └── "全部" Tab
├── LazyVerticalGrid(columns=Fixed(4))（礼物网格）
│   └── GiftCard × N
│       ├── 礼物图标
│       ├── 礼物名称
│       ├── 礼物价格
│       └── 金色边框（选中态）
├── CountSelector（数量选择器）
│   └── Chip Row × 6 档位（1/10/66/520/786/1314）
├── RecipientSelector（接收者选择器 - T-30029 ✅）
│   ├── LazyRow 横向滚动麦位头像条
│   ├── 主麦（slot=0）置首，其他麦位按 micIndex 升序
│   ├── 选中项金色 2dp 光圈边框 + 底部实心金色圆点
│   └── 无人在麦时显示 "当前无人在麦"（居中灰字）
└── Button("送出 {totalPrice}💎")
    ├── enabled = canSend && !sending （T-30030 ✅）
    ├── 发送中显示 CircularProgressIndicator （T-30030 ✅）
    ├── 禁用文案 = "余额不足" 或 "无人在麦"
    └── onClick = { viewModel.sendGift() } （T-30030 接入 ✅）
```

### GiftCard（礼物卡片）

- 可点击，触发 `onSelectGift(giftId)`
- 显示礼物图标（Coil AsyncImage）+ 名称 + 价格
- 选中态：金色边框（`MenaColors.Primary`）

### CountSelector（数量选择器）

- 6 个 Chip：1、10、66、520、786、1314（吉祥数）
- 选中态高亮（`MenaColors.Primary` 背景）
- 点击更新 `selectedCount`，即时刷新 totalPrice 与 canSend

### BalanceBar（余额条）

- 左侧：💎 + balance 金色大号字体（`MenaColors.Primary`，`titleLarge`）
- 右侧："充值" 按钮（金色轮廓，占位 Toast）
- WS `BalanceUpdated` 实时更新 balance 数值（无动画，直接替换）

---

## 四、关键交互流程

### 1. 打开礼物面板
```
RoomScreen.🎁Button.onClick
  → showGiftPanel = true
  → GiftPanelBottomSheet 出现
  → GiftPanelViewModel.loadGifts() 触发
  → 若缓存命中（<60s）：立即渲染
  → 若缓存过期：显示 Loading 骨架屏 → HTTP 请求 → 渲染列表
```

### 2. 选中礼物与数量
```
用户点击 GiftCard
  → selectGift(giftId)
  → selectedGiftId 更新
  → GiftCard 组件重组，显示金色边框

用户点击 CountSelector Chip
  → selectCount(value)
  → selectedCount 更新
  → totalPrice 重新计算
  → 按钮文案即时更新 "送出 {totalPrice}💎"
```

### 3. 实时余额更新
```
WebSocket BalanceUpdated 事件
  → GiftPanelViewModel 监听并调用 updateBalance(newBalance)
  → BalanceBar 数值立即更新（无加载态）
  → 若 selectedGift != null && totalPrice > newBalance
    → canSend = false
    → 按钮自动禁用，文案改为 "余额不足"
```

### 4. 错误处理与重试
```
HTTP listGifts() 失败（IOException / HTTP 5xx / 解析异常）
  → UiState.error = "网络连接失败，请检查网络"
  → GiftPanelBottomSheet 显示骨架屏占位卡
  → 占位卡底部"点击重试"按钮可见
  → 用户点击 → onRetry 回调 → giftViewModel.retryLoad()
  → retryLoad() 清空缓存 + 重新调用 loadGifts()
  → UiState.loading = true → 骨架屏显示
  → 加载完成或再次失败 → 更新 UiState
```

### 5. SendGift 发送流程（T-30030 ✅）

```
用户点击"送出"按钮
  ↓
viewModel.sendGift() 触发 {
  1. 创建 SendGiftJob(msgId=UUID, giftId, recipientId, count, roomId)
  2. _state.update { it.copy(sending=true) }  // 按钮变灰 + 显示 Loading
  3. 构建 JSON：buildSendGiftJson(job) 用 Gson JsonObject API
  4. wsClient.send(json, msgId=job.msgId)
  5. withTimeoutOrNull(5_000L) 等待 SendGiftResultEvent
  6. 根据错误码处理：
     - code=0: 成功，toast"赠送成功"，面板保留
     - code=40290: 余额不足，触发 ShowInsufficientDialog（T-30032）
     - code=40403: 接收者离线，toast"接收者已下麦"，面板保留
     - code=40400: 你已离房，触发 DismissPanel
     - 超时/网络错: toast"请求超时，请重试"
  7. _state.update { it.copy(sending=false) }  // 按钮还原
}
  ↓
接收 GiftReceived 广播事件，驱动特效播放（T-30031）
```

**幂等设计**：
- 每次点击生成唯一 UUID `msg_id`
- Server 通过 `msg_id` 判重，重复请求返回首次结果
- MVP 策略：按钮发送中禁用 (`canSend && !sending`)，防止用户多发

**连击聚合**（ComboAggregator）：
```kotlin
// 3s 内同礼物+接收者累加 count，最后只发一次
val combo = aggregator.press(giftId, recipientId, count=1)
// 返回 Combo { giftId, recipientId, msgId, count: 累计数, lastTs }
```

### 6. 关闭面板
```
用户点击外部 / × 按钮 / 返回键 / DismissPanel 事件
  → RoomScreen.showGiftPanel = false
  → dismiss() 触发
  → UiState.selectedGiftId = null （清除选中态）
  → selectedCount 保留（可选设计决策）
  → BottomSheet 退出动画 → 销毁
```

---

## 五、多语言支持（Accept-Language）

### 实现流程
```
GiftPanelViewModel.loadGifts()
  ↓
val locale = LocalConfiguration.current.locale.language  // 从系统配置获取
  ↓
val gifts = repository.listGifts(locale)
  ↓
RetrofitGiftRepository.listGifts(locale)
  ↓
apiService.listGifts(acceptLanguage = locale)  // Retrofit 添加 Header
  ↓
OkHttp 请求：GET /api/v1/gifts/list
           Headers: Accept-Language: ar (或 en、pt 等)
  ↓
后端根据 locale 返回对应语言的 name 字段
  ↓
GiftCard 显示 gift.name（自动适配语言）
```

**支持的语言**（示例）：
- `ar` — 阿拉伯语
- `en` — 英语
- `pt` — 葡萄牙语
- ...（由后端决定支持列表）

---

## 六、测试覆盖

### 单元测试：`GiftPanelViewModelTest.kt`（20 个测试）

| 用例 | 验证内容 |
|-----|--------|
| G28-01 | BottomSheet 可见性检验 |
| G28-02 | 列表加载后渲染 ≥6 个 GiftCard |
| G28-03 | 选中礼物后显示金色边框 |
| G28-04 | 数量×单价=总价计算正确 |
| G28-05 | 余额不足时按钮禁用 |
| G28-06 | 关闭面板清除选中态 |
| G28-07 | WS BalanceUpdated 实时刷新 |
| G28-08 | 无人在麦时按钮禁用 + 提示 |
| G28-09 | 网络失败显示重试按钮 + onClick 绑定有效 |
| G28-10 | Accept-Language=ar 时显示阿拉伯语 name |
| R1-01 | retryLoad() 状态机：Error → Loading → Success |
| Extra-01~10 | 边界场景：空列表、zero balance、极大数量等 |

### 单元测试：`RecipientSelectorViewModelTest.kt`（12 个测试，T-30029 新增）

| 用例 | 验证内容 |
|-----|--------|
| R29-01 | 只显示在麦的用户（过滤 slot_index != null） |
| R29-02 | 首次渲染默认选主麦（slot=0，位置第一） |
| R29-03 | 点击切换选中项金色边框 + 底部实心金色圆点 |
| R29-04 | 原选中用户下麦后自动切换到主麦 |
| R29-05 | 全部下麦后显示空状态 + canSend=false |
| R29-07 | 新用户上麦后列表立即更新（无 3s 延迟） |
| Sort-01 | 多个用户时按 micIndex 升序排列 |
| Sort-02 | 主麦（slot=0）置首，乱序传入仍能正确排序 |
| Extra-01~04 | 边界：单用户、重复点击同一项、micIndex 冲突等 |

### 集成测试：`RetrofitGiftRepositoryTest.kt`（8 个测试）

| 用例 | 验证内容 |
|-----|--------|
| R01 | 成功响应正确映射为 GiftVO |
| R02 | IOException 捕获并转 Result.failure |
| R03 | HTTP 500 错误处理 |
| R04 | <60s 缓存命中，不发起新请求 |
| R05 | ≥60s 缓存过期，重新发起请求 |
| **MEDIUM-1** | **并发调用只发起一次 HTTP 请求**（验证 Mutex 有效） |
| locale 传递 | Accept-Language Header 正确构造 |
| 空列表 | 返回 `[]` 不产生异常 |

**预期结果**：总计 40 个新单元测试通过，Review Round 1 ✅

---

## 七、集成点与依赖关系

### 输入依赖（由外部提供）

| 依赖项 | 来源 | 用途 |
|-----|------|------|
| `RoomViewModel.recipients` | RoomScreen WS 事件 | 更新接收者列表 |
| `WalletViewModel.balance` | 钱包模块或 WS BalanceUpdated | 显示当前钻石余额 |
| `roomId` / `userId` | RoomScreen 状态 | HTTP 请求可选参数 |

### 输出依赖（提供给外部）

| 输出项 | 目标 | 用途 |
|-----|------|------|
| `GiftPanelViewModel` | RoomScreen | 状态管理与事件处理 |
| `onRetry` 回调 | GiftPanelBottomSheet | 错误重试绑定 |
| `showGiftPanel` 状态 | RoomScreen | 控制面板显示/隐藏 |
| `GiftPanelEvent` | RoomScreen LaunchedEffect | Toast/弹窗通知 |

### 后续接入（T-30030~T-30033）

| Task | 接入内容 | 状态 |
|-----|---------|------|
| **T-30030** | SendGift 客户端实现：UUID msg_id + 幂等 + 3s 连击聚合 + 5s 超时 | ✅ 完成（DoD） |
| **T-30031** | 送礼特效：GiftReceived 事件驱动动画播放 | 进行中 |
| **T-30032** | 余额不足弹窗：ShowRechargeHint 事件处理 | 待开发 |

---

## 八、SendGift 实现详解（T-30030 ✅）

### 新增类与组件

#### 1. SendGiftJob（单次发送作业）
```kotlin
data class SendGiftJob(
    val msgId: String,           // UUID，每次生成独立 msg_id
    val giftId: String,          // 礼物 ID
    val recipientId: String,     // 接收者用户 ID
    val count: Int,              // 数量（连击聚合后）
    val roomId: String = ""      // 房间 ID
)
```
**职责**：承载单次 SendGift 请求的完整信息，支持超时等待与结果匹配。

#### 2. ComboAggregator（连击聚合器）
```kotlin
class ComboAggregator(private val windowMs: Long = 3000) {
    data class Combo(
        val giftId: String,
        val recipientId: String,
        val msgId: String,
        val count: Int,      // ✅ val，不可变
        val lastTs: Long     // ✅ val，不可变
    )
    
    fun press(giftId: String, recipientId: String, unitCount: Int = 1): Combo {
        val now = System.currentTimeMillis()
        val c = current
        return if (c != null && c.giftId==giftId && c.recipientId==recipientId
                   && now - c.lastTs < windowMs) {
            // 3s 内同礼物+接收者，count 累加
            c.copy(count = c.count + unitCount, lastTs = now).also { current = it }
        } else {
            // 新的聚合周期，生成新 UUID msg_id
            Combo(giftId, recipientId, UUID.randomUUID().toString(), unitCount, now)
                .also { current = it }
        }
    }
    
    fun flush() { current = null }  // 清空，通常在发送后调用
}
```
**特性**：
- **不可变设计**：Combo 数据类所有字段为 `val`，每次更新通过 `copy()` 生成新实例
- **幂等 msg_id**：聚合周期内共用一个 msgId；超出窗口或 flush 后生成新 msgId
- **线程安全**（Main 线程）：假设单线程调用；若需多线程，可加 Mutex

#### 3. GiftEvents（事件定义）
```kotlin
// SendGift 结果事件
data class SendGiftResultEvent(
    val msgId: String,     // 与 SendGiftJob.msgId 关联
    val code: Int          // 错误码：0=成功, 40290=余额不足, 40403=接收者不可用, ...
)

// 礼物接收广播事件（所有房间人员都可收到）
data class GiftReceivedEvent(
    val senderId: String,
    val senderNickname: String,
    val senderAvatar: String?,
    val receiverUserId: String,
    val receiverNickname: String,
    val receiverAvatar: String?,    // ✅ R1-HIGH 修复：补充 avatar 字段
    val giftId: String,
    val giftName: String,
    val giftIconUrl: String,
    val count: Int,
    val effectLevel: Int,           // 特效等级（用于 T-30031 分层特效）
    val totalInRoom: Int            // 当前房间累计收到此礼物次数
)
```

#### 4. buildSendGiftJson() 安全构造（R1-MEDIUM 修复）

**原实现的问题**：字符串插值易被注入

❌ 不安全（原实现）：
```kotlin
private fun buildSendGiftJson(job: SendGiftJob): String {
    return """{
        "type": "SendGift",
        "msg_id": "${job.msgId}",
        "payload": {
            "gift_id": "${job.giftId}",
            "receiver_id": "${job.recipientId}",
            "count": ${job.count}
        }
    }"""
    // 若 giftId = "abc\"def"，JSON 会被破坏！
}
```

✅ 安全（修复后）：
```kotlin
private fun buildSendGiftJson(job: SendGiftJob): String {
    val payload = com.google.gson.JsonObject().apply {
        addProperty("room_id", job.roomId)
        addProperty("gift_id", job.giftId)
        addProperty("receiver_id", job.recipientId)
        addProperty("count", job.count)
    }
    return com.google.gson.JsonObject().apply {
        addProperty("type", "SendGift")
        addProperty("msg_id", job.msgId)
        add("payload", payload)
    }.toString()
    // Gson 自动转义 "、\、换行等特殊字符，安全可靠
}
```

**为什么使用 Gson JsonObject？**
- 自动转义特殊字符，避免 JSON 注入
- 类型安全：数值字段用 `addProperty(String, Number)`，Gson 自动序列化为数字（非字符串）
- 可读性好，对标 JSONBuilder 常见模式

---

## 九、错误代码与处理（完整映射）

| 错误代码 | 含义 | 客户端动作 | 相关 Task |
|---------|------|----------|---------|
| 0 | 成功 | Toast"赠送成功"，面板保留 | T-30030 ✅ |
| 40290 | 余额不足 | 触发 ShowInsufficientDialog（跳 T-30032） | T-30030 ✅ / T-30032 |
| 40403 | 接收者已离线 | Toast"接收者已下麦或离开"，面板保留 | T-30030 ✅ |
| 40402 | 礼物已下架 | Toast"该礼物已下架"，自动刷新列表 | T-30030 ✅ |
| 40400 | 你已离房 | DismissPanel 事件，关闭面板 | T-30030 ✅ |

---

## 十五、包路径与文件清单（更新）

```
app/android/app/src/main/java/com/voiceroom/
├── domain/
│   └── gift/
│       ├── IGiftRepository.kt
│       ├── GiftVO.kt
│       └── MicUserVO.kt
├── data/
│   ├── remote/
│   │   ├── api/
│   │   │   └── GiftApiService.kt
│   │   └── model/
│   │       └── GiftModels.kt
│   └── gift/
│       ├── RetrofitGiftRepository.kt
│       └── DebugGiftRepository.kt
└── feature/
    └── gift/
        ├── GiftPanelViewModel.kt       # T-30030 修改：新增 sendGift()、buildSendGiftJson()、handleSendGiftResult()
        ├── GiftPanelUiState.kt         # T-30030 修改：新增 sending: Boolean 字段
        ├── GiftPanelEvent.kt           # T-30030 修改：新增 ShowInsufficientDialog、DismissPanel
        ├── GiftPanelBottomSheet.kt     # T-30030 修改：发送按钮 Loading、新事件处理
        ├── SendGiftJob.kt              # ✅ T-30030 新增：发送作业描述对象
        ├── ComboAggregator.kt          # ✅ T-30030 新增：3s 连击聚合器
        └── components/
            ├── GiftCard.kt
            ├── CountSelector.kt
            ├── RecipientSelector.kt        # T-30029 新增
            └── BalanceBar.kt

app/android/app/src/main/java/com/voiceroom/core/ws/event/
├── GiftEvents.kt                           # ✅ T-30030 新增：SendGiftResultEvent、GiftReceivedEvent（含 receiverAvatar）

app/android/app/src/test/java/com/voiceroom/
├── feature/
│   └── gift/
│       ├── GiftPanelViewModelTest.kt (20 个测试，T-30028)
│       ├── RecipientSelectorViewModelTest.kt (12 个测试，T-30029)
│       └── SendFlowTest.kt               # ✅ T-30030 新增：15 个测试用例（S30-01~S30-15）
└── data/
    └── gift/
        └── RetrofitGiftRepositoryTest.kt (8 个测试)
```

---

## 十六、参考资源

- **TDS 文档**：`doc/tds/android/T-30028.md`、`doc/tds/android/T-30029.md`、`doc/tds/android/T-30030.md` （✅ T-30030 Complete）
- **Design 文档**：`doc/design/android/T-30028.md`、`doc/design/android/T-30029.md`、`doc/design/android/T-30030.md`
- **Protocol 文档**：`doc/protocol/websocket_signals.md` 
  - §6.4.1 BalanceUpdated（余额推送）
  - §6.4.2 SendGift（客户端请求信令）
  - §6.4.3 GiftReceived（服务端广播）
- **依赖库**：Coil 2.x（图片加载）/ Kotlin Coroutines（并发）/ Compose Material3（UI）/ Gson（JSON 安全构造）
- **Server 实现**：`doc/arch/server/gift.md` T-00020 SendGift 事务处理与幂等机制
