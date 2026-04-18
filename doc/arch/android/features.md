# Android 业务骨架与测试现状

## 一、 Feature 现状

| 模块 | 关键文件 | 当前状态 |
| --- | --- | --- |
| Auth | `feature/auth/AuthFeature.kt` | 🟡 仅保留模块描述，登录/刷新 Token 尚未实现 |
| Room | `feature/room/RoomFeature.kt` | 🟡 已强调“服务端权威”，但房间状态仍是占位 |
| Profile | `feature/profile/ProfileFeature.kt` | 🟡 仅保留模块描述与后续落点 |
| Gift / Wallet / Seat / Family / CP / VIP / Backpack / Game | `feature/*/.gitkeep` | 🔴 仅目录预留，尚无 UI 与逻辑 |

## 二、 当前测试覆盖

| 测试文件 | 覆盖范围 |
| --- | --- |
| `common/AppContainerTest.kt` | 校验容器装配、Debug 依赖注入与 `NoOp` 能力可调用 |
| `core/config/AppEnvironmentTest.kt` | 校验环境值裁剪与物理机 Loopback 警告 |
| `core/network/AppHttpClientFactoryTest.kt` | 校验 OkHttp 超时与重试参数 |
| `core/ws/RoomSocketRequestFactoryTest.kt` | 校验 WS URL 拼接、鉴权头与 OkHttp 兼容转换 |
| `presentation/MainViewModelTest.kt` | 校验默认页面状态、模块切换与基础埋点记录 |
| `androidTest/presentation/MainActivitySmokeTest.kt` | 校验首页可启动且默认标题正确 |

## 三、 对业务推进的含义

- 当前 Android 端已经具备继续向真实业务演进的基础工程脚手架。
- 但除壳层展示外，绝大多数业务状态仍不是来自真实 API / WS / RTC / IM。
- 后续开发必须优先对齐 `doc/protocol.md` 与服务端广播模型，避免客户端自行推断核心状态。
