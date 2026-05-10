# 测试套件：Web AppLayout 侧栏 RBAC 菜单可见性矩阵（🚨 已下线，等真实路由+菜单代码摸清后重写）

> **本文件 v1 因虚构 RBAC 矩阵被下线**：
> - 凭空假设了 4 角色 × 7 菜单（Dashboard / 房间管理 / 用户管理 / 礼物管理 / 治理日志 / 操作日志 / 系统设置）矩阵，**未读** `app/web/src/app/AppLayout.tsx` 与 `app/web/src/router/index.tsx`；
> - 真实 router 中目前**只有** `/gifts` 与 `/governance` 显式包了 `RoleGuard`，其它菜单是否在 AppLayout 按角色过滤未核实；
> - finance 越权跳 `/403` 还是 `/dashboard` 是凭空假设；Zustand `useAuthStore.admin.role` 被篡改后服务端 403 兜底链路也需在 AdminServer 真实代码中确认。
>
> **重写计划**：
> 1. 先读 `app/web/src/app/AppLayout.tsx` 拿到**真实菜单清单 + 各项 role 过滤**；
> 2. 再读 `app/web/src/router/index.tsx` 与所有 `RoleGuard` 使用点拿到**真实路由守卫矩阵**；
> 3. 然后只针对**已存在**的菜单 + 守卫写覆盖用例，禁止再凭空扩展角色或菜单。

<!-- 历史 v1 内容已废弃，禁止参照执行 -->

