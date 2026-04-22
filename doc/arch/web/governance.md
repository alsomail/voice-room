<!--
[AI 读写指令与维护规约]
1. 本文件记录 Web Admin 治理日志查询模块的架构设计，对应 T-20014。
2. 路由变更、组件新增、API 扩展均需同步更新本文件。
3. 所有相对路径链接必须真实有效，禁止生成无法点击的死链接。
-->

# Web 治理日志查询模块 (T-20014)

**最后更新：** 2026-05-21  
**入口点：** `src/pages/governance/`  
**路由：** `/rooms/governance`  
**关联 Task：** T-20014 ✅ Done（依赖 T-10016、T-20007）  
**测试：** 26 个测试用例全部通过

---

## 一、架构概述

治理日志查询页面向运营/客服人员，提供对房间内踢人（kick）与禁麦（mute）操作记录的查询、筛选、导出能力。  
权限由 `RoleGuard` 控制：`super_admin` / `operator` / `cs` 可访问；`finance` 角色重定向至 403 页。

```
AppLayout (侧栏菜单 RBAC：super_admin/operator/cs 可见)
  └── /rooms/governance
        └── GovernanceLogsPage
              ├── FiltersBar（房间ID / 目标用户 / 操作者 / 类型 / 时间区间）
              ├── Tabs
              │     ├── KickLogsTab（踢出记录表格 + 分页）
              │     └── MuteLogsTab（禁麦记录表格 + 分页 + 类型筛选）
              └── RoleGuard（权限守卫，finance → 403）
```

---

## 二、核心组件

### 2.1 GovernanceLogsPage
- **文件：** `src/pages/governance/index.tsx`
- **职责：** 页面入口，持有筛选状态，协调两个 Tab 子组件。
- **默认时间窗：** 最近 7 天（`from = now - 7d`，`to = now`）
- **Tab 切换行为：** 切换 Tab 时重置筛选条件与分页至初始状态，避免跨类型数据串扰。
- **testTag：** `data-testid="governance-logs-page"`

### 2.2 KickLogsTab
- **文件：** `src/pages/governance/KickLogsTab.tsx`
- **职责：** 展示踢出记录。调用 `listKicks` API，支持分页（pageSize=20）。
- **表格列：** 时间 / 房间名 / 操作者（点击跳转管理员详情）/ 被踢用户（点击弹出 Drawer）/ 原因
- **空状态：** `<Empty description={t('governance.noKickLogs')} />`
- **testTag：** `data-testid="kick-logs-tab"` / `data-testid="kick-logs-table"`

### 2.3 MuteLogsTab
- **文件：** `src/pages/governance/MuteLogsTab.tsx`
- **职责：** 展示禁麦/禁言记录。调用 `listMutes` API，支持分页（pageSize=20）。
- **表格列：** 时间 / 房间名 / 操作者 / 被禁用户（点击弹出 Drawer）/ 类型（mic/chat）/ 时长（秒→分钟格式化）/ 原因
- **mute 专属筛选：** 类型下拉 `[全部 / 禁麦 / 禁言]`（`mic | chat | all`）
- **空状态：** `<Empty description={t('governance.noMuteLogs')} />`
- **testTag：** `data-testid="mute-logs-tab"` / `data-testid="mute-logs-table"`

### 2.4 FiltersBar
- **文件：** `src/pages/governance/FiltersBar.tsx`
- **职责：** 通用筛选栏，被 GovernanceLogsPage 持有并向两个 Tab 传递筛选参数。
- **筛选字段：**
  - `room_id` — Input，房间 ID
  - `target_user_id` — Input，被操作用户 ID
  - `operator_user_id` — Input，操作者用户 ID
  - `time_range` — `DatePicker.RangePicker`，时间区间（≤90 天）
- **行为：** 点击"搜索"触发 `onSearch` 回调；点击"重置"清空所有字段并恢复默认 7 天时间窗。
- **testTag：** `data-testid="governance-filters-bar"` / `data-testid="filter-room-id"` / `data-testid="filter-target-user"` / `data-testid="filter-operator"` / `data-testid="filter-time-range"` / `data-testid="btn-search"` / `data-testid="btn-reset"`

