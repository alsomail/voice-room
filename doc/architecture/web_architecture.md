# 6. Web 架构：Admin 管理后台 (Ant Design + React)

## 6.1 定位说明

Web 端定位为 **B 端后台管理系统（Admin Web）**，面向运营人员和客服，通过 VPN 访问。  
**不是 C 端用户页面**，不涉及 WebSocket、RTC、IM 等实时通信能力。

## 6.2 约束

- 页面（pages）只负责组合 `features` 和 `components`，不直接写请求细节。
- 所有 HTTP 必须走 `api/client.ts` 或 `core/network` 统一拦截器，目标为 **Admin Server**（非 App Server）。
- UI 组件库统一使用 **Ant Design**，禁止引入 shadcn/ui 或其他竞争组件库。
- 全局状态管理使用 **Zustand**，管理员登录态、权限信息集中在 `useAuthStore`。
- 路由守卫基于 Admin JWT 的 `role` 字段做 **RBAC 前端权限控制**，不同角色看到不同菜单。
- 错误边界、异常上报必须统一走 `core/telemetry` 基建层。
- **不需要** WS 客户端、RTC 防腐层、IM 适配层。
