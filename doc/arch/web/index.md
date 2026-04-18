<!--
[AI 读写指令与维护规约 (Doc Management Skill)]
1. 本文件是 Web 架构的总路由，严禁在此文件内编写具体业务逻辑或冗长代码片段。
2. 架构拆分为独立的子 Markdown 文件存放于本目录下。
3. [索引规则]：当你在本目录新增了 `.md` 子文件，必须立即同步更新本文件的【二、子模块索引】。
4. [状态规则]：当某项能力完成开发，必须同步更新本文件的【三、当前能力全景与状态】。
5. 所有的相对路径链接必须真实有效，禁止生成无法点击的死链接。
-->

# Web 端（Admin 管理后台）架构总索引与状态盘点

## 一、 架构概述
当前 Web 端定位为 **B 端后台管理系统（Admin Web）**，面向运营人员和客服，通过 VPN 访问 Admin Server。
技术栈：React + Vite + TypeScript + Ant Design + Zustand。
已完成 Vite 工程脚手架与基础环境配置；业务页面（管理员登录、数据看板、用户管理、房间管理、操作日志）尚未开发。
**重要**：Web 端只通过 HTTP 与 Admin Server 通信，不涉及 WebSocket、RTC、IM 等实时通信能力。

## 二、 子模块索引 (Module Router)
> ⚠️ AI 寻路提示：Web 端是后台管理系统，面向 Admin Server 的 HTTP API。不涉及 C 端用户登录、WebSocket、RTC 或 IM。

### 实际目录：
- 🧱 [目录结构与入口链路](./structure.md) - `main.tsx`、`App`、`HomePage`、环境变量与基础 helper 现状。
- 📡 [Telemetry 与网络能力现状](./status.md) - 埋点 mock、URL 约束、WS/HTTP helper 与未落地项。

## 三、 当前能力全景与状态 (Capability Matrix)
> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发

### 核心能力
- 🟢 React + Vite + TypeScript 工程、构建脚本与 `VITE_` 环境变量约束
- 🟢 基础 HTTP 客户端封装 (`apiClient`)
- 🔴 Ant Design 组件库集成与全局主题配置
- 🔴 管理员登录页（账号密码登录，对接 Admin Server）
- 🔴 路由守卫与 RBAC 前端权限控制（基于 Admin JWT role 字段）
- 🔴 Zustand 全局状态管理（useAuthStore）
- 🔴 数据看板首页（ECharts 趋势图）
- 🔴 用户管理页面（搜索/封禁/解封）
- 🔴 房间管理页面（查看/强制关闭）
- 🔴 操作日志页面
- 🔴 中英文国际化（i18n，不含 RTL — 后台不需要阿语）

### 遗留技术债 (Tech Debt)
- 当前工程脚手架仍保留 C 端时期的 telemetry mock 和 WS helper，需要在后续重构中清理。
- `src/services/` 下的 RTC/IM 适配层在 Admin Web 中不需要，应删除或标记为 deprecated。
- Ant Design 尚未引入，当前仍使用临时样式。
- API 客户端尚未配置 Admin Server 的 baseURL 和 JWT 拦截器。
