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
已完成 Vite 工程脚手架与基础环境配置、管理员登录页 UI（T-20001）、登录逻辑与路由守卫（T-20002）、数据看板首页（T-20003）、房间管理页面（T-20004）、房间详情弹窗（T-20005）、用户管理页面（T-20006）、用户详情抽屉（T-20007）、封禁对话框（T-20008）、操作日志页面（T-20009）、解封弹窗（T-20010）、活水房间监控增强（T-20011）。
**重要**：Web 端只通过 HTTP 与 Admin Server 通信，不涉及 WebSocket、RTC、IM 等实时通信能力。

## 二、 子模块索引 (Module Router)
> ⚠️ AI 寻路提示：Web 端是后台管理系统，面向 Admin Server 的 HTTP API。不涉及 C 端用户登录、WebSocket、RTC 或 IM。

### 实际目录：
- 🧱 [目录结构与入口链路](./structure.md) - `main.tsx`、`App`、`HomePage`、环境变量与基础 helper 现状。
- 📡 [Telemetry 与网络能力现状](./status.md) - 埋点 mock、URL 约束、WS/HTTP helper 与未落地项。
- 🔐 [Auth 模块（登录页 UI + Zustand 状态管理 + 路由守卫）](./auth.md) - `LoginPage`/`LoginForm` 组件结构、`useAuthStore` JWT 状态管理、`AuthGuard` 路由守卫、401 拦截器、localStorage XSS 风险说明（T-20001 + T-20002）。
- 🏠 **房间管理模块**（T-20004 ✅ · T-20005 ✅ · T-20011 ✅）- 路由 `/rooms`（在 `AuthGuard` 内）；涉及以下文件：
  - `src/pages/rooms/index.tsx` — `RoomsPage` 页面入口，组合 Hook + 组件；透传 `activityFilter`/`setActivityFilter`/`filteredItems` 给 `RoomsTable`（T-20011）
  - `src/pages/rooms/useRoomsPage.ts` — `useRoomsPage` Hook：分页（pageSize=20）、状态过滤（active/closed/all）、关键词搜索（300 ms debounce）、关闭房间（`closingId` 细粒度 loading）、AbortController 防竞态、`selectedRoomId` 作为 T-20005 接口契约；**T-20011 扩展**：新增 `activityFilter`（`ActivityFilter` 类型，默认 `'all'`）/ `setActivityFilter` / `filteredItems`（`useMemo` 纯前端过滤，不触发新 API 请求）
  - `src/pages/rooms/RoomsTable.tsx` — `RoomsTable` 组件：Ant Design Table、工具栏（搜索框 + 状态下拉 + **活跃度下拉** + 刷新按钮）、Popconfirm 二次确认关闭、closed 行关闭按钮禁用；**T-20011 扩展**：新增「活跃状态」列（`RoomActivityTag`）、「持续时长」列（`formatDuration`）、异常房间行高亮（`rgba(231,76,60,0.1)`）；新增 Props：`activityFilter?: ActivityFilter`（默认 `'all'`）/ `onActivityFilterChange?: (filter: ActivityFilter) => void`（默认 noop，向后兼容）
  - `src/pages/rooms/RoomStatusTag.tsx` — `RoomStatusTag` 组件：active=绿色、closed=灰色（使用 i18n）
  - `src/pages/rooms/roomUtils.ts` — **T-20011 新增**：活跃状态纯函数工具库；导出类型 `ActivityLevel`（`'active' | 'quiet' | 'abnormal' | 'normal'`）/ `ActivityFilter`（`'all' | 'active' | 'quiet' | 'abnormal'`）；`getActivityStatus(room, now?)` 按优先级规则计算活跃等级（≥5人→active / 0人且active状态→abnormal / 1-4人且>1h→quiet / 其余→normal）；`formatDuration(createdAt, now?)` 格式化持续时长（`0m` / `35m` / `2h 35m` / `3d 2h`）；`filterByActivity(items, filter, now?)` 纯前端列表过滤；所有函数注入 `now` 参数支持确定性单元测试
  - `src/pages/rooms/RoomActivityTag.tsx` — **T-20011 新增**：`RoomActivityTag` 组件；Props：`{ level: ActivityLevel; roomId: string }`；颜色映射：active=success（绿）/ quiet=warning（黄）/ abnormal=error（红）/ normal=processing（蓝）；`useMemo` 缓存 labelMap；`data-testid="room-activity-tag-{roomId}"` 供测试定位
  - `src/pages/rooms/useRoomDetail.ts` — `useRoomDetail(roomId)` Hook：监听 roomId 变化，调用 `adminGetRoomDetail`，含 AbortController 防竞态，返回 `{ detail, loading, error }`（T-20005）
  - `src/pages/rooms/RoomDetailModal.tsx` — `RoomDetailModal` 组件：Ant Design Modal（`destroyOnHidden={true}`，切换房间时清除旧数据）展示房间详情（基本信息 + 占位成员列表 + 占位聊天记录）；[强制关闭] 按钮使用 `Modal.confirm` 二次确认，`closeRoom` re-throw 设计保证失败时 Modal 保持打开（T-20005）
  - `src/services/apiClient.ts`（扩展）— `adminCloseRoom(roomId: string): Promise<void>`（T-20004）；`adminGetRoomDetail(roomId: string, signal?: AbortSignal): Promise<RoomDetail>`（T-20005，GET `/admin/rooms/:id`）
