# Web 用户管理模块架构

**最后更新：** 2026-04-28  
**涉及 Tasks：** T-20006 (用户列表) · T-20007 (用户详情) · T-20008 (封禁) · T-20010 (解封) · T-20012 (余额调整) · T-20013 (行为流 Tab)

---

## 概述

用户管理模块提供 B 端管理员对 C 端用户的全生命周期管理，包含用户列表查询、详情查看、状态管理（封禁/解封）、资产调整（手动充值）。

**路由**：`/users`（AuthGuard 内）

---

## 模块架构

```
UsersPage
├── useUsersPage Hook
│   ├── 分页管理 (pageSize=20)
│   ├── 状态筛选 (normal/banned/all)
│   ├── 关键词搜索 (phone/userId/nickname，debounce 300ms)
│   ├── URL Query String 双向同步
│   └── AbortController 防竞态
├── UsersTable 组件
│   ├── Ant Design Table：ID/phone/nickname/avatar/balance/VIP/status/regTime/actions
│   └── 点击"查看详情"打开 UserDetailDrawer
└── UserDetailDrawer 组件
    ├── 用户基本信息展示（头像/phone/nickname/balance/VIP/status/regTime）
    ├── Tabs（T-20013 新增）
    │   ├── 基本信息 Tab（原有内容）
    │   │   ├── AdjustBalanceModal（T-20012）
    │   │   ├── BanModal（T-20008）
    │   │   └── UnbanModal（T-20010）
    │   └── **行为流 Tab**（**T-20013 新增**）
    │       ├── EventStreamTab 组件
    │       │   ├── 时间筛选：Radio.Group [最近 1h / 24h / 7d / 30d / 自定义]
    │       │   ├── 自定义时间：DatePicker.RangePicker（限 30 天）
    │       │   ├── 事件名过滤：Select mode=multiple
    │       │   ├── 时间线列表：倒序展示
    │       │   │   └── EventTimelineItem（单条事件卡片）
    │       │   │       ├── event_name (Tag 高亮色按类别)
    │       │   │       ├── server_ts 格式化
    │       │   │       ├── 设备/app_version/network_type（小字）
    │       │   │       └── properties JSON 折叠展开（`<mark>` 关键字高亮+XSS防护）
    │       │   ├── 分页：Ant Design Pagination，20/页
    │       │   └── 导出 CSV 按钮
    │       │       ├── limit=100，最多导出 1000 条
    │       │       ├── 文件名：user_{id}_events_{ts}.csv
    │       │       └── 独立 AbortController 防竞态
    │       └── 常量字典：EVENT_CATEGORIES（auth/gift/wallet/room 等）
```

---

## 关键组件详解

### UsersPage & useUsersPage Hook

**文件**：`src/pages/users/index.tsx` + `src/pages/users/useUsersPage.ts`

**职责**：
- 分页逻辑（pageSize=20 固定，支持 pageNum 变化）
- 状态筛选：normal/banned/all（对应 `status` Query 参数）
- 关键词搜索：手机号/用户ID/昵称，300ms debounce
- URL Query String 双向同步（`useSearchParams`）
- AbortController 防竞态

**API 调用**：
```ts
adminGetUsers({
  page: pageNum,
  size: 20,
  keyword?: search,        // 同时搜索 phone/userId/nickname
  status?: statusFilter,   // 'normal' / 'banned' / 'all'
}, signal)
```

**Hook 返回值**：
```ts
{
  users: AdminUserItem[];
  loading: boolean;
  error: Error | null;
  pageNum: number;
  setPageNum: (n: number) => void;
  statusFilter: 'normal' | 'banned' | 'all';
  setStatusFilter: (s) => void;
  search: string;
  setSearch: (q) => void;
  total: number;
}
```

### UserDetailDrawer 组件

**文件**：`src/pages/users/UserDetailDrawer.tsx`  
**涉及 Task**：T-20007 (基础) · T-20008 (BanModal) · T-20010 (UnbanModal) · T-20012 (AdjustBalanceModal)

**Props**：
```ts
interface UserDetailDrawerProps {
  userId?: string;           // undefined 时 Drawer 不展示
  open: boolean;
  onClose: () => void;
}
```

