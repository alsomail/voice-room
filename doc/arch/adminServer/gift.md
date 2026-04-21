<!--
[AI 读写指令]
1. 本文件由 Plan Agent 创建，记录 T-10014 礼物 CRUD 管理 API 的架构设计
2. TDD 阶段完成实现后，在【二、API 设计】中更新实际实现的代码片段
3. DoD 阶段同步文档状态后，在【四、状态检查清单】更新完成时间戳
-->

# Admin Server 礼物管理模块 (Gift Module) - T-10014

**最后更新**: 2025-07-17 (DoD 完成)  
**负责人**: Dod  
**状态**: ✅ 已完成

---

## 一、模块概述

### 功能定位
Admin Server 的 Gift 模块提供后台运营人员**管理虚拟礼物配置**的能力，包括礼物列表查询、创建、更新、删除、上传图片/Lottie 动效。所有写操作自动记入审计日志，并通过 Redis 事件通知 App Server 清除缓存。

### 核心特性
- **完整 CRUD 操作**：列表查询（含分页+过滤）、创建、更新、删除（软删）
- **异步文件上传**：使用 `tokio::fs` 异步 I/O 上传图片/Lottie JSON，防止阻塞事件循环
- **URL 白名单校验**：`icon_url` / `animation_url` 必须指向受控目录 `/uploads/gifts/` 或 CDN 白名单前缀，防止内容劫持风险
- **缓存失效事件**：写操作后清除 App Server 的礼物列表缓存 `gifts:list:*`
- **审计追溯**：所有变更记入 `admin_logs`，包含完整差异（create/update/delete 具体变更内容）
- **RBAC 权限控制**：礼物读取权限 `operator/super_admin`，删除权限 `super_admin` 专享

### 关联 Task
- **上游依赖**: T-00019 (App Server 礼物配置表初始化)、T-10012 (审计日志模块)
- **下游消费者**: T-20012 (Web 后台礼物管理 UI)、T-30028 (Android 送礼面板)

---

## 二、API 设计