- 👤 **用户管理模块**（T-20006 ✅ · T-20007 ✅）- 路由 `/users`（在 `AuthGuard` 内）；涉及以下文件：
  - `src/pages/users/index.tsx` — `UsersPage` 页面入口，组合 Hook + 组件；`useCallback` 包裹 `handleReset` / `handleViewDetail` / `handleDrawerClose` 防止不必要渲染
  - `src/pages/users/useUsersPage.ts` — `useUsersPage` Hook：分页（pageSize=20）、状态过滤（normal/banned/all）、关键词搜索（手机号/用户ID/昵称）、AbortController 防竞态、`useSearchParams` 双向同步 URL Query String（刷新恢复搜索状态）
  - `src/pages/users/UsersTable.tsx` — `UsersTable` 组件：Ant Design Table、工具栏（搜索表单 + 刷新按钮）、列：ID/手机号/昵称/头像/金币余额/VIP等级/状态/注册时间/操作（查看详情）；`useMemo` 缓存 columns 数组避免重复创建
  - `src/pages/users/UserSearchForm.tsx` — `UserSearchForm` 组件：Ant Design Form inline 布局，手机号/用户ID/昵称 Input + 状态 Select + 搜索/重置 Button，按钮触发提交（非 debounce）
  - `src/pages/users/UserStatusTag.tsx` — `UserStatusTag` 组件：normal=绿色"正常"，banned=红色"封禁"
  - `src/pages/users/UserDetailDrawer.tsx` — `UserDetailDrawer` 组件：Ant Design Drawer（`destroyOnClose={true}`）展示用户详情（头像/手机号/昵称/金币余额/VIP等级/状态/注册时间）及 [封禁]/[解封] 操作按钮，点击 [封禁] 打开 `BanModal`（T-20007 · T-20008）
  - `src/pages/users/useUserDetail.ts` — `useUserDetail(userId)` Hook：监听 userId 变化，调用 `adminGetUserDetail`，含 AbortController 防竞态，返回 `{ detail, loading, error }`（T-20007）
  - `src/pages/users/BanModal.tsx` — `BanModal` 组件：Ant Design Modal 封禁对话框；表单含封禁时长 Select（1天/7天/30天/永久）、封禁原因 Select（违规言论/骚扰用户/欺诈行为/其他）、备注 TextArea（可选）；提交前 `Modal.confirm` 二次确认；`isConfirming` ref 并发防护，防止重复提交；成功后回调 `onSuccess` 触发详情刷新（T-20008）
  - `src/pages/users/UnbanModal.tsx` — `UnbanModal` 组件：与 `BanModal` 对称的解封确认弹窗；表单含解封原因 Select（必填）、备注 TextArea（可选）；提交前 `Modal.confirm` 二次确认；`isConfirming` ref 并发防护，防止重复提交；成功后回调 `onSuccess` 触发用户列表刷新（T-20010）
  - `src/pages/users/useBanUser.ts` — `useBanUser` Hook：封装 `adminBanUser` API 调用；管理 `loading` / `error` 状态；返回 `{ banUser, loading, error }`；调用方无需关心请求细节（T-20008）
  - `src/core/network/apiClient.ts`（扩展）— `adminGetUsers(params, signal?): Promise<AdminUsersData>`；新增类型 `AdminUserItem` / `AdminUsersData` / `AdminGetUsersParams`；`adminGetUserDetail(userId, signal?): Promise<AdminUserDetail>`（T-20007）；`adminBanUser(userId, params): Promise<void>`；新增类型 `AdminBanUserParams`（T-20008）
