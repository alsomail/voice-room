# Web 用户管理模块架构

**最后更新：** 2025-07-18  
**涉及 Tasks：** T-20006 (用户列表) · T-20007 (用户详情) · T-20008 (封禁) · T-20010 (解封) · T-20012 (余额调整)

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
    ├── AdjustBalanceModal（T-20012 新增）
    │   ├── Form: amount (InputNumber，可负) + reason (TextArea)
    │   ├── 负数二次确认 (Modal.confirm)
    │   ├── RBAC 控制：仅 super_admin/operator/finance 可见按钮
    │   └── 成功后 refreshKey 触发 balance 刷新
    ├── BanModal（T-20008）
    │   ├── Form: duration (Select) + reason (Select) + remark (TextArea)
    │   ├── Modal.confirm 二次确认
    │   └── isConfirming 防并发
    └── UnbanModal（T-20010）
        ├── Form: reason (Select，必填) + remark (TextArea)
        ├── Modal.confirm 二次确认
        └── isConfirming 防并发
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
            ├─ API: useUserDetail(userId)
            │   └─ 返回: detail, loading, error
            │
            ├─ "调整余额" 按钮点击 → AdjustBalanceModal open
            │   ├─ API: adminAdjustBalance(userId, { amount, reason })
            │   └─ 成功后 → onSuccess() → setRefreshKey() → useUserDetail 重新拉取
            │
            ├─ "[封禁]" 按钮点击 → BanModal open
            │   ├─ API: adminBanUser(userId, { duration, reason, remark })
            │   └─ 成功后 → onSuccess() → useUsersPage 列表刷新
            │
            └─ "[解封]" 按钮点击 → UnbanModal open
                ├─ API: adminUnbanUser(userId, { reason, remark })
                └─ 成功后 → onSuccess() → useUsersPage 列表刷新
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

- [T-20012 技术设计文档](../../tds/web/T-20012.md)
- [Web 架构总索引](./index.md)
- [礼物管理模块](./gift-management.md)