### 接口清单

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/v1/admin/gifts` | GiftRead | 列表查询（分页+过滤） |
| POST | `/api/v1/admin/gifts` | GiftWrite | 创建礼物 |
| PUT | `/api/v1/admin/gifts/:id` | GiftWrite | 更新礼物 |
| DELETE | `/api/v1/admin/gifts/:id` | GiftDelete | 软删除礼物 |
| POST | `/api/v1/admin/gifts/upload` | GiftWrite | 上传图片/Lottie |

### 2.1 GET 列表接口

**请求**:
```http
GET /api/v1/admin/gifts?include_inactive=true&page=1&size=20 HTTP/1.1
Authorization: Bearer <admin_jwt>
```

**查询参数**:
| 参数 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `include_inactive` | bool | 可选，默认=false | true 则返回所有未上架 + 已上架礼物（除已软删）；false 仅返回 `is_active=true` 的礼物 |
| `page` | i32 | 可选，默认=1 | 分页号，从 1 开始 |
| `size` | i32 | 可选，默认=50，最大=200 | 每页记录数 |

**响应 200** (成功):
```json
{
  "code": 0,
  "data": {
    "total": 8,
    "page": 1,
    "size": 50,
    "items": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "code": "unicorn_01",
        "name_en": "Unicorn",
        "name_ar": "يونيكورن",
        "icon_url": "/uploads/gifts/2026-04-21/unicorn.png",
        "price": 66,
        "tier": 3,
        "effect_level": 3,
        "animation_url": "https://cdn.your-domain.com/unicorn.json",
        "is_active": true,
        "sort_order": 35,
        "is_deleted": false,
        "created_at": "2025-07-17T10:00:00Z",
        "updated_at": "2025-07-17T10:00:00Z"
      }
    ]
  }
}
```

---

### 2.2 POST 创建接口

**请求**:
```http
POST /api/v1/admin/gifts HTTP/1.1
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "code": "unicorn_01",
  "name_en": "Unicorn",
  "name_ar": "يونيكورن",
  "icon_url": "/uploads/gifts/2026-04-21/unicorn.png",
  "price": 66,
  "tier": 3,
  "effect_level": 3,
  "animation_url": "https://cdn.your-domain.com/unicorn.json",
  "sort_order": 35,
  "is_active": true
}
```

**参数说明**:
| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `code` | string | 唯一，长度 1-32，仅英文数字下划线 | 礼物编码 |
| `name_en` | string | 1-64 字符 | 英文名称 |
| `name_ar` | string | 1-64 字符 | 阿拉伯文名称 |
| `icon_url` | string | 必填，白名单前缀 | 礼物图标 URL（必须以 `/uploads/gifts/` 或 CDN 白名单前缀开头） |
| `animation_url` | string | 可选，白名单前缀（若提供则校验） | Lottie 动效 JSON URL |
| `price` | i32 | ≥ 1 | 礼物价格（钻石数） |
| `tier` | i32 | ∈ [1,5] | 礼物等级（影响前端排序） |
| `effect_level` | i32 | ∈ [1,5] | 特效级别 |
| `sort_order` | i32 | ≥ 0 | 同级内排序权重 |
| `is_active` | bool | 默认=true | 是否上架 |

**响应 201** (创建成功):
```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "code": "unicorn_01",
    "name_en": "Unicorn",
    "name_ar": "يونيكورن",
    "icon_url": "/uploads/gifts/2026-04-21/unicorn.png",
    "price": 66,
    "tier": 3,
    "effect_level": 3,
    "animation_url": "https://cdn.your-domain.com/unicorn.json",
    "is_active": true,
    "sort_order": 35,
    "is_deleted": false,
    "created_at": "2025-07-17T10:00:00Z",
    "updated_at": "2025-07-17T10:00:00Z"
  }
}
```

**错误响应**:
| HTTP | code | 说明 |
|------|------|------|
| 400 | 40003 | price < 1 / tier ∉ [1,5] / effect_level ∉ [1,5] / code 不符合格式 / name_en/ar 超长 |
| 400 | 40006 | icon_url 不在白名单前缀内 / animation_url 不在白名单前缀内 |
| 409 | 40900 | code 已存在（重复） |

---

### 2.3 PUT 更新接口

**请求**:
```http
PUT /api/v1/admin/gifts/{id} HTTP/1.1
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "name_en": "Unicorn v2",
  "price": 88,
  "is_active": false,
  "sort_order": 40
}
```

**参数说明**:
- 所有字段可选，仅更新提供的字段
- 校验规则同 POST 创建
- 更新 `is_active=false` 后，App Server 的 `/gifts/list` 将不再返回此礼物（缓存失效）

**响应 200** (更新成功):
```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "code": "unicorn_01",
    "name_en": "Unicorn v2",
    "price": 88,
    "is_active": false,
    "updated_at": "2025-07-17T11:00:00Z"
  }
}
```

---

### 2.4 DELETE 删除接口

**请求**:
```http
DELETE /api/v1/admin/gifts/{id} HTTP/1.1
Authorization: Bearer <admin_jwt>
```

**响应 200** (删除成功):
```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "Gift soft-deleted"
  }
}
```

**特点**:
- 本质为**软删除** (`is_deleted=true`)，数据库记录仍存在
- 所有查询自动过滤已删除礼物（除非明确指定返回）
- 再次 DELETE 相同 id 返回 404

**错误响应**:
| HTTP | code | 说明 |
|------|------|------|
| 403 | 40301 | 非 super_admin 角色（权限不足） |
| 404 | 40400 | 礼物不存在或已删除 |

---

### 2.5 POST 上传接口

**请求**:
```http
POST /api/v1/admin/gifts/upload HTTP/1.1
Authorization: Bearer <admin_jwt>
Content-Type: multipart/form-data

--boundary123
Content-Disposition: form-data; name="file"; filename="unicorn.png"
Content-Type: image/png

[二进制文件内容]
--boundary123
Content-Disposition: form-data; name="kind"

