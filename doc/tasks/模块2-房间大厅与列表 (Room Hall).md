# 模块 2: 房间大厅与列表 (Room Hall)

> 返回 [任务总索引](./index.md)

## Phase 0: MVP 基础设施 (预计 6-8 周)


## 模块 2: 房间大厅与列表 (Room Hall)

#### App Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-00006** | App Server | Room | 房间表设计 [TDS](../tds/server/T-00006.md) | T-00001 | 设计 `rooms` 表（id, owner_id, title, type, member_count, status） | 1. 外键关联 users 表<br>2. 索引（status, created_at）<br>3. 房间类型枚举（normal/password/paid） | 2h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |
| **T-00007** | App Server | Room | 创建房间接口 [TDS](../tds/server/T-00007.md) | T-00006, T-00004 | POST `/api/v1/rooms` | 1. 需要 JWT 认证<br>2. 标题长度 1-30 字符<br>3. 用户同时只能拥有 1 个房间<br>4. 成功返回 201 + room_id | 4h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |
| **T-00008** | App Server | Room | 房间列表接口 [TDS](../tds/server/T-00008.md) | T-00006 | GET `/api/v1/rooms?page=1&size=20` | 1. 按热度排序（member_count desc）<br>2. 过滤已关闭房间<br>3. 分页返回 (total, page, items) | 3h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |
| **T-00009** | App Server | Room | 房间详情接口 [TDS](../tds/server/T-00009.md) | T-00008 | GET `/api/v1/rooms/:id` | 1. 包含房主信息<br>2. 在线人数<br>3. 麦位列表（初始为空） | 2h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |
| **T-00010** | App Server | Room | 关闭房间接口 [TDS](../tds/server/T-00010.md) | T-00007 | DELETE `/api/v1/rooms/:id` | 1. 只有房主可关闭<br>2. 广播 RoomClosed 事件<br>3. 踢出所有成员 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |

#### Admin Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-10004** | Admin Server | Room | 房间列表接口（后台） [TDS](../tds/adminServer/T-10004.md) | T-00006, T-10003 | GET `/api/v1/admin/rooms` | 1. 支持多条件筛选（房主/状态/时间）<br>2. 返回完整字段（含举报次数）<br>3. 支持导出 CSV | 3h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |
| **T-10005** | Admin Server | Room | 房间详情接口（后台） [TDS](../tds/adminServer/T-10005.md) | T-10004 | GET `/api/v1/admin/rooms/:id` | 1. 包含所有成员列表<br>2. 最近聊天记录<br>3. 举报记录 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |
| **T-10006** | Admin Server | Room | 强制关闭房间接口 [TDS](../tds/adminServer/T-10006.md) | T-10005 | DELETE `/api/v1/admin/rooms/:id` | 1. 需要 RoomForceClose 权限（operator/super_admin）<br>2. 不存在/软删除 → 404/40400<br>3. 已 closed → 409/40901<br>4. 无 owner 检查 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/API/TC-ROOM/Report.md) | ⏳ Pending |

#### Web 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-20003** | Web | Dashboard | 数据看板首页 [TDS](../tds/web/T-20003.md) | T-20002, **T-10010** | 实现首页数据大盘 | 1. 实时在线人数/房间数<br>2. 今日 DAU/新增用户<br>3. ECharts 趋势图<br>4. 自动刷新（每 30 秒） | 6h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/WEB/TC-ROOM/Report.md) | ⏳ Pending |
| **T-20004** | Web | Room | 房间管理页面 [TDS](../tds/web/T-20004.md) | T-10004 | Ant Design Table 展示房间列表 | 1. 支持搜索/筛选<br>2. 分页加载<br>3. 点击查看详情 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/WEB/TC-ROOM/Report.md) | ⏳ Pending |
| **T-20005** | Web | Room | 房间详情弹窗 [TDS](../tds/web/T-20005.md) | T-10005, T-20004 | Modal 展示房间详情 | 1. 显示成员列表<br>2. 实时聊天记录<br>3. [强制关闭] 按钮 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | [✅ Passed](../../tests/report-20260429-072049/WEB/TC-ROOM/Report.md) | ⏳ Pending |

#### Android 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-30005** | Android | Room | 大厅页 UI (Compose) [TDS](../tds/android/T-30005.md) | T-00008 | LazyVerticalGrid 展示房间列表 | 1. Coil 加载房主头像<br>2. 显示在线人数<br>3. 点击导航到房间页 | 6h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | ⚠️ Partial · 5P/1F · BUG-ANDROID-002 · [report](../../tests/report-20260429-084501/SUMMARY.md) | ⏳ Pending |
| **T-30006** | Android | Room | 房间列表 ViewModel [TDS](../tds/android/T-30006.md) | T-00008, T-30005 | Paging3 分页加载 | 1. 下拉刷新<br>2. 上拉自动加载<br>3. 错误重试 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | ✅ Pass · 6P/0F · [report](../../tests/report-20260429-084501/SUMMARY.md) | ⏳ Pending |
| **T-30007** | Android | Room | 创建房间对话框 [TDS](../tds/android/T-30007.md) | T-00007 | BottomSheet 输入房间信息 | 1. 标题输入框<br>2. 房间类型选择<br>3. 创建成功导航到房间 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | ⏭️ SKIP-OOS (无androidTest) · [report](../../tests/report-20260429-084501/SUMMARY.md) | ⏳ Pending |

---
