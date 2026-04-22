# 测试套件：ROOM 房间监控（Web）

> **需求模糊点 (Ambiguity Notes)**：
> - 无（活跃状态 Tag 规则：活跃=人数≥5 且持续≥5 分钟；冷清=人数<2；异常=持续 >6 小时 或 闪断频繁。以 TDS T-20011 为准，若不同以 TDS 为准。）

覆盖 Task：T-20004（Dashboard）、T-20005（房间管理列表）、T-20006（房间详情抽屉 + 强制关闭）、T-20011（活跃房间监控增强）。

---

## TC-ROOM-00001：Dashboard 概览 + ECharts + 30s 自动刷新
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**":`P1`

**【前置条件】**
1. 已登录进入 `/dashboard`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 页面布局 | 顶部 4 张数字卡片：在线用户/在线房间/今日新增/今日活跃，每张含 "+5.2%" 趋势 |
| 2 | `AdminWeb` | 下方折线图 | ECharts 渲染 24 小时在线人数曲线，X 轴时间，Y 轴人数，悬浮显示 tooltip |
| 3 | `AdminWeb` | 等待 30 秒 | 数字卡片自动刷新；折线图追加最新一点 |
| 4 | `AdminWeb` | 浏览器 DevTools Network | 每 30 秒看到 `GET /api/v1/admin/stats/overview` 与 `/stats/online-history` |

**【数据清理】**
- 无。

---

## TC-ROOM-00002：房间列表 - 搜索 / 筛选 / 分页
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. DB 存在 30 个房间（20 active + 10 closed）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 访问 `/rooms` | Antd Table 渲染，默认每页 20 条，共 30 条分页显示 `1 / 2` |
| 2 | `AdminWeb` | 顶部搜索框输入"测试" | 请求 `?keyword=测试`，表格实时刷新 |
| 3 | `AdminWeb` | 状态筛选下拉选"已关闭" | 请求 `?status=closed`，表格仅显示 10 条 |
| 4 | `AdminWeb` | 切换至第 2 页 | 请求 `?page=2&page_size=20` |
| 5 | `AdminWeb` | 点击列头"在线人数" | 升序/降序切换 |

**【数据清理】**
- 无。

---

## TC-ROOM-00003：房间详情抽屉 - 强制关闭完整闭环
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 active，内有 U1 WS 活跃。
2. operator 已登录。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 列表点击 R1 行"详情"按钮 | 右侧滑入 Drawer，标题"房间详情 - {R1 标题}"，含基本信息 + 在线成员列表 + 公屏最近 20 条 |
| 2 | `AdminWeb` | 点击红色"强制关闭"按钮 | 弹出 Modal.confirm "确定强制关闭该房间？" |
| 3 | `AdminWeb` | 点击 Modal 中"确认" | 按钮转 loading；DELETE `/api/v1/admin/rooms/{R1}` |
| 4 | `DB` | rooms.status WHERE id=R1 | `closed` |
| 5 | `DB` | admin_logs 最新一条 | action=`room_force_close`，target_id=R1 |
| 6 | `AdminWeb` | 成功后 | message.success "房间已关闭"；Drawer 关闭或状态更新为"已关闭"；列表中该行状态 Tag 变为红色 `已关闭` |
| 7 | `AppServer` | U1 的 WS | 收到 RoomClosed，自动从房间退出 |
| 8 | `AdminWeb` | 对已 closed 房间再次点强制关闭 | 按钮置灰不可点 |

**【数据清理】**
- 无。

---

## TC-ROOM-00004：XSS 防护 - 标题恶意输入
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Security`
- **回归级别**：`P1`

**【前置条件】**
1. DB 手工插入房间 title=`<script>alert('xss')</script>`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 访问 `/rooms` | 该行标题单元格显示 `<script>alert('xss')</script>` 原字符串（转义后） |
| 2 | `AdminWeb` | 页面不弹 alert 框 | 脚本未执行 |
| 3 | `AdminWeb` | 检查 DOM | 元素文本是转义字符，不是 `<script>` 子节点 |

**【数据清理】**
- 删除恶意测试房间。

---

## TC-ROOM-00005：活跃房间监控增强 - 状态/时长/筛选/异常高亮（T-20011）
**【元数据】**
- **归属模块**：`ROOM`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 构造数据：
   - R_A active 10 人在线，持续 3 小时（活跃）
   - R_B active 1 人在线，持续 30 分钟（冷清）
   - R_C active 8 人在线，持续 7 小时（异常）
   - R_D closed

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 访问 `/rooms` | 表头新增列"活跃状态"、"持续时长" |
| 2 | `AdminWeb` | R_A 行"活跃状态" | 绿色 Tag `活跃` |
| 3 | `AdminWeb` | R_B 行 | 灰色 Tag `冷清` |
| 4 | `AdminWeb` | R_C 行 | 红色 Tag `异常`，整行背景淡红色高亮 |
| 5 | `AdminWeb` | 持续时长列 | R_A 显示 `3h 00m`，R_C 显示 `7h 05m` |
| 6 | `AdminWeb` | 顶部"活跃度"筛选选"异常" | 仅展示 R_C |
| 7 | `AdminWeb` | 原有操作列（详情/强制关闭） | 按钮保留，点击行为同 TC-ROOM-00009，无回归 |

**【数据清理】**
- 清理测试房间。