**组件结构**：
- **Ant Design Drawer**（`destroyOnClose={true}` 切换用户清除旧数据）
- **useUserDetail Hook**：监听 userId，发起 API 请求，含 AbortController 防竞态
- **用户信息卡片**：头像（Avatar）、phone、nickname、balance、VIP等级、status、regTime
- **操作按钮行**：
  - "调整余额"按钮（**T-20012 新增**）：RBAC 控制（`super_admin/operator/finance`），点击打开 `AdjustBalanceModal`
  - "[封禁]" 按钮（T-20008）：点击打开 `BanModal`，禁用条件为 `status === 'banned'`
  - "[解封]" 按钮（T-20010）：点击打开 `UnbanModal`，禁用条件为 `status === 'normal'`

**余额刷新机制**（T-20012）：
```ts
const [refreshKey, setRefreshKey] = useState(0);
const { detail, loading, error } = useUserDetail(userId, refreshKey);

const handleAdjustSuccess = () => {
  // 触发 useUserDetail 重新拉取最新余额
  setRefreshKey(prev => prev + 1);
  message.success(t('wallet.adjust.success'));
};
```

### AdjustBalanceModal 组件（T-20012）

**文件**：`src/features/user/AdjustBalanceModal.tsx`

**Props**：
```ts
interface AdjustBalanceModalProps {
  userId: string;
  currentBalance: number;
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;  // 调整成功后回调
}
```

**Form 字段**：
- **amount**：InputNumber
  - 可输入负数（扣减）
  - 必填且非零（`amount !== 0`）
  - 绝对值 ≤ 10,000,000
  - 禁用条件：`amount === 0 || !reason`
- **reason**：Input.TextArea
  - 必填（2-200 字符）
  - 禁用条件：同上

**验证与交互**：
1. **实时禁用**：使用 `Form.useWatch` 监听 amount/reason，动态禁用"确定"按钮
2. **负数二次确认**：`amount < 0` 时，Modal 头部显示红色警示"扣减操作"，点击"确定"后弹 `Modal.confirm`
   ```tsx
   <Modal.confirm({
     title: '扣减操作确认',
     content: `此操作将扣减用户 ${Math.abs(amount)} 钻石，确定继续？`,
     okButtonProps: { danger: true },
     onOk: () => { /* 提交 API */ }
   })>
   ```
3. **提交防并发**：`isConfirming` ref，防止重复点击

**API 调用**：
```ts
await adminAdjustBalance(userId, {
  amount: number;
  reason: string;
});
```

**成功流程**：
- API 返回新余额后，调用 `onSuccess()` → 触发 UserDetailDrawer 的 `refreshKey` 刷新
- 显示成功 toast
- 关闭 Modal

**错误处理**：
- 400/403 显示 `errorMessage`，**保留 Modal 打开**（用户可修改后重试）
- 其他错误显示通用错误信息

### BanModal 组件（T-20008）

**文件**：`src/pages/users/BanModal.tsx`

**Props**：
```ts
interface BanModalProps {
  userId: string;
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
}
```

**Form 字段**：
- **duration**：Select（必选）
  - 选项：1天 / 7天 / 30天 / 永久
  - 对应值：'1d' / '7d' / '30d' / 'permanent'
- **reason**：Select（必选）
  - 选项：违规言论 / 骚扰用户 / 欺诈行为 / 其他
- **remark**：TextArea（可选）
  - 最多 500 字符

**验证与交互**：
1. 二次确认：`Modal.confirm` + 红色按钮
2. 防并发：`isConfirming` ref

**API 调用**：
```ts
await adminBanUser(userId, {
  duration: '1d' | '7d' | '30d' | 'permanent';
  reason: string;
  remark?: string;
});
```

### UnbanModal 组件（T-20010）

**文件**：`src/pages/users/UnbanModal.tsx`

**Props**：
```ts
interface UnbanModalProps {
  userId: string;
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
}
```

**Form 字段**：
- **reason**：Select（**必选**）
  - 选项：申诉通过 / 改过自新 / 其他
- **remark**：TextArea（可选）
  - 最多 500 字符

**验证与交互**：
- 二次确认 + `isConfirming` 防并发

