# 模块 5: Web 管理端增强 (Admin Web Enhancements)

> 返回 [任务总索引](./index.md)

## Phase 0.5: 交互壳体与基础体验

> **说明**：Phase 0 的代码已全部完成，但 Android App 仍停留在 Auth Bootstrap 调试页面，缺少完整的用户交互壳。Phase 0.5 聚焦于让 App "能看能用"：中东黑金视觉主题、Splash 启动页、主页三Tab框架、个人中心，以及对已有页面的视觉升级。Web 端补充解封确认弹窗和活水房间监控。  
> **产品设计规范**: 详见 [doc/product/android_app_design.md](../product/android_app_design.md)


## 模块 5: Web 管理端增强 (Admin Web Enhancements)

#### Web 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|------------|
| **T-20010** | Web | User | 解封用户确认弹窗 [TDS](../tds/web/T-20010.md) | T-20007, T-10009 | UnbanModal 组件：解封原因+备注+二次确认+API调用。与 BanModal 对称 | 1. 封禁用户 [解封] 弹出 UnbanModal<br>2. 原因必填<br>3. 成功后状态变"正常"<br>4. isConfirming 防重复 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块5-Web管理端增强.md) | ✅ N/A | ✅ Released | [T-20010.md](../design/adminWeb/T-20010.md) |
| **T-20011** | Web | Room | 活水房间监控增强 [TDS](../tds/web/T-20011.md) | T-20004 | 房间列表增加"活跃状态"Tag(活跃/冷清/异常) + "持续时长"列 + 活跃度筛选条件 | 1. 新增活跃状态+持续时长两列<br>2. Tag颜色根据规则渲染<br>3. 活跃度筛选可过滤<br>4. 异常房间行高亮<br>5. **现有功能不回归** | 4h | Dod | ✅ Done | [✅ Passed](../review/模块5-Web管理端增强.md) | [⚠️ SKIP-KNOWN](../../tests/report-20260429-072049/WEB/TC-ROOM/Report.md) | ⏳ Pending | [T-20011.md](../design/adminWeb/T-20011.md) |

---
