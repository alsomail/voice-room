# 测试套件：ANALYTICS 埋点与隐私合规（Android）

> **需求模糊点 (Ambiguity Notes)**：
> - "仅 Crash" 同意模式下，是否允许 login/logout 等用户生命周期事件上报，TDS 未显式列白名单，本套件按"全部 track() 调用均被拦截"断言，如实现为部分白名单需反馈。

覆盖 Task：T-30034（Analytics 防腐层 + Sentry 集成）、T-30035（EventReportClient + 核心埋点 + 隐私弹窗）。

---

## TC-ANALYTICS-00001：防腐层约束 - 业务层零 Sentry 直接依赖
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. Android 仓库 clone 完毕；可运行 `./gradlew :app:check` 和 shell。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 运行 `bash scripts/check_no_sentry_imports.sh` | 退出码 0；`app/src/main` 下 `import io.sentry` 数量 = 0（仅允许 `core/analytics/impl/`） |
| 2 | `Shell` | `grep -r "io.sentry" app/src/main --include='*.kt' | grep -v 'core/analytics/impl'` | 空输出 |
| 3 | `Android` | Hilt 模块注入 `AnalyticsPort` 默认实现 | 非 Debug 构建为 `SentryAnalytics`，Unit Test 为 `NoopAnalytics` |
| 4 | `Android` | BuildConfig.SENTRY_DSN 来自 `gradle.properties`/CI secret | 在 build.gradle 中读取 `findProperty("SENTRY_DSN")` 注入 |

**【数据清理】**
- 无。

---

## TC-ANALYTICS-00002：Sentry Crash 捕获 + 敏感字段脱敏
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. dev 环境 SENTRY_DSN 指向自建 Sentry；Dashboard 可访问。
2. App 在前台，U1（phone=+9660500000000）已登录。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 触发 Debug 菜单"模拟 Crash" | 应用崩溃；重启后 Sentry Dashboard 10s 内收到事件 |
| 2 | `Sentry` | 检查事件 tags | 含 `env=dev`、`user.id=U1`；不含 `phone` 原文 |
| 3 | `Android` | 调用 `analytics.captureException(RuntimeException("phone=+9660500000000 jwt=eyJabc"))` | Sentry 事件 message 为 `phone=*** jwt=***` |
| 4 | `Android` | 触发模拟 ANR（5s 主线程阻塞） | Sentry 收到 ANR 事件 |
| 5 | `Android` | 用户"仅 Crash"同意模式下再次模拟 Crash | Sentry 仍收到（合规豁免） |

**【数据清理】**
- 无。

---

## TC-ANALYTICS-00003：隐私弹窗 + 同意模式分流
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 清除 App 数据（首次启动态）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 启动 App，通过 Splash 进入 | 主页之前弹出 PrivacyConsentDialog，两个按钮 `Key('btn_privacy_agree')` 与 `Key('btn_privacy_crash_only')` |
| 2 | `Android` | 返回键/点击外部 | 弹窗不可关闭 |
| 3 | `Android` | 点击"仅 Crash" | DataStore consent_mode=`crash_only`；关闭弹窗进入主页 |
| 4 | `Logcat` | 观察非 Crash 埋点日志 | 30s 内 Logcat 无 `EventReportClient.track(` 实际上报，也无 HTTP/WS POST /events |
| 5 | `Android` | 切回设置页点击"开启完整分析" | 写入 consent_mode=`full`；后续 `track()` 开始入队 |
| 6 | `Android` | 再次冷启动 | 不再弹窗（已有同意记录） |

**【数据清理】**
- 清 App 数据。

---

## TC-ANALYTICS-00004：EventReportClient 节流队列 + WS/HTTP 通道切换
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. 已同意 `full` 模式；WS 在线。
2. Logcat 可过滤 `EventReport`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 快速 track 7 条事件 | 队列积压 7 条，未触发 flush（阈值 ≥8 条） |
| 2 | `Android` | track 第 8 条 | 立即触发 flush；WS 发 `ReportEvent` 一次含 8 事件 |
| 3 | `Android` | 单条 track 后静置 2min | 到达时间阈值，自动 flush 1 条 |
| 4 | `Android` | 断网，连续 track 20 条 | 队列积累；WS 重连后优先 flush 通过 WS |
| 5 | `Android` | 断网 5min，期间 1050 条事件 | 队列保留 1000 条（丢弃最早 50 条，Logcat 有 WARN）；恢复后全部成功上报 |
| 6 | `Server` | 采集 WS vs HTTP 上报比例（5min 在线样本） | WS 通道上报数 / 总数 > 80% |
| 7 | `Android` | track event 时网络 Airplane Mode | 事件持久化到 Room `event_queue` 表；kill 进程重启后仍可恢复 |

**【数据清理】**
- 清事件表；恢复网络。

---

## TC-ANALYTICS-00005：公共字段自动注入 + 事件字典覆盖
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. full 同意模式；U1 已登录；进入房间 R1 送礼一次。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | U1 登录成功后 | 上报事件 `login_verify_success`，properties 含 `session_id`/`device_id`/`app_version`/`os_version`/`locale`/`network_type` |
| 2 | `Android` | 进入 R1 送 520 礼物成功 | 触发 `gift_send_success`，properties 含 `gift_id`、`count`、`room_id` |
| 3 | `Android` | 送礼失败（余额不足） | 触发 `insufficient_balance_dialog_shown`、`gift_send_fail` 两条 |
| 4 | `AppServer` | DB 查 events 表 | 上述事件均已落库，server_ts 为服务端时间 |
| 5 | `Android` | session_id 在应用存活期唯一 | 两次 track 的 session_id 相同；kill App 重启后 session_id 变更 |
| 6 | `Android` | 任何事件 properties 扫描 | 不含字段 `phone`、`jwt`、`token`、精确坐标 `lat`/`lng`（敏感过滤器） |

**【数据清理】**
- 清本用例事件。

---

## TC-ANALYTICS-00006：Consent 变更后队列行为
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. full 模式；队列内已有 3 条未上报事件。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 用户切换为 `crash_only` | 队列中 3 条非 Crash 事件被清空；DataStore consent_mode=`crash_only` |
| 2 | `Android` | 新 track 10 条非 Crash 事件 | 队列长度仍为 0；Logcat WARN `track() dropped: consent=crash_only` |
| 3 | `Android` | captureException 一次 | Sentry 正常收到 |
| 4 | `Android` | 用户再次切回 `full` | 后续新事件恢复入队 |

**【数据清理】**
- 清队列。