**API 调用**：
```ts
await adminUnbanUser(userId, {
  reason: string;
  remark?: string;
});
```

### EventStreamTab 组件（T-20013）

**文件**：
- `src/features/user/EventStreamTab.tsx` — Tab 主体组件 + 时间筛选 + 事件多选 + 分页 + CSV 导出
- `src/features/user/components/EventTimelineItem.tsx` — 单条事件卡片组件（包含关键字高亮）
- `src/features/user/events.dict.ts` — 事件字典常量与颜色映射
- `src/lib/csv.ts` — CSV 生成/下载工具
- `src/pages/users/UserDetailDrawer.tsx` — 修改为 Tabs 容器

**Props**（EventStreamTab）：
```ts
interface EventStreamTabProps {
  userId: string;
}
```

**主要功能**：

1. **时间筛选**：
   - Radio.Group 选项：[最近 1h | 24h | 7d | 30d | 自定义]
   - 自定义模式：DatePicker.RangePicker（限 30 天）
   - 验证函数：`validateCustomRange()` 纯函数（≤30 天返回 true，否则 false）

2. **事件名过滤**：
   - Select mode=multiple，选项来自 `EVENT_CATEGORIES` 常量
   - 无需后端枚举，前端硬编码

3. **时间线列表**：
   - 倒序展示（newest first）
   - 分页：Ant Design Pagination，pageSize=20
   - 每项展示：
     - event_name（Tag，颜色按事件类别）
     - server_ts 格式化（"2026-04-28 14:30:45" 格式）
     - 设备信息小字（app_version / network_type / os_version）
     - properties JSON 折叠展开（EventTimelineItem 子组件）

4. **关键字高亮**（XSS 防护）：
   - `escapeHtml()` 纯函数：先对文本 HTML 转义（`<`→`&lt;` 等）
   - `highlightText()` 函数：先转义文本再转义关键字后替换为 `<mark>` 包裹
   - 渲染分支控制：有 highlight 才走 `dangerouslySetInnerHTML`，无 highlight 走安全的 React 文本节点
   - 测试覆盖：properties 含 `<script>` 时不产生 XSS

5. **CSV 导出**：
   - **limit=100**：每次 API 调用最多拉 100 条，导出最多 1000 条需最多 10 次请求
   - 文件名：`user_{userId}_events_{timestamp}.csv`，timestamp 格式 `YYYYMMDD_HHmmss`
   - 独立 `exportAbortRef`：管理导出请求的 AbortController
   - 流程：
     1. 点击导出按钮 → `setExporting(true)`
     2. 创建新 `AbortController`，取消前一次导出请求
     3. 循环调用 `listUserEvents` 分页拉取（最多 10 次）
     4. 合并所有事件数据，使用 `Papa.unparse` 或自实现轻量 CSV 序列化
     5. 生成 Blob，触发浏览器下载
     6. `AbortError` 静默处理（用户主动取消）；其他错误显示 toast

6. **AbortSignal 防竞态**：
   - 主查询（时间/事件过滤）：使用 `useEffect` 的 AbortController
   - CSV 导出：独立的 `exportAbortRef.current`
   - 组件卸载时：cleanup 函数中 `signal.abort()` + `exportAbortRef.current?.abort()`

**API 调用**：
```ts
// 主查询
listUserEvents(userId, {
  event_name?: string;  // 多选用 ',' 分隔
  from?: string;        // ISO 8601 或 Unix ms
  to?: string;
  page?: number;
  limit?: number;       // 默认 20，CSV 导出时 100
});
```

**EventTimelineItem Props**：
```ts
interface EventTimelineItemProps {
  event: {
    id: string;
    event_name: string;
    server_ts: number;      // Unix ms
    app_version?: string;
    network_type?: string;
    os_version?: string;
    properties?: Record<string, any>;
  };
  highlight?: string;  // 关键字高亮（暂未使用，XSS 修复已提前防御）
}
```

**EventTimelineItem 内部**：
- JSON.stringify(properties) 并转义 HTML
- 若有 highlight，使用 `highlightText()` 包裹关键字为 `<mark>` 标签
- 折叠/展开按钮：`props-toggle-{eventId}`
- `dangerouslySetInnerHTML` 仅在有 highlight 且已转义后使用