icon
--boundary123--
```

**参数说明**:
| 参数 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `file` | multipart file | 必填 | 上传文件内容 |
| `kind` | string | 必填，枚举: `icon` / `animation` | 文件类型 |

**白名单配置**:
- **icon** (图片): MIME: `image/png`, `image/jpeg`, `image/webp`；Size ≤ 1MB
- **animation** (Lottie): MIME: `application/json`；Size ≤ 2MB；JSON 字段需含 `"v"` (版本号，Lottie 标识)

**响应 200** (上传成功):
```json
{
  "code": 0,
  "data": {
    "url": "/uploads/gifts/2025-07-17/a1b2c3d4e5f6.png",
    "file_name": "a1b2c3d4e5f6.png"
  }
}
```

**错误响应**:
| HTTP | code | 说明 |
|------|------|------|
| 400 | 40007 | MIME 类型不在白名单 / 文件大小超限 / 无法解析 Lottie JSON |
| 400 | 40003 | kind 参数缺失或不合法 |
| 413 | 41300 | 文件过大 |

---

## 三、核心数据流

### 3.1 创建流程

```
POST /api/v1/admin/gifts
  ↓
[RBAC 中间件] 校验 JWT + 权限 (GiftWrite)
  ↓ 若失败 → 403
[参数校验]
  - code 唯一性检查
  - code 格式校验 (1-32 chars, [a-zA-Z0-9_])
  - name_en / name_ar 长度校验 (1-64)
  - price ≥ 1
  - tier ∈ [1,5]
  - effect_level ∈ [1,5]
  - icon_url 白名单前缀校验 (ALLOWED_URL_PREFIXES)
  - animation_url 可选，若提供则白名单前缀校验
  ↓ 若失败 → 400 (40003/40006)
[数据库操作]
  - INSERT gifts 表，生成 UUID id
  ↓ 若 code 重复 → 409 (40900)
[审计日志]
  - INSERT admin_logs (action='gift_create', detail={完整创建字段})
[缓存失效]
  - Redis DEL gifts:list:ar gifts:list:en
  ↓ 发布失败仅 warn
[响应] 201 {id, code, name_en, ...}
```

### 3.2 更新流程

```
PUT /api/v1/admin/gifts/:id
  ↓
[RBAC 中间件] 校验 JWT + 权限 (GiftWrite)
[参数校验] 仅校验提供的字段
  - icon_url / animation_url 若提供则白名单校验
  - price / tier / effect_level 若提供则范围校验
  ↓ 若失败 → 400
[数据库操作]
  - SELECT gifts WHERE id=? (取原值用于 diff)
  ↓ 若不存在 → 404
  - UPDATE gifts SET ... WHERE id=?
[审计日志]
  - INSERT admin_logs (action='gift_update', detail={changes: 序列化的 UpdateRequest})
[缓存失效]
  - Redis DEL gifts:list:*
  ↓ 发布失败仅 warn
[响应] 200 {id, 更新后的字段}
```

### 3.3 删除流程

```
DELETE /api/v1/admin/gifts/:id
  ↓
[RBAC 中间件] 校验 JWT + 权限 (GiftDelete — 仅 super_admin)
  ↓ 若失败 → 403
[数据库操作]
  - SELECT gifts WHERE id=? (取被删除记录，用于 audit detail)
  ↓ 若不存在或已删除 → 404
  - UPDATE gifts SET is_deleted=true WHERE id=?
[审计日志]
  - INSERT admin_logs (action='gift_delete', detail={code, name_en, is_active, ...})
[缓存失效]
  - Redis DEL gifts:list:*
