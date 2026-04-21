# Web 礼物管理模块架构

**最后更新：** 2025-07-18  
**涉及 Tasks：** T-10014 (Admin 礼物 CRUD API) · T-20012 (Web 礼物管理页)

---

## 概述

礼物管理模块提供 B 端管理员对虚拟礼物的全生命周期管理，包含列表查询、新增/编辑礼物、上下架管理、图片上传、软删除。是 E-07 虚拟礼物与钱包闭环 MVP 的管理端实现。

**路由**：`/gifts`（AuthGuard → AppLayout）  
**权限**：super_admin / operator 可见菜单  
**相关后端 API**：T-10014 提供 CRUD 接口

---

## 模块架构

```
AppLayout（新增 T-20012）
└── 侧栏菜单
    ├── RBAC 控制：super_admin / operator 可见"礼物管理"菜单项
    └── GiftManagementPage 路由

GiftManagementPage (/gifts)
├── useGiftsPage Hook
│   ├── 分页管理 (pageSize=20)
│   ├── Tier 筛选 (1/2/3/4/all)
│   ├── 状态筛选 (all/active/inactive/deleted)
│   ├── AbortController 防竞态
│   └── 客户端过滤 (inactive 状态客户端侧过滤)
├── GiftsTable 组件
│   ├── 列：icon/code/name_cn/name_ar/price/tier/is_active (Switch)/actions
│   ├── Tier 下拉筛选
│   ├── 状态下拉筛选
│   ├── "新增礼物"按钮
│   └── 操作列 (编辑/删除)
└── GiftEditModal 组件
    ├── 新增模式
    │   ├── Form: code/name_cn/name_ar/price/tier/icon_url/animation_url/description
    │   ├── 图片上传区 + 实时预览
    │   └── 提交：POST /admin/gifts
    └── 编辑模式
        ├── Form: 同新增
        ├── 图片可重新上传
        └── 提交：PUT /admin/gifts/:id
```

---

## 关键组件详解

### AppLayout 组件更新（T-20012 新增）

**文件**：`src/app/AppLayout.tsx`

**修改内容**：
- 侧栏菜单新增"礼物管理"菜单项
- **RBAC 控制**：仅 `super_admin` 和 `operator` 角色可见

```tsx
const GIFT_MENU_ROLES = ['super_admin', 'operator'];
const canViewGiftMenu = GIFT_MENU_ROLES.includes(role);

{canViewGiftMenu && (
  <Menu.Item key="gifts" icon={<GiftOutlined />}>
    <Link to="/gifts">{t('gift.mgmt.title')}</Link>
  </Menu.Item>
)}
```

**路由关键点**：
- `/gifts` 在 `AuthGuard` 内，使用 `AppLayout` 布局
- 菜单项默认选中状态与路由对应

---

### GiftManagementPage 组件

**文件**：`src/features/gift/GiftManagementPage.tsx`

**职责**：
- 礼物列表展示（Table）
- Tier 筛选
- 状态筛选（all/active/inactive/deleted）
- "新增礼物"按钮
- Switch 上下架切换（乐观更新 + 失败回滚）

**核心逻辑 - 状态筛选的客户端过滤**：

```ts
// 后端 API 仅支持 include_inactive boolean 参数
// 前端需实现状态过滤逻辑

const fetchGifts = async (params: GiftsFetchParams, clientFilter?: GiftStatusFilter) => {
  try {
    // 根据 clientFilter 决定 include_inactive 参数
    const apiParams = {
      ...params,
      include_inactive: clientFilter !== 'active',  // active 时仅返回上架；其他情况返回全量
    };

    const result = await adminListGifts(apiParams, signal);
    
    // 客户端过滤
    const filtered = clientFilter === 'inactive'
      ? result.items.filter(g => !g.is_active)  // 仅显示下架
      : clientFilter === 'deleted'
      ? result.items.filter(g => g.deleted_at !== null)  // 仅显示已删除
      : result.items;  // all 显示全量

    setGifts(filtered);
    setTotal(filtered.length);
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return;
    setError(error);
  }
};

// useEffect 中调用
const [controller] = useState(() => new AbortController());
useEffect(() => {
  return () => controller.abort();  // cleanup 取消请求
}, []);

useEffect(() => {
  void fetchGifts(
    { page, size: 20, ...(tierFilter !== undefined ? { tier: tierFilter } : {}) },
    statusFilter
  );
}, [page, statusFilter, tierFilter]);
```

**Switch 上下架逻辑**：

```ts
const handleToggleActive = async (giftId: string, newActive: boolean) => {
  // 1. 乐观更新 (Optimistic Update)
  const oldGifts = gifts;
  setGifts(gifts =>
    gifts.map(g => g.id === giftId ? { ...g, is_active: newActive } : g)
  );

  try {
    // 2. 发送 API 请求
    await adminUpdateGift(giftId, { is_active: newActive });
    message.success(newActive ? '已上架' : '已下架');
  } catch (error) {
    // 3. 失败回滚
    setGifts(oldGifts);
    message.error('操作失败，已回滚');
  }
};
```