### 2.5 RoleGuard
- **文件：** `src/components/RoleGuard.tsx`（复用/新增）
- **职责：** 包裹治理日志路由，按角色放行或重定向。
- **放行角色：** `super_admin` / `operator` / `cs`
- **拒绝角色：** `finance` → 重定向 `/403`
- **实现：** 读取 `useAuthStore().admin.role`，不满足条件时 `<Navigate to="/403" replace />`
- **testTag：** `data-testid="role-guard"`

---

## 三、API 层

### 3.1 listKicks
```typescript
// src/core/network/apiClient.ts（或 src/services/api/governance.ts re-export）
export async function listKicks(params: GovernanceQueryParams, signal?: AbortSignal): Promise<KickLogsData>

// 对应后端：GET /api/v1/admin/governance/logs?type=kick&...
// GovernanceQueryParams:
// {
//   room_id?: string;
//   target_user_id?: string;
//   operator_user_id?: string;
//   from?: string;     // ISO 8601
//   to?: string;       // ISO 8601
//   page?: number;
//   limit?: number;    // max 100
// }
```

### 3.2 listMutes
```typescript
export async function listMutes(params: GovernanceQueryParams & { mute_type?: 'mic' | 'chat' }, signal?: AbortSignal): Promise<MuteLogsData>

// 对应后端：GET /api/v1/admin/governance/logs?type=mute&...
```

### 3.3 数据类型
```typescript
interface KickLogItem {
  id: string;
  room_id: string;
  room_name: string;
  target_user_id: string;
  target_nickname: string;
  operator_user_id: string;
  operator_nickname: string;
  reason: string;
  created_at: string;  // ISO 8601
}

interface MuteLogItem extends KickLogItem {
  mute_type: 'mic' | 'chat';
  duration_sec: number;  // 0 = 解除
}

interface KickLogsData { items: KickLogItem[]; total: number; page: number; limit: number; }
interface MuteLogsData { items: MuteLogItem[]; total: number; page: number; limit: number; }
```

### 3.4 re-export 路径
```typescript
// src/services/api/governance.ts
export { listKicks, listMutes } from '@/core/network/apiClient';
```

---

## 四、权限矩阵

| 角色 | 菜单可见 | 页面可访问 | CSV 导出 |
|------|----------|------------|----------|
| super_admin | ✅ | ✅ | ✅ |
| operator | ✅ | ✅ | ✅ |
| cs | ✅ | ✅ | ✅ |
| finance | ❌ | ❌（→ 403） | ❌ |

---

## 五、国际化（i18n）

两端（en / zh）各新增 32 个 key，命名空间为 `governance.*`：

| Key | EN | ZH |
|-----|----|----|
| `governance.title` | Governance Logs | 治理日志 |
| `governance.kickTab` | Kick Logs | 踢出记录 |
| `governance.muteTab` | Mute Logs | 禁麦记录 |
| `governance.filterRoomId` | Room ID | 房间 ID |
| `governance.filterTargetUser` | Target User | 目标用户 |
| `governance.filterOperator` | Operator | 操作者 |
| `governance.filterTimeRange` | Time Range | 时间范围 |
| `governance.defaultWindow` | Last 7 days | 最近 7 天 |
| `governance.colTime` | Time | 时间 |
| `governance.colRoom` | Room | 房间 |
| `governance.colOperator` | Operator | 操作者 |
| `governance.colTarget` | Target User | 目标用户 |
| `governance.colReason` | Reason | 原因 |
| `governance.colMuteType` | Mute Type | 禁用类型 |
| `governance.colDuration` | Duration | 时长 |
| `governance.muteTypeMic` | Mic Mute | 禁麦 |
| `governance.muteTypeChat` | Chat Mute | 禁言 |
| `governance.muteTypeAll` | All | 全部 |
| `governance.noKickLogs` | No kick logs | 暂无踢出记录 |
| `governance.noMuteLogs` | No mute logs | 暂无禁麦记录 |
| `governance.btnSearch` | Search | 搜索 |
| `governance.btnReset` | Reset | 重置 |
| `governance.btnExport` | Export CSV | 导出 CSV |
| `governance.exportFilename` | governance_logs_{ts}.csv | governance_logs_{ts}.csv |
| `governance.durationSec` | {n}s | {n}秒 |
| `governance.durationMin` | {n}min | {n}分钟 |
| `governance.durationHour` | {n}h | {n}小时 |
| `governance.durationUnlimited` | Unlimited | 永久 |
| `governance.403Title` | Access Denied | 无权限访问 |
| `governance.403Desc` | Your role cannot access this page. | 您的角色无法访问此页面。 |
| `governance.tabResetTip` | Filters reset on tab change | 切换 Tab 时筛选已重置 |
| `governance.loadError` | Failed to load logs | 日志加载失败 |