**测试覆盖**：
- E13-01：Drawer 新 Tab 可见（`tab-event-stream` testid）
- E13-02：默认加载最近 24h
- E13-03：event_name 多选过滤生效
- E13-04：自定义时间 >30 天显示错误提示
- E13-05：无数据显示占位（`events-empty`）
- E13-06：CSV 文件名含 user_id 和时间戳
- E13-07：properties JSON 折叠/展开（`props-toggle-{id}` / `props-content-{id}`）
- E13-08：i18n 切换（ar/en）文案正确
- **XSS 防护**：properties 含 HTML 特殊字符时被正确转义
- **AbortController**：组件卸载后 CSV 导出请求被取消

---

## 数据流

```
UsersPage (useUsersPage Hook)
    ↓
    ├─ API: adminGetUsers(params, signal)
    └─ 返回: users[], total, loading, error
        ↓
    UsersTable (展示列表)
        ↓
        └─ 点击"查看详情" → setSelectedUserId()
            ↓
        UserDetailDrawer (open=true)
            ├─ Tabs 容器
            │
            ├─ [基本信息 Tab]
            │   ├─ API: useUserDetail(userId)
            │   │   └─ 返回: detail, loading, error
            │   │
            │   ├─ "调整余额" 按钮点击 → AdjustBalanceModal open
            │   │   ├─ API: adminAdjustBalance(userId, { amount, reason })
            │   │   └─ 成功后 → onSuccess() → setRefreshKey() → useUserDetail 重新拉取
            │   │
            │   ├─ "[封禁]" 按钮点击 → BanModal open
            │   │   ├─ API: adminBanUser(userId, { duration, reason, remark })
            │   │   └─ 成功后 → onSuccess() → useUsersPage 列表刷新
            │   │
            │   └─ "[解封]" 按钮点击 → UnbanModal open
            │       ├─ API: adminUnbanUser(userId, { reason, remark })
            │       └─ 成功后 → onSuccess() → useUsersPage 列表刷新
            │
            └─ [**行为流 Tab**]（**T-20013**）
                ├─ 时间筛选：Radio.Group 选择 [1h|24h|7d|30d|custom]
                ├─ 自定义时间：DatePicker.RangePicker(from, to, ≤30天)
                ├─ 事件名多选：Select mode=multiple (from EVENT_CATEGORIES)
                ├─ 分页：Pagination pageNum/pageSize=20 变化
                │
                ├─ API: listUserEvents(userId, {
                │         from, to, event_name?, page, limit: 20
                │       }, signal)
                │   └─ 返回：events[], total, loading, error
                │       ↓
                │   EventTimelineItem[] (倒序展示)
                │       ├─ event_name (Tag)
                │       ├─ server_ts (格式化)
                │       ├─ 设备信息
                │       └─ properties (JSON 折叠, 关键字高亮+XSS防护)
                │
                └─ 导出 CSV 按钮 → handleExportCsv()
                    ├─ 创建独立 AbortController (exportAbortRef)
                    ├─ 循环 API: listUserEvents(..., {limit: 100}) 最多 10 次
                    ├─ 合并事件数据，Papa.unparse() 生成 CSV
                    ├─ 生成 Blob，触发浏览器下载
                    └─ AbortError 静默处理；其他错误 toast
```

---

## API 客户端函数

**文件**：`src/core/network/apiClient.ts`

| 函数 | HTTP 方法 | 端点 | 说明 | Task |
|------|---------|------|------|------|
| `adminGetUsers(params, signal?)` | GET | `/admin/users` | 用户列表查询（分页/筛选/搜索） | T-20006 |
| `adminGetUserDetail(userId, signal?)` | GET | `/admin/users/:id` | 用户详情 | T-20007 |
| `adminBanUser(userId, params)` | POST | `/admin/users/:id/ban` | 封禁用户 | T-20008 |
| `adminUnbanUser(userId, params)` | POST | `/admin/users/:id/unban` | 解封用户 | T-20010 |
| `adminAdjustBalance(userId, params)` | POST | `/admin/users/:id/wallet/adjust` | 手动调整余额 | T-20012 |
| **`listUserEvents(userId, params, signal?)`** | **GET** | **`/admin/users/:id/events`** | **用户行为流事件列表（时间/事件名过滤、分页）** | **T-20013** |

