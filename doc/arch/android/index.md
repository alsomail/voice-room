<!-- 
[AI 读写指令与维护规约 (Doc Management Skill)]
1. 本文件是 Android 架构的总路由，严禁在此文件内编写具体的业务逻辑或冗长的代码片段。
2. 架构拆分为独立的子 Markdown 文件存放于本目录下。
3. [索引规则]：当你在本目录新增了 `.md` 子文件，必须立即同步更新本文件的【二、子模块索引】。
4. [状态规则]：当某项能力完成开发，必须同步更新本文件的【三、当前能力全景与状态】。
5. 所有的相对路径链接必须真实有效，禁止生成无法点击的死链接。
-->

# Android 端架构总索引与状态盘点

## 一、 架构概述
本项目 Android 端当前采用 Kotlin + 分层骨架（`core / common / data / domain / presentation / feature`）推进建设，`ViewModel + MainUiState` 已在壳层页面落地，完整业务级状态管理仍在后续演进中。详情参见全局 `/doc/ARCHITECTURE.md` 的第 5 节。

## 二、 子模块索引 (Module Router)
> ⚠️ AI 寻路提示：请点击以下具体模块查看详细架构说明、API 映射和代码存放路径。
### 实际目录：
- 🧱 [启动装配与壳层页面](./bootstrap.md) - `Application`、`AppContainer`、`MainActivity`、`MainViewModel` 的当前链路。
- 🌐 [核心基建与防腐层骨架](./foundation.md) - 环境配置、HTTP、WebSocket、遥测、媒体、IM 与调试适配器现状。
- 🧩 [业务骨架与测试现状](./features.md) - `auth/room/profile` 壳层能力、预留模块以及测试覆盖面。

## 三、 当前能力全景与状态 (Capability Matrix)
> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发 

### 核心能力
- 🟢 Application 启动装配、`BuildConfig` 环境注入与 `AppContainer` 依赖装配
- 🟢 HTTP 客户端工厂、`RoomSocketRequestFactory` 与物理机 Loopback 预警
- 🟢 `MainActivity`/`MainViewModel` 壳层页面、阿拉伯语资源入口与基础导航切换
- 🟡 `auth` / `room` / `profile` Feature 已有占位描述，Repository / Service 仍为 Debug 实现
- 🟡 Telemetry / Media / IM 已通过接口与 `NoOp*` 适配器隔离第三方依赖
- 🔴 WebSocket 长连接状态机、真实鉴权、房间同步、RTC/IM 接入
- 🔴 钱包、礼物、麦位、家族、CP、VIP、背包、小游戏等业务页面与数据链路


### 遗留技术债 (Tech Debt)
- 目前仅有 XML 壳层页面，尚未落地真正的业务流与状态管理闭环。
- `feature/` 下多数模块仍以 `.gitkeep` 预留，未接入真实 API、WS 协议与服务端广播。
- `core` 层已完成接口隔离，但远程配置、本地存储、安全、日志等能力还只有骨架或占位。
