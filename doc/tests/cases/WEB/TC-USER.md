# 测试套件：USER 用户管理（Web）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-20007（用户列表 + 详情抽屉）、T-20008（BanModal）、T-20010（UnbanModal）。

---

## TC-USER-00001：用户列表 - 分页/搜索/角色权限
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. DB 有 30 个用户，含 1 名已封禁。
2. operator 登录。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 访问 `/users` | 表格渲染，列含 ID/昵称/手机号/状态/注册时间/操作 |
| 2 | `AdminWeb` | 已封禁行 | 状态列 Tag 红色 `已封禁` |
| 3 | `AdminWeb` | 搜索框输入 `13800` | 请求 `?keyword=13800`，表格刷新 |
| 4 | `AdminWeb` | 状态筛选下拉选"已封禁" | 仅显示 1 行 |
| 5 | `AdminWeb` | CS 角色登录 | 操作列"封禁"按钮置灰或不显示 |
| 6 | `AdminWeb` | finance 角色访问 `/users` | 页面显示 `403 无权限` |

**【数据清理】**
- 无。

---

## TC-USER-00002：用户详情抽屉 + 封禁 E2E 多端闭环
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 在 Android 端登录在线，在 R1 内。
2. operator 登录 AdminWeb。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 列表点击 U1 行"详情" | Drawer 滑入，标题"用户详情 - {昵称}"，显示基础信息/钱包/流水/设备 |
| 2 | `AdminWeb` | 点击红色"封禁"按钮 | 弹出 BanModal |
| 3 | `AdminWeb` | BanModal 内容 | 类型单选（临时/永久）、时长下拉（仅临时显示）、封禁原因 TextArea（required） |
| 4 | `AdminWeb` | 不填原因点"确认" | 原因框下方红色字"请输入封禁原因" |
| 5 | `AdminWeb` | 选择"临时 24 小时"+ 原因"测试违规"→ 确认 | 二次 Modal.confirm 弹出；确认后请求 POST ban |
| 6 | `DB` | users.status=`banned` & admin_logs 新增 | 成立 |
| 7 | `Android(U1)` | WS 观察 | 3 秒内收到 BanNotice，被踢下线回到 LoginScreen + Toast "您的账号已被封禁" |
| 8 | `AdminWeb` | Drawer 刷新 | U1 状态 Tag 变红 `已封禁`，封禁按钮隐藏，"解封"按钮出现 |

**【数据清理】**
- 解封 U1。

---

## TC-USER-00003：解封弹窗（T-20010）- 原因必填 + 二次确认 + 刷新
**【元数据】**
- **归属模块**：`USER`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. U1 status=`banned`，Drawer 已打开。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 点击"解封"按钮 | 弹出 UnbanModal，标题"确认解封用户"，提示含 @昵称 |
| 2 | `AdminWeb` | Select 下拉 | 含 3 项：处罚到期 / 误封 / 其他 |
| 3 | `AdminWeb` | 不选原因直接点"确认解封" | 表单下方红色字"请选择解封原因" |
| 4 | `AdminWeb` | 选"处罚到期"+ 可选备注"审核通过" → 确认 | Modal.confirm 二次弹窗"确认解封此用户？" |
| 5 | `AdminWeb` | 再次确认 | PUT `/api/v1/admin/users/{U1}/unban`；按钮 loading 防双击 |
| 6 | `AdminWeb` | 成功后 | UnbanModal 关闭；message.success "解封成功"；Drawer 中状态 Tag 变绿 `正常` |
| 7 | `DB` | users.status=`active`，admin_logs 新增 action=`user_unban` | 成立 |
| 8 | `AdminWeb` | API 失败（模拟 500） | UnbanModal 保持打开；message.error 显示错误；可重试 |

**【数据清理】**
- 无。
