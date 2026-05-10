# 测试套件：Android 弹性场景（🚨 已下线，等真实代码 + 环境能力摸清后重写）

> **本文件 v1 因虚构 / 环境能力假设过强被下线**：
> - 多处依赖 `adb shell tc qdisc add ... netem`（root 权限），`svc wifi disable`（Android 14+ 模拟器已禁），`content insert SET_LOCALE`（不可靠），实测环境普遍不具备这些能力；
> - 引用了"指数退避 1s→2s→4s"、"30s 隐式 LeaveMic"、"前台保活心跳 + 35s 新 session"、"client_msg_id 幂等" 等机制，**未对照** `WsClient` / `SessionManager` / `chat_messages` 真实代码核实；
> - 屏幕旋转 + 礼物面板状态保留依赖 `rememberSaveable` 实现细节，未跟 Compose 真实代码对齐。
>
> **重写计划**：
> 1. 主流程套件 `E2E/TC-MAIN-FLOW.md` 落地后，先在套件内做 1 条「WS 拔网→重连」的最小可执行用例（仅 `kill 9 vr-app-server` 模拟服务端短暂不可用）；
> 2. 其余环境强依赖项（root tc qdisc / 旋转 / 进程被杀）按 `regression_level: P2 + env-gated`，不满足时由 envProbe.ts SKIP 而非 FAIL；
> 3. 所有"自动 LeaveMic / 心跳超时 / 幂等"等服务端行为，**先到 `app/server/src/modules/{ws,mic,chat}` 读真实实现，再写断言**。

<!-- 历史 v1 内容已废弃，禁止参照执行 -->