**编辑成功刷新**：

```ts
const handleEditSuccess = () => {
  // 与 useEffect 保持一致的参数逻辑
  void fetchGifts(
    {
      page,
      size: 20,
      include_inactive: statusFilter !== 'active',  // ← 正确
      ...(tierFilter !== undefined ? { tier: tierFilter } : {}),
    },
    statusFilter
  );
};
```

### GiftEditModal 组件

**文件**：`src/features/gift/GiftEditModal.tsx`

**Props**：
```ts
interface GiftEditModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
  editingId?: string;  // undefined 时为新增，有值时为编辑
  editingData?: AdminGiftItem;
}
```

**Form 字段**：
- **code**：Input（必填，唯一）
  - 礼物代码，编辑时禁用（不可修改）
  - 长度：2-20 字符
- **name_cn**：Input（必填）
  - 中文名称，长度：2-50 字符
- **name_ar**：Input（必填）
  - 阿拉伯语名称，长度：2-50 字符
- **price**：InputNumber（必填）
  - 价格，正整数，范围：1-10,000,000
  - **禁用条件**：`price === 0` 时提交按钮禁用（验收标准 W12-09）
- **tier**：Select（必填）
  - 等级：1/2/3/4
- **icon_url**：Upload 组件（必填）
  - 图片上传，调用 `/api/v1/admin/gifts/upload?kind=icon`
  - 文件校验（MIME + 大小）
  - **实时预览**：上传成功后立即在下方展示图标预览
  - **错误提示**：非图片文件显示错误提示（验收标准 W12-10）
  - 返回值填充 `form.setFieldValue('icon_url', url)`
- **animation_url**：Upload 组件（可选）
  - Lottie JSON 动效 URL
  - 可选上传
- **description**：Input.TextArea（可选）
  - 描述，长度：0-500 字符

**验证与交互**：
1. **实时禁用**：使用 `Form.useWatch` 监听 `price`，当 `price === 0` 时禁用提交按钮
2. **图片上传校验**：
   ```tsx
   beforeUpload={(file: File) => {
     const isImage = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'].includes(file.type);
     const isLt5MB = file.size / 1024 / 1024 < 5;
     
     if (!isImage) {
       message.error(t('gift.form.upload.image_type_error'));
       return false;  // 阻止上传
     }
     if (!isLt5MB) {
       message.error(t('gift.form.upload.file_too_large'));
       return false;
     }
     return true;
   }}
   ```
3. **实时预览**：
   ```tsx
   <FormItem name="icon_url">
     <Upload maxCount={1} beforeUpload={...} onChange={handleIconChange}>
       <Button>上传图标</Button>
     </Upload>
   </FormItem>
   {previewUrl && <img src={previewUrl} alt="preview" style={{ maxWidth: 100 }} />}
   ```

**API 调用**：
```ts
// 新增
await adminCreateGift({
  code: string;
  name_cn: string;
  name_ar: string;
  price: number;
  tier: 1|2|3|4;
  icon_url: string;
  animation_url?: string;
  description?: string;
});

// 编辑
await adminUpdateGift(giftId, {
  name_cn?: string;
  name_ar?: string;
  price?: number;
  tier?: 1|2|3|4;
  icon_url?: string;
  animation_url?: string;
  description?: string;
  // code 不可修改（后端忽略）
});
```

**成功流程**：
- API 返回后，关闭 Modal
- 调用 `onSuccess()` → 触发列表刷新

**错误处理**：
- 400（验证错误）：显示 `errorMessage`，保留 Modal
- 其他错误：显示通用错误信息

---

## 数据流

```
GiftManagementPage (useGiftsPage Hook)
    ├─ API: adminListGifts(params, signal)
    │   └─ 参数：page, size=20, tier?, include_inactive?
    │
    ├─ Tier 筛选下拉 → setTierFilter()
    │   └─ 触发 useEffect → 重新 fetchGifts
    │
    ├─ 状态筛选下拉 → setStatusFilter()
    │   └─ 触发 useEffect → 重新 fetchGifts (含客户端过滤逻辑)
    │
    ├─ GiftsTable 展示列表
    │   ├─ Switch 上下架点击 → handleToggleActive()
    │   │   ├─ 乐观更新列表
    │   │   ├─ API: adminUpdateGift(giftId, { is_active })
    │   │   └─ 失败时回滚列表
    │   │
    │   ├─ "编辑"按钮点击 → setEditingId() → GiftEditModal open
    │   │   └─ 回调 handleEditSuccess() → 刷新列表
    │   │
    │   └─ "删除"按钮点击 → Modal.confirm → API: adminDeleteGift()
    │       └─ 成功后刷新列表
    │
    └─ "新增礼物"按钮点击 → setEditingId(undefined) → GiftEditModal open
        ├─ GiftEditModal 组件
        │   ├─ 上传图标 → API: adminUploadGiftAsset(file, 'icon')
        │   │   └─ 返回 URL → form.setFieldValue('icon_url', url)
        │   │
        │   ├─ Form 提交 → API: adminCreateGift({ code, name_cn, ... })
        │   │   (或编辑时 adminUpdateGift)
        │   │
        │   └─ 成功 → onSuccess() → 刷新列表
        │
        └─ Modal 关闭
```

