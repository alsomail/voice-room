# QA Gate Regression Report — 20260429-081255

> **任务关联**: C方案 B阶段 — 模块 1/2/4/7 Android 切片 QA Gate  
> **执行人**: E2E-Runner (Android QA Agent)  
> **执行时间**: 2026-04-29 08:12:55  
> **报告目录**: `tests/report-20260429-081255/`

---

## 🔴 最终战报（已熔断 — 等待 master 处置）

| 套件 | 设备 | PASS | FAIL | SKIP | 结论 |
|------|------|------|------|------|------|
| Android instrumentation | Pixel 4 / API 33 | 0 | 57 | 0 | 🚫 **BLOCK** |
| **合计** | — | **0** | **57** | **0** | 🔴 **熔断** |

---

## 根本原因

**BUG-ANDROID-001**: `MenaColors.kt` 使用 `Color(ULong)` 构造函数传入 ARGB 十六进制值，导致低 6 位被误解为颜色空间 ID（colorspace ID），而 Android 13 上仅有 18 个合法颜色空间（ID 0-17）。  
例：`Color(0xFFD4AF37uL)` → colorspace ID = 55 → `ArrayIndexOutOfBoundsException: length=18; index=55`

**影响范围**: 所有 57 个 instrumentation 测试 100% 崩溃。  
**需要修复**: `MenaColors.kt` 第 28-38 行，11 处 `Color(VALUE)` → `Color(VALUE.toInt())`  
**修复者**: TDD Agent（业务代码变更，E2E Agent 无权操作）

---

## 模块覆盖汇总

| 模块 | 端 | 关联 Task | QA 结论 |
|------|----|----------|---------|
| 模块 1 — 用户认证 | Android | T-30001 (LoginScreenVisualTest 19 tests) | ❌ FAIL / BUG-ANDROID-001 |
| 模块 1 — 用户认证 | Android | T-30002/T-30003/T-30004 | ⏭️ SKIP-OOS (ViewModel/Repo JVM-only) |
| 模块 2 — 房间大厅 | Android | T-30005/T-30006/T-30007 | ⚠️ 未执行 (待查原因) |
| 模块 4 — 黑金主题 | Android | T-30018~T-30026 (9 tasks) | ❌ FAIL (>5) / BUG-ANDROID-001 → 🚫 BLOCK |
| 模块 7 — 埋点基建 | Android | T-30034/T-30035 | ⚠️ 未执行 (待查原因) |

---

## 状态机汇总

| 场景 | 当前状态 |
|------|---------|
| Android MenaTheme (T-30018) | TDD \| ❌ FAILED \| 1/5 |
| Android LoginScreenVisual (T-30021/T-30001) | TDD \| ❌ FAILED \| 1/5 |
| Android PlaceholderScreen (T-30023) | TDD \| ❌ FAILED \| 1/5 |
| Android MainScreen (T-30020) | TDD \| ❌ FAILED \| 1/5 |
| Android 模块4其余 (T-30019/22/24/25/26) | TDD \| ❌ FAILED \| 1/5 (推断) |
| Android 模块2 (T-30005~7) | ⚠️ 未执行 |
| Android 模块7 (T-30034~35) | ⚠️ 未执行 |

---

## 参考链接

- 详细分析: [ANDROID.md](./ANDROID.md)
- A阶段参考: [tests/report-20260429-072049/SUMMARY.md](../report-20260429-072049/SUMMARY.md)