[响应] 200 {message: "Gift soft-deleted"}
```

### 3.4 权限矩阵

| 角色 | GiftRead | GiftWrite | GiftDelete |
|------|----------|-----------|-----------|
| super_admin | ✅ | ✅ | ✅ |
| operator | ✅ | ✅ | ❌ |
| cs | ❌ | ❌ | ❌ |
| finance | ❌ | ❌ | ❌ |

---

## 四、模块结构

```text
app/adminServer/src/modules/gift/
├── mod.rs                  # 模块入口、pub use
├── dto.rs                  # CreateGiftRequest / UpdateGiftRequest / GiftResponse
├── handler.rs              # 5 个 HTTP Handler (list/create/update/delete/upload)
├── service.rs              # GiftService 业务逻辑、文件上传、缓存失效事件
├── repo.rs                 # GiftRepository trait、PgGiftRepository、FakeGiftRepository
└── [涉及的其他模块修改]
    ├── bootstrap/router.rs # 注册 5 条路由
    ├── common/auth/context.rs # 新增 Permission::GiftWrite / GiftDelete / GiftRead
    └── common/error.rs     # 新增 AppError::DuplicateCode(String)
```

### 4.1 关键数据库操作

**涉及表**:
- `gifts` — CREATE / READ / UPDATE / DELETE（软删）
- `admin_logs` — INSERT (action='gift_create/gift_update/gift_delete')

**索引策略**:
- `gifts(code)` — UNIQUE 唯一索引，防止重复
- `gifts(is_active, is_deleted, sort_order DESC)` — 复合索引，加速列表查询

### 4.2 存储架构

**本地存储**（MVP）:
- 存储路径：`app/server/static/uploads/gifts/{date}/{uuid}.{ext}`
- 使用 `tokio::fs::create_dir_all / tokio::fs::write` 异步 I/O
- 防止在 Tokio 工作线程中阻塞

**后续升级**（S3 适配器）:
- 遵循防腐层模式，`StorageProvider` trait
- 当前 MVP 使用 `LocalStorageProvider`

---

## 五、错误码映射

| ErrorCode | HTTP 状态 | 说明 |
|-----------|----------|------|
| `ValidationError` (40003) | 400 | 参数校验失败（格式/范围） |
| `InvalidUrl` (40006) | 400 | URL 不在白名单前缀内 |
| `InvalidMimeType` (40007) | 400 | 文件 MIME 类型不在白名单 |
| `DuplicateCode` (40900) | 409 | gift code 已存在 |
| `NotFound` (40400) | 404 | 礼物不存在或已删除 |
| `Unauthorized` (40101/40102) | 401 | JWT 无效/过期 |
| `Forbidden` (40301) | 403 | 权限不足 |
| `PayloadTooLarge` (41300) | 413 | 上传文件过大 |

---

## 六、验收用例

### TDD 验收标准 (GC01~GC12)

| 用例 | 操作 | 期望结果 | 关键断言 |
|------|------|---------|--------|
| **GC01** | POST 创建，所有必填字段合法 | 201，返回 id | 数据库新增 1 条记录 |
| **GC02** | POST 创建，code 重复 | 409 (40900) | 请求被拒绝 |
| **GC03** | GET 不含 `include_inactive`，默认 is_active=false 的礼物不返回；含 `include_inactive=true` 则全返回 | 200，列表符合过滤 | 分页正确，数据完整 |
| **GC04** | PUT `is_active=false`，后查询 App Server `/gifts/list` | 缓存已失效，礼物不再返回 | Redis key `gifts:list:*` 已清除 |
| **GC05** | DELETE 软删后再 DELETE 同一 id | 首次 200，二次 404 | is_deleted=true，再次访问返回 404 |
| **GC06** | cs 角色尝试 DELETE | 403 | 权限校验生效 |
| **GC07** | 上传 gif 图片（非白名单 MIME） | 400 (40007) | 拒绝上传 |
| **GC08** | 上传 >1MB 的图片 | 400 (40007) | 文件大小校验生效 |
| **GC09** | POST price=0 | 400 (40003) | 参数校验失败 |
| **GC10** | POST tier=6 | 400 (40003) | 范围校验失败 |
| **GC11** | 执行任意写操作（POST/PUT/DELETE）后，admin_logs +1 | 操作完成 | 审计日志已落库 |
| **GC12** | POST 创建成功后，Redis 缓存 keys 已清除 | 操作完成 | `gifts:list:ar` / `gifts:list:en` key 不存在 |

### Review 发现与修复确认

| Review 问题 | 等级 | 修复情况 |
|------------|------|---------|
| HIGH-1: upload handler 使用阻塞同步 I/O | 🟠 HIGH | ✅ 已修复（`tokio::fs` 异步 I/O） |
| MEDIUM-1: icon_url 缺失白名单校验 | 🟡 MEDIUM | ✅ 已修复（`ALLOWED_URL_PREFIXES` 前缀白名单） |
| MEDIUM-2: audit detail 过于稀疏 | 🟡 MEDIUM | ✅ 已修复（update 含 `changes` 字段，delete 含 `code`/`name_en`） |
| R2-MEDIUM: animation_url 缺少白名单校验 | 🟡 MEDIUM | ✅ 已修复（animation_url 追加与 icon_url 完全对称的白名单校验，新增 GS-15/GS-16 单元测试） |
| LOW-1: upload_dir 字段过度公开 | 🔵 LOW | ✅ 已修复（改为 `pub(crate)` 可见性） |

---

## 七、外部依赖

| 依赖 | 来源 | 用途 |
|-----|------|------|
| `axum::extract::multipart` | axum crate | multipart/form-data 解析 |
| `tokio::fs` | tokio crate | 异步文件 I/O（非阻塞） |
| `sqlx::PgPool` | infrastructure::db | 数据库事务 |
| `AdminAuthContext` | common::auth | JWT 解析与权限检查 |
| `EventPublisher` trait | modules::event (T-10011) | Redis 缓存失效事件发布 |
| `AuditLogger` | modules::audit (T-10012) | 审计日志记录 |
| `mime_guess` | 第三方 crate | MIME 类型识别 |

---

## 八、测试覆盖

### 集成测试 (18 条)
- **验收用例**: GC01~GC12 (12 条) — 覆盖完整 CRUD + 缓存 + 权限 + 文件上传
- **扩展用例**: extra 01~03 (3 条) — 额外边界场景
- **Review 用例**: review 01~03 (3 条) — 专项修复验证（高性能异步 I/O + 白名单 + audit detail）

### 单元测试 (36 条)
- **DTO**: 3 条
- **Repository**: 6 条
- **Service**: 16 条（含 GS-13/GS-14/GS-15/GS-16 白名单校验）
- **Handler**: 11 条（含 HV-11 异步文件上传）

### 全部测试状态
- ✅ `cargo test` — 340 passed, 0 failed
- ✅ `cargo clippy -- -D warnings` — 零警告

---

## 九、状态检查清单

- [x] TDS 文档完成（[doc/tds/adminServer/T-10014.md](../../tds/adminServer/T-10014.md)）
- [x] 代码实现完成（36 新增单元测试 + 18 集成测试，全量 340 tests passed）
- [x] Clippy 零警告（Review 修复 HIGH-1 异步 I/O）
- [x] 异步文件上传验证（HIGH-1 修复确认，tokio::fs 非阻塞）
- [x] URL 白名单校验（MEDIUM-1/R2-MEDIUM 修复确认，icon_url + animation_url 双重白名单）
- [x] 审计日志完整性（MEDIUM-2 修复确认，update 含 changes，delete 含 code）
- [x] 缓存失效事件（GC12 验证，Redis DEL 生效）
- [x] 权限矩阵验证（GC06 验证，DELETE 权限限制）
- [x] 架构文档已同步 (2025-07-17)

---

## 十、相关文档

- **TDS 完整设计**: [doc/tds/adminServer/T-10014.md](../../tds/adminServer/T-10014.md)
- **RBAC 权限体系**: [rbac.md](./rbac.md)
- **事件发布模块**: [event.md](./event.md) ← T-10011
- **审计日志模块**: [audit.md](./audit.md) ← T-10012
- **App Server 礼物配置**: [../appServer/gift.md](../appServer/gift.md) ← T-00019/T-00020
- **产品方向**: [doc/product/phase1_gift_economy.md](../../product/phase1_gift_economy.md)