---

## API 客户端函数（T-20012 新增）

**文件**：`src/core/network/apiClient.ts`

| 函数 | HTTP 方法 | 端点 | 说明 |
|------|---------|------|------|
| `adminListGifts(params?, signal?)` | GET | `/admin/gifts` | 礼物列表（分页/筛选） |
| `adminCreateGift(body)` | POST | `/admin/gifts` | 新增礼物 |
| `adminUpdateGift(giftId, body)` | PUT | `/admin/gifts/:id` | 编辑礼物 |
| `adminDeleteGift(giftId)` | DELETE | `/admin/gifts/:id` | 删除礼物（软删除） |
| `adminUploadGiftAsset(file, kind, signal?)` | POST | `/admin/gifts/upload?kind=:kind` | 上传图片/动效（multipart） |

**列表 API 参数**：
```ts
interface AdminListGiftsParams {
  page?: number;           // 分页
  size?: number;           // 每页数量
  tier?: 1|2|3|4;         // Tier 筛选
  include_inactive?: boolean;  // 是否包含下架礼物
}
```

---

## 数据库字段映射

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 礼物 ID |
| `code` | String | 唯一代码 |
| `name_cn` | String | 中文名 |
| `name_ar` | String | 阿拉伯语名 |
| `price` | Int | 价格（钻石数） |
| `tier` | Int | 等级（1-4） |
| `icon_url` | String | 图标 URL |
| `animation_url` | String | 动效 Lottie URL（可空） |
| `description` | String | 描述（可空） |
| `is_active` | Bool | 是否上架 |
| `deleted_at` | Timestamp | 软删除时间戳（可空） |
| `created_at` | Timestamp | 创建时间 |
| `updated_at` | Timestamp | 更新时间 |

---

## 权限控制（RBAC）

**菜单可见性**：
- ✅ super_admin / operator 可见"礼物管理"菜单项
- ❌ finance / cs 等其他角色不可见

**操作权限**（由后端 API 401/403 二重防护）：
- 新增礼物：super_admin / operator
- 编辑礼物：super_admin / operator
- 删除礼物：super_admin / operator
- 上下架：super_admin / operator

---

## 国际化 (i18n)

**文件**：`src/i18n/locales/en.ts` / `zh.ts`

**礼物管理相关 key**（T-20012 新增 60+ key）：
```json
{
  "gift.mgmt.title": "Gift Management",
  "gift.mgmt.add": "Add Gift",
  "gift.mgmt.tier": "Tier",
  "gift.mgmt.status": "Status",
  "gift.mgmt.status_all": "All",
  "gift.mgmt.status_active": "Active",
  "gift.mgmt.status_inactive": "Inactive",
  "gift.mgmt.status_deleted": "Deleted",
  "gift.form.code": "Code",
  "gift.form.name_cn": "Name (Chinese)",
  "gift.form.name_ar": "Name (Arabic)",
  "gift.form.price": "Price",
  "gift.form.tier": "Tier",
  "gift.form.icon": "Icon",
  "gift.form.animation": "Animation URL",
  "gift.form.description": "Description",
  "gift.form.upload.image_type_error": "Only image files are supported",
  "gift.form.upload.file_too_large": "File must be less than 5MB",
  "gift.form.create_success": "Gift created successfully",
  "gift.form.update_success": "Gift updated successfully",
  "gift.form.price_error": "Price must be greater than 0",
  // ...
}
```

---

## 测试覆盖

**文件**：`src/features/gift/__tests__/GiftManagementPage.test.tsx`

**关键测试场景**（16 个单元测试，覆盖 W12-07~W12-10）：
- W12-07：菜单可见性（RBAC）
- W12-08：Switch 切换后列表更新
- W12-09：price=0 时提交按钮禁用
- W12-10：非图片文件上传错误提示
- G11a：inactive 筛选只显示下架礼物
- G11b：inactive 筛选时请求携带 `include_inactive=true`

**修复后覆盖率**：
- Stmts 89.46% / Branch 74.24% / Funcs 84.61% / Lines 89.46%

---

## 相关文档

- [T-20012 技术设计文档](../../tds/web/T-20012.md)
- [T-10014 Admin 礼物 CRUD API](../../tds/adminServer/T-10014.md)
- [Web 架构总索引](./index.md)
- [用户管理模块](./user-management.md)
