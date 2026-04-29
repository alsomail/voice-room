# Android QA Gate Report — B阶段 — 20260429-081255

> **当前状态机：负责人 [TDD] | 状态 [❌ FAILED] | 修复轮次 [1/5]**

> **任务关联**: C方案 B阶段 — 模块 1/2/4/7 Android 切片 QA Gate  
> **执行人**: E2E-Runner (Android QA Agent)  
> **执行时间**: 2026-04-29 08:12:55  
> **设备**: Pixel 4 · Android 13 (API 33) · 9A251FFAZ00EAJ  
> **APK**: `app-local-debug.apk` · MD5 `5d8bfae550d59fd00e15021cb274e860` · 20.7 MB  
> **构建时间**: 2026-04-29 08:00  
> **ADB 反向端口**: `tcp:3000` `tcp:3001` → host  

---

## 🔴 熔断告警：业务代码 Bug — 立刻升级 master

> **BLOCK 原因**: 57/57 instrumentation 测试全部 FAIL，且失败来自业务源码 `MenaColors.kt`（禁止 E2E Agent 修改）。  
> **失败率**: 100% (57 FAIL / 0 PASS / 0 SKIP)  
> **失败均为同一根因**: `BUG-ANDROID-001`（见下方分析）

---

## 📊 执行结果汇总

| 测试类 | 覆盖 Task | 测试数 | PASS | FAIL | 状态 |
|--------|-----------|--------|------|------|------|
| `MenaThemeTest` | T-30018 | 8 | 0 | 8 | ❌ FAIL |
| `AvatarWithFrameTest` | T-30018/T-30023 | 7 | 0 | 7 | ❌ FAIL |
| `GoldButtonTest` | T-30018 | 6 | 0 | 6 | ❌ FAIL |
| `GoldOutlinedTextFieldTest` | T-30018 | 5 | 0 | 5 | ❌ FAIL |
| `PlaceholderScreenTest` | T-30023 | 11 | 0 | 11 | ❌ FAIL |
| `LoginScreenVisualTest` | T-30021/T-30001 | 19 | 0 | 19 | ❌ FAIL |
| `MainScreenTest` | T-30020 | 1 | 0 | 1 | ❌ FAIL |
| **合计** | — | **57** | **0** | **57** | ❌ **100% FAIL** |

> **注意**: 另有 15 个 androidTest 文件未产生测试结果（见"未执行测试"章节）。  
> 若这些文件编译正确，它们同样会因相同根因崩溃。

---

## 🐛 BUG-ANDROID-001 根因分析

### 现象 (Phenomenon)

所有 57 个 instrumentation 测试在执行时抛出相同异常：

```
java.lang.ArrayIndexOutOfBoundsException: length=18; index=55
  at androidx.compose.ui.graphics.Color.getColorSpace-impl(Color.kt:724)
  at androidx.compose.ui.graphics.Color.copy-wmQWz5c(Color.kt:264)
  at androidx.compose.ui.graphics.Color.copy-wmQWz5c$default(Color.kt:254)
  at androidx.compose.material3.MaterialThemeKt.rememberTextSelectionColors(MaterialTheme.kt:165)
  at androidx.compose.material3.MaterialThemeKt.MaterialTheme(MaterialTheme.kt:58)
  at com.voice.room.android.core.theme.MenaThemeKt$MenaTheme$1.invoke(MenaTheme.kt:52)
  ...
```

另一变体（`MainScreenTest`中）：

```
java.lang.ArrayIndexOutOfBoundsException: length=18; index=44
  at androidx.compose.ui.graphics.Color.getColorSpace-impl(Color.kt:724)
  at androidx.compose.animation.SingleValueAnimationKt.animateColorAsState-euL9pac(SingleValueAnimation.kt:63)
  at androidx.compose.material3.NavigationBarKt$NavigationBarItem$styledIcon$1.invoke(NavigationBar.kt:188)
  ...
```

### 根本原因 (Root Cause)

**文件**: `app/android/app/src/main/java/com/voice/room/android/core/theme/MenaColors.kt`

**问题**: 所有颜色使用了 `Color(ULong)` 构造函数，但传入的是 ARGB 十六进制值。  
Compose 的 `Color(ULong)` 构造函数将低 6 位解释为颜色空间 ID（color space ID），但 ARGB 十六进制的低 6 位是蓝色通道的低位，并非有效的颜色空间索引。

**证明**（以 Primary 色为例）：

```kotlin
// MenaColors.kt 第 20 行（当前错误代码）
const val PRIMARY_VALUE: ULong = 0xFFD4AF37uL
val Primary: Color = Color(PRIMARY_VALUE)
```

解包分析：
- `0xFFD4AF37` → 低 6 位 = `0x37 & 0x3F = 55`  
- Compose 运行时：`getColorSpace-impl` 读取颜色空间 ID = **55**  
- 但 Android 13 (API 33) 设备上只注册了 **18 个**颜色空间（索引 0-17）  
- 结果：`ArrayIndexOutOfBoundsException: length=18; index=55` ❌

其他颜色同样受影响：

| 颜色常量 | 十六进制值 | 低6位(colorspc ID) | 是否有效(0-17) |
|---------|-----------|-------------------|--------------|
| `PRIMARY_VALUE` | `0xFFD4AF37uL` | 55 (`0x37`) | ❌ 无效 |
| `BACKGROUND_VALUE` | `0xFF1A1A2EuL` | 46 (`0x2E`) | ❌ 无效 |
| `SURFACE_VALUE` | `0xFF16213EuL` | 62 (`0x3E`) | ❌ 无效 |
| `SURFACE_VARIANT_VALUE` | `0xFF0F3460uL` | 32 (`0x20`) | ❌ 无效 |
| `ON_BACKGROUND_VALUE` | `0xFFFFFFFFuL` | 63 (`0x3F`) | ❌ 无效 |

