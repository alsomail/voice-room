# 测试套件：WALLET 钱包（Web）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-20012（AdjustBalanceModal）。

---

## TC-WALLET-00001：调整余额弹窗 - 正/负值校验 + 双重确认
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. U1 coin_balance=500。
2. operator 登录，已打开 U1 UserDetailDrawer。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | Drawer 中点击"调整余额"按钮 | 弹出 AdjustBalanceModal，标题"调整用户余额" |
| 2 | `AdminWeb` | Modal 内容 | 当前余额只读显示 `500 💎`；金额输入框（支持负数）；原因 TextArea（required） |
| 3 | `AdminWeb` | 输入 金额=200 原因="活动奖励" → 确认 | Modal.confirm 二次"确认给该用户增加 200 💎？"，确认后请求 POST adjust |
| 4 | `AdminWeb` | 成功后 | message.success；Modal 关闭；Drawer 中钱包卡片余额刷新为 `700 💎` |
| 5 | `AdminWeb` | 再次打开输入 金额=-300 | 表单下方红色警示文字"负值将扣减用户余额，请谨慎操作" |
| 6 | `AdminWeb` | 确认后 Modal.confirm | 按钮为红色 Danger 按钮"确认扣减" |
| 7 | `AdminWeb` | 输入 金额=-1000（超出余额） | 确认后 API 返回 40290；Modal 保持打开；message.error "余额不足" |
| 8 | `AdminWeb` | 金额=0 | 确认按钮置灰不可点 |
| 9 | `AdminWeb` | 原因为空 | 原因框下方红字"请填写调整原因" |
| 10 | `AdminWeb` | CS 角色打开 Drawer | "调整余额"按钮不显示或置灰 |

**【数据清理】**
- 恢复 U1 余额。
