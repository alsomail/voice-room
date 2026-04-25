# 测试套件：ANALYTICS 用户行为流 Tab（Web Admin）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-20013（用户详情页 EventStreamTab + event_name 多选下拉 + 时间窗筛选 + CSV 导出 + 关键字高亮）。

---

## TC-ANALYTICS-00001：行为流 Tab 默认加载 + 时间窗切换
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. super_admin 登录；用户 U1 最近 24h 有 150 条事件，最近 7 天 1200 条。
2. 进入 `/users/U1` 抽屉。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 切换到"行为流"Tab | 发起请求 `?from=now-24h&to=now&limit=100`；列表按 server_ts DESC 渲染 |
| 2 | `AdminWeb` | 切换时间窗为"最近 7 天" | 请求 `from=now-7d`；列表刷新到 1200 条（分页加载前 100） |
| 3 | `AdminWeb` | 时间窗选"自定义"超过 30 天 | 前端 disable 确认按钮 + Tooltip "最多 30 天" |
| 4 | `AdminWeb` | 无事件用户 U_NEW | 显示空状态"暂无行为记录" |

**【数据清理】**
- 无。

---

## TC-ANALYTICS-00002：event_name 多选下拉 + properties JSON 展开
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 同 00001。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击 event_name 下拉 | 展示事件枚举（从后台枚举接口），支持搜索 |
| 2 | `AdminWeb` | 多选 `gift_send_success` + `gift_send_fail` | 请求 `event_name=gift_send_success,gift_send_fail`；列表仅这两种 |
| 3 | `AdminWeb` | 每行事件 `properties` 区域 | 默认折叠，点击展开 JSON 格式化显示 |
| 4 | `AdminWeb` | 搜索框输 `gift_id` | 展开的 JSON 中 `gift_id` 关键字高亮为黄底 |
| 5 | `AdminWeb` | properties 含 HTML 如 `<img src=x onerror=alert(1)>` | 渲染为纯文本，不触发脚本执行（XSS 防护）|

**【数据清理】**
- 无。

---

## TC-ANALYTICS-00003：CSV 导出 + limit=100 前端约束
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. 当前筛选下有 1200 条事件。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击"导出 CSV" | 文件名 `events_U1_{yyyymmdd_HHMMSS}.csv`，UTF-8 BOM |
| 2 | `File` | 行数统计 | 1001（含表头，最多 1000 条前置上限） |
| 3 | `AdminWeb` | DevTools 观察 GET 请求 | 单次请求 limit ≤100；CSV 导出为多次分页聚合（或后端单独 CSV 接口） |
| 4 | `AdminWeb` | 手动改 URL 强行 limit=500 | 前端 AbortController 拦截或后端 400 `LIMIT_EXCEEDED` |
| 5 | `AdminWeb` | 连续快速切换时间窗 | 上一次请求被 AbortController 取消，仅最后一次生效 |

**【数据清理】**
- 无。

---

## TC-ANALYTICS-00004：权限控制 - admin_* 事件仅 super_admin 可见
**【元数据】**
- **归属模块**：`ANALYTICS`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. `operator` 账号登录；U1 有事件 `admin_ban_user`、`login_verify_success`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 进入 U1 行为流 Tab | 列表不显示 `admin_*` 开头事件 |
| 2 | `AdminWeb` | event_name 下拉选项 | 不含 `admin_*` 项（前端按角色裁剪） |
| 3 | `AdminWeb` | 直接 URL 带 `event_name=admin_ban_user` | 后端 HTTP 403；前端 Toast "无权查看该事件" |
| 4 | `AdminWeb` | 换 super_admin 登录 | 全部事件可见，event_name 枚举含 admin_* |

**【数据清理】**
- 无。
