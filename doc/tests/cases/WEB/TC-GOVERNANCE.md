# 测试套件：GOVERNANCE 房间治理日志查询页（Web Admin）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-20014（`/governance/logs` 页面 + CSV 导出 + 用户详情联动）。

---

## TC-GOVERNANCE-00001：治理日志页 - 筛选 + 分页 + 空状态
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 管理员 `super_admin` 登录后台。
2. DB 治理记录近 7 天 2000 条（kick 1200 / mute 800）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 访问 `/governance/logs` | 页面标题"房间治理日志"；时间区间默认"最近 7 天" |
| 2 | `AdminWeb` | 断言表格列 | 时间/类型/房间/操作者/目标/原因/时长；分页默认 20 条 |
| 3 | `AdminWeb` | 切换"类型"下拉选"禁麦/禁言" | 请求 `type=mute`；返回全为 mute 记录，总条数 800 |
| 4 | `AdminWeb` | 输入"操作者 admin_op01" | 列表过滤；URL 同步查询参数 |
| 5 | `AdminWeb` | 时间区间选"今日 08:00 - 09:00" 无数据 | 空状态插画 + "暂无治理记录"文案 |
| 6 | `AdminWeb` | 点击分页第 3 页 | 请求 `page=3`；滚动回顶部 |
| 7 | `AdminWeb` | 清除所有筛选 | 恢复默认条件，总数恢复 2000 |

**【数据清理】**
- 无。

---

## TC-GOVERNANCE-00002：CSV 导出 - 当前筛选 + UTF-8 BOM + 文件名
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. 已完成筛选（type=kick，近 7 天），列表 1200 条。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击"导出 CSV" | 触发下载；文件名 `governance_logs_kick_{yyyymmdd_HHMMSS}.csv` |
| 2 | `File` | 检查首 3 字节 | `EF BB BF`（UTF-8 BOM） |
| 3 | `Excel` | 用 Excel 打开 | 中文、阿语昵称正常显示（无乱码） |
| 4 | `File` | 统计 CSV 行数 | 1201（含表头） |
| 5 | `AdminWeb` | 超大时间范围（如 90 天 15 万条）点导出 | 前端提示"记录数超过 5 万条，请收紧筛选"（按产品规约限制） |

**【数据清理】**
- 无。

---

## TC-GOVERNANCE-00003：用户详情页联动跳转 + 权限控制
**【元数据】**
- **归属模块**：`GOVERNANCE`
- **测试类型**：`Integration`
- **回归级别**：`P1`

**【前置条件】**
1. 在 `/users/U1` 抽屉内有"治理记录"入口。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击"查看治理记录" | 跳转 `/governance/logs?target_user_id=U1`；筛选栏自动预填 U1 昵称 |
| 2 | `AdminWeb` | 列表操作者列点击 `admin_op01` | 跳转 `/admins/admin_op01` 详情页 |
| 3 | `AdminWeb` | 无 `governance.view` 权限的 `cs` 账号访问 `/governance/logs` | 路由守卫重定向回首页，Toast "无权限" |
| 4 | `AdminWeb` | XSS 尝试：注入房间名 `<script>alert(1)</script>` 后查询 | 表格渲染为纯文本，不执行脚本 |

**【数据清理】**
- 无。