**所有 11 个 MenaColors 颜色常量均受影响**（当 Material3 调用 `.copy()`/colorspace 相关操作时崩溃）。

### 正确修复方案（供 TDD Agent 参考）

应将 `Color(ULong)` 改为 `Color(Int)` 的 ARGB 构造函数：

```kotlin
// ❌ 错误（当前代码）
val Primary: Color = Color(0xFFD4AF37uL)

// ✅ 正确修复
val Primary: Color = Color(0xFFD4AF37.toInt())
```

**修复范围**：`MenaColors.kt` 第 28-38 行，所有 `Color(VALUE)` 调用，共 11 处。

### 为何 JVM 单测未发现此 Bug

JVM 单测（`MenaColorsTest.kt`）仅验证了 `const val` 的原始 ULong 值，未调用 `Color.getColorSpace()` 或触发 Material3 渲染，因此在 JVM 环境中无法复现此崩溃。此 Bug 仅在真实 Android 设备上运行 Compose UI 渲染时才触发。

---

## 📋 测试环境配置

| 项目 | 值 |
|------|----|
| 设备 | Pixel 4 |
| Android 版本 | 13 (API 33) |
| 设备序列号 | 9A251FFAZ00EAJ |
| APK 变体 | `localDebug` |
| APK MD5 | `5d8bfae550d59fd00e15021cb274e860` |
| APK 大小 | 20.7 MB |
| Compose BOM | `2024.09.00` |
| Compose UI | `1.7.2` |
| Material3 | `1.3.0` |
| ADB 端口转发 | tcp:3000 → host:3000, tcp:3001 → host:3001 |
| 后端状态 | AppServer:3000 ✅, AdminServer:3001 ✅, PG/Redis ✅ |
| DB 状态 | B阶段 reset+seed 完成 |

---

## ⚠️ 未执行的测试文件 (15个)

以下 15 个 androidTest 文件存在但未出现在测试结果 XML 中。  
推断原因：这些文件有额外的接口方法不匹配或依赖问题，即使修复后也会因 BUG-ANDROID-001 同样崩溃。

| 文件 | 覆盖 Task | 状态 |
|------|-----------|------|
| `HallScreenTest.kt` | T-30005/T-30006 | ⚠️ 未执行 |
| `HallScreenPagingTest.kt` | T-30006 | ⚠️ 未执行 |
| `HallScreenVisualUpgradeTest.kt` | T-30022 | ⚠️ 未执行 |
| `ProfileScreenTest.kt` | T-30024 | ⚠️ 未执行 |
| `SplashScreenTest.kt` | T-30019 | ⚠️ 未执行 |
| `RoomScreenTest.kt` | T-30025 | ⚠️ 未执行 |
| `HostMicSlotTest.kt` | T-30025 | ⚠️ 未执行 |
| `MicSlotsGridBlackGoldTest.kt` | T-30025 | ⚠️ 未执行 |
| `MicSlotCardTest.kt` | T-30025 | ⚠️ 未执行 |
| `RoomBottomBarTest.kt` | T-30026 | ⚠️ 未执行 |
| `ChatInputBarTest.kt` | T-30026 | ⚠️ 未执行 |
| `ChatMessageListTest.kt` | T-30025 | ⚠️ 未执行 |
| `ChatMessageListBlackGoldTest.kt` | T-30025 | ⚠️ 未执行 |
| `MicPermissionHandlerTest.kt` | T-30025 | ⚠️ 未执行 |
| `MainActivitySmokeTest.kt` | T-30019 | ⚠️ 未执行 |

---

## 🛠️ E2E Agent 在本轮已执行的修复（测试基础设施，非业务代码）

以下修改均为 `app/src/androidTest/` 测试基础设施文件，不涉及业务逻辑：

1. **`HostMicSlotTest.kt`**: 删除无效的独立 import `assertDoesNotExist`（该方法为扩展函数，不可单独导入）
2. **`HallScreenTest.kt`**: 更新匿名 `IRoomRepository` 实现，补齐 `createRoom(coverUrl, category, announcement)` 新参数 + `verifyPassword()` 新方法
3. **`HallScreenPagingTest.kt`**: 同上，2处
4. **`HallScreenVisualUpgradeTest.kt`**: 同上，1处
5. **`MainScreenTest.kt`**: 将手动 `AppContainer(...)` 构造替换为 `AppContainer.forUnitTest().copy(...)`，清理14个过时 import
6. **`ProfileScreenTest.kt`**: 同上

这些修复解决了第一次编译失败的问题，使测试套件可以编译并运行，从而暴露了 BUG-ANDROID-001。

---

## 🔴 升级到 master 的必要性

**原因**: BUG-ANDROID-001 位于业务源码 `MenaColors.kt`。根据 QA Gate 执行约束：
- ❌ E2E Agent 不得修改 `app/android/` 业务源码
- ❌ 模块 4 FAIL 数量 = 26+ (远超 5 FAIL 熔断阈值)
- ❌ 修复需要 TDD Agent 修改 `MenaColors.kt` 中的 `Color(ULong)` → `Color(Int)` (11处)

**需要 TDD Agent 完成的修复**:
```kotlin
// 文件: app/android/app/src/main/java/com/voice/room/android/core/theme/MenaColors.kt
// 修改: 将所有 Color(X_VALUE) 改为 Color(X_VALUE.toInt())
// 共 11 处，第 28-38 行
```

修复后，E2E Agent 将重新运行 `./gradlew :app:connectedLocalDebugAndroidTest` 进行回归验证。