---

## 权限控制（RBAC）

### 操作权限矩阵

| 操作 | super_admin | operator | finance | cs |
|-----|-----------|----------|---------|-----|
| 查看用户列表 | ✅ | ✅ | ✅ | ✅ |
| 查看用户详情 | ✅ | ✅ | ✅ | ✅ |
| 封禁用户 | ✅ | ✅ | ❌ | ❌ |
| 解封用户 | ✅ | ✅ | ❌ | ❌ |
| **调整余额** | ✅ | ✅ | ✅ | ❌ |

**实现**：`useAuthStore` + 前端条件渲染（后端通过 API 403 二重防护）

---

## 国际化 (i18n)

**文件**：`src/i18n/locales/en.ts` / `zh.ts`

**调整余额相关 key**（T-20012 新增）：
```json
{
  "wallet.adjust.title": "Adjust Balance",
  "wallet.adjust.amount": "Amount (positive=add, negative=deduct)",
  "wallet.adjust.reason": "Reason (required)",
  "wallet.adjust.confirm_deduct": "This will deduct {amount} diamonds from the user. Continue?",
  "wallet.adjust.success": "Balance adjusted successfully",
  "wallet.adjust.adjustBalance": "Adjust Balance"  // 按钮文案
}
```

**封禁相关 key**（T-20008）：
```json
{
  "user.ban.title": "Ban User",
  "user.ban.duration": "Ban Duration",
  "user.ban.reason": "Reason",
  // ...
}
```

**解封相关 key**（T-20010）：
```json
{
  "user.unban.title": "Unban User",
  "user.unban.reason": "Reason for Unban",
  // ...
}
```

**行为流 Tab 相关 key**（**T-20013 新增**）：
```json
{
  "events.title": "Event Stream",
  "events.timeRange": "Time Range",
  "events.last1h": "Last 1 hour",
  "events.last24h": "Last 24 hours",
  "events.last7d": "Last 7 days",
  "events.last30d": "Last 30 days",
  "events.custom": "Custom Range",
  "events.eventName": "Event Name",
  "events.startDate": "Start Date",
  "events.endDate": "End Date",
  "events.invalidRange": "Range must be ≤ 30 days",
  "events.noData": "No events found",
  "events.loading": "Loading events...",
  "events.error": "Failed to load events",
  "events.exportCsv": "Export CSV",
  "events.exporting": "Exporting...",
  "events.exportSuccess": "CSV exported successfully",
  "events.exportFailed": "Export failed",
  "events.maxLimitReached": "Only exported the first 1000 records",
  "events.timestamp": "Timestamp",
  "events.properties": "Properties",
  "events.deviceInfo": "Device Info"
}
```

---

## 测试覆盖

**文件**：
- `src/pages/users/__tests__/UserDetailDrawer.test.tsx`（覆盖 D11-D14 RBAC 测试）
- `src/features/user/__tests__/AdjustBalanceModal.test.tsx`（覆盖 A01-A12，含 20 个单元测试）
- `src/pages/users/__tests__/BanModal.test.tsx`
- `src/pages/users/__tests__/UnbanModal.test.tsx`

**关键测试场景**：
- W12-01：调整余额按钮可见性（仅 super_admin/operator/finance 可见）
- W12-02~05：金额/原因校验、二次确认、刷新机制
- W12-06：错误处理（400/403 保留 Modal）
- W12-12：`isConfirming` 防并发

---

## 相关文档

- [T-20006 用户列表技术设计](../../tds/web/T-20006.md)
- [T-20007 用户详情技术设计](../../tds/web/T-20007.md)
- [T-20008 封禁对话框技术设计](../../tds/web/T-20008.md)
- [T-20010 解封弹窗技术设计](../../tds/web/T-20010.md)
- [T-20012 余额调整技术设计](../../tds/web/T-20012.md)
- [T-20013 行为流 Tab 技术设计](../../tds/web/T-20013.md)
- [Web 架构总索引](./index.md)
- [礼物管理模块](./gift-management.md)