- 📋 **操作日志模块**（T-20009 ✅）- 路由 `/logs`（在 `AuthGuard` 内）；涉及以下文件：
  - `src/pages/logs/index.tsx` — `LogsPage` 页面入口，组合 Hook + 组件
  - `src/pages/logs/useLogsPage.ts` — `useLogsPage` Hook：分页（pageSize=20）、操作人ID/操作类型/时间范围过滤、AbortController 防竞态、`useSearchParams` 双向同步 URL Query String
  - `src/pages/logs/LogsTable.tsx` — `LogsTable` 组件：Ant Design Table、工具栏（刷新按钮）、只读列（日志ID/操作人ID/操作类型Tag/目标类型/目标ID/IP地址/详情/操作时间），`action` 用 `<Tag>` 渲染（ban_user=红/unban_user=绿/close_room=橙）
  - `src/pages/logs/LogSearchForm.tsx` — `LogSearchForm` 组件：Ant Design Form inline 布局，操作人 ID Input + 操作类型 Select + `DatePicker.RangePicker` 时间范围 + 搜索/重置 Button
  - `src/core/network/apiClient.ts`（扩展）— `adminGetLogs(params?, signal?): Promise<AdminLogsData>`；新增类型 `AdminLogItem` / `AdminLogsData` / `AdminGetLogsParams`

## 三、 当前能力全景与状态 (Capability Matrix)
> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发