---

## 六、testTag 清单

| testTag | 所属组件 | 说明 |
|---------|----------|------|
| `governance-logs-page` | GovernanceLogsPage | 页面根节点 |
| `governance-filters-bar` | FiltersBar | 筛选栏容器 |
| `filter-room-id` | FiltersBar | 房间 ID Input |
| `filter-target-user` | FiltersBar | 目标用户 Input |
| `filter-operator` | FiltersBar | 操作者 Input |
| `filter-time-range` | FiltersBar | 时间区间选择器 |
| `btn-search` | FiltersBar | 搜索按钮 |
| `btn-reset` | FiltersBar | 重置按钮 |
| `btn-export-csv` | GovernanceLogsPage | 导出 CSV 按钮 |
| `kick-logs-tab` | KickLogsTab | Tab 面板 |
| `kick-logs-table` | KickLogsTab | 踢出记录表格 |
| `mute-logs-tab` | MuteLogsTab | Tab 面板 |
| `mute-logs-table` | MuteLogsTab | 禁麦记录表格 |
| `mute-type-filter` | MuteLogsTab | 禁麦类型下拉 |
| `role-guard` | RoleGuard | 权限守卫容器 |
| `governance-403-page` | RoleGuard/403Page | 403 无权限页 |
| `operator-link-{userId}` | KickLogsTab/MuteLogsTab | 操作者跳转链接（动态 userId） |
| `target-user-link-{userId}` | KickLogsTab/MuteLogsTab | 目标用户点击弹 Drawer（动态 userId） |

---

## 七、数据流

```
FiltersBar (筛选状态 owned by GovernanceLogsPage)
    │
    ▼
GovernanceLogsPage.useGovernanceLogs(params)
    ├── listKicks(params)  ──► GET /admin/governance/logs?type=kick&...
    └── listMutes(params)  ──► GET /admin/governance/logs?type=mute&...
         │
         ▼
    KickLogsTab / MuteLogsTab (纯展示，接收 items/total/loading)
         │
         ▼
    目标用户点击 ──► UserDetailDrawer（复用 T-20007 组件）
    操作者点击   ──► navigate('/users?id={operator_user_id}')
```

---

## 八、CSV 导出

- 按钮位置：筛选栏右侧 `data-testid="btn-export-csv"`
- 导出范围：当前筛选条件下全量数据（`limit=100`，最多 100 条/次）
- 文件名格式：`governance_logs_{YYYYMMDD_HHmmss}.csv`
- 编码：UTF-8 BOM（`\uFEFF`）确保 Excel 中文兼容
- 导出内容：与当前激活 Tab（Kick / Mute）一致

---

## 九、相关文档

- [Web 架构总索引](./index.md)
- [用户管理模块](./user-management.md)（UserDetailDrawer 复用）
- [Admin Server 治理日志查询 API](../adminServer/governance.md)（T-10016）
- [E-10 Epic 方向总纲](../../product/phase1_room_governance.md)
- [T-20014 TDS](../../tds/web/T-20014.md)