### 核心能力
- 🟢 React + Vite + TypeScript 工程、构建脚本与 `VITE_` 环境变量约束
- 🟢 基础 HTTP 客户端封装 (`apiClient`，含 HTTP 状态检查、AbortController 15s 超时、JWT 自动附加、401 拦截器自动退出)
- 🟢 Ant Design v6 组件库集成（登录页已使用 Form / Input / Button / Checkbox / Alert / Card / Typography）
- 🟢 管理员登录页 UI（账号密码登录表单、记住账号、错误提示、i18n）← **T-20001 ✅ Done**
- 🟢 中英文国际化（i18n，i18next + react-i18next，en/zh 双语）← **T-20001 ✅ Done**
- 🟢 登录逻辑与路由守卫（useAuthStore JWT 状态管理、AuthGuard 路由守卫、401 拦截器）← **T-20002 ✅ Done**
- 🟢 Zustand 全局状态管理（useAuthStore：token/admin/isAuthenticated/checkAuth）← **T-20002 ✅ Done**
- 🟢 数据看板首页（StatCards 4张统计卡片 + ECharts 折线趋势图 + 30s 自动刷新 + AbortController 卸载取消）← **T-20003 ✅ Done**
- 🟢 房间管理页面（`/rooms` 路由；RoomsPage + useRoomsPage + RoomsTable + RoomStatusTag；分页/过滤/搜索/关闭；apiClient 新增 `adminCloseRoom`）← **T-20004 ✅ Done**
- 🟢 房间详情弹窗（`useRoomDetail` Hook + `RoomDetailModal` 组件；`destroyOnHidden={true}` 切换房间清除旧数据；`Modal.confirm` 二次确认强制关闭；`closeRoom` re-throw 保证失败时弹窗保持开启；apiClient 新增 `adminGetRoomDetail`）← **T-20005 ✅ Done**
- 🟢 用户管理页面（`/users` 路由；UsersPage + useUsersPage + UsersTable + UserSearchForm + UserStatusTag；手机号/用户ID/昵称搜索/状态筛选/分页/URL双向同步；apiClient 新增 `adminGetUsers`）← **T-20006 ✅ Done**
- 🟢 用户详情抽屉（`useUserDetail` Hook + `UserDetailDrawer` 组件；`destroyOnClose={true}` 切换用户清除旧数据；AbortController 防竞态；头像/手机号/资产信息展示；[封禁]/[解封] 按钮接入 T-20008 BanModal；apiClient 新增 `adminGetUserDetail`）← **T-20007 ✅ Done**
- 🟢 封禁对话框（`BanModal` 组件：时长/原因/备注表单 + `Modal.confirm` 二次确认 + `isConfirming` 并发防护；`useBanUser` Hook：封装 `adminBanUser` API，loading/error 状态管理；apiClient 新增 `adminBanUser`）← **T-20008 ✅ Done**
- 🟢 解封弹窗（`UnbanModal` 组件：解封原因必填 Select + 备注 TextArea + `Modal.confirm` 二次确认 + `isConfirming` 并发防护；与 `BanModal` 对称设计；成功后回调 `onSuccess` 刷新用户列表；apiClient 新增 `adminUnbanUser`）← **T-20010 ✅ Done**
- 🟢 操作日志页面（`/logs` 路由；LogsPage + useLogsPage + LogsTable + LogSearchForm；操作人ID/操作类型/时间范围筛选/分页/URL双向同步；apiClient 新增 `adminGetLogs`）← **T-20009 ✅ Done**
- 🟢 活水房间监控增强（`roomUtils.ts` 纯函数库：`getActivityStatus`/`formatDuration`/`filterByActivity`，注入 `now` 参数支持测试；`RoomActivityTag` 组件：4 种活跃等级颜色标签；`RoomsTable` 新增活跃状态列 + 持续时长列 + 活跃度筛选下拉 + 异常行高亮；`useRoomsPage` 新增 `filteredItems`/`activityFilter`/`setActivityFilter`；i18n 新增 8 个 `rooms.activity.*` 翻译键；全部为纯前端过滤，不影响 API 调用）← **T-20011 ✅ Done**
- 🟢 余额调整弹窗 + 礼物管理页（`AdjustBalanceModal`：Form.useWatch 动态禁用、负数二次确认、isConfirming 防并发、成功后 refreshKey 刷新余额；`GiftManagementPage`：tier/状态筛选 + Switch 乐观更新回滚 + 软删除；`GiftEditModal`：图片上传校验 + price=0 禁用 + 预览；`AppLayout`：Ant Design 侧栏 + RBAC 礼物菜单（super_admin/operator）；apiClient 新增 6 个 wallet/gift API；i18n 新增 60+ key）← **T-20012 ✅ Done**

### 遗留技术债 (Tech Debt)
- 当前工程脚手架仍保留 C 端时期的 telemetry mock 和 WS helper，需要在后续重构中清理。
- `src/services/` 下的 RTC/IM 适配层在 Admin Web 中不需要，应删除或标记为 deprecated。
- ~~Ant Design 尚未引入~~ — **T-20001 已引入**，登录页完整使用 antd v6 组件。
- ~~API 客户端尚未配置 Admin Server 的 baseURL 和 JWT 拦截器~~ — **T-20001 已完成** baseURL 配置与 JWT 自动附加；**T-20002 已完成** 完整 JWT 鉴权逻辑（useAuthStore + AuthGuard + 401 拦截器）。
