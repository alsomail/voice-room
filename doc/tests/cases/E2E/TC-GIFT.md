# 测试套件：E2E 端到端 - 礼物赠送闭环（Android U1 + Android U2 + Android U3 + AppServer + DB + Redis）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

---

## TC-GIFT-00001：Android U1 向麦位 U2 送礼 - 多端闭环
**【元数据】**
- **归属模块**：`GIFT`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. R1 内：U1 余额=1000；U2 主播麦 charm=0；U3 普通成员。
2. 礼物 G1 价格=100，L2 等级。
3. 榜单 Redis ZSet 为空。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android(U1)` | 点击底栏 🎁 | Bottom Sheet 从底部滑入 55% 屏高 |
| 2 | `Android(U1)` | 选中礼物 G1 + 数量 1 + 接收者 U2（默认选中） | "送出"按钮金色可点，文字"送出 100 💎" |
| 3 | `Android(U1)` | 点击"送出" | 按钮短暂 Loading |
| 4 | `AppServer` | 日志 | 收到 WS SendGift，开启 SQL 事务 |
| 5 | `DB` | users.coin_balance WHERE id=U1 | =900 |
| 6 | `DB` | users.charm_value WHERE id=U2 | =100 |
| 7 | `DB` | wallet_transactions 新增 2 行 | U1 expense 100 ref_type=`gift_send`；U2 income 100 ref_type=`gift_receive`；两行 ref_id 相同 |
| 8 | `DB` | gift_records 新增 1 行 | sender=U1, receiver=U2, gift_id=G1, count=1, amount=100 |
| 9 | `Redis` | ZSCORE `ranking:charm:day:{today}` U2 | =100 |
| 10 | `Redis` | ZSCORE `ranking:wealth:day:{today}` U1 | =100 |
| 11 | `Android(U1)` | WS | 收到 BalanceUpdated → 余额条 `900 💎` 动画滚动 |
| 12 | `Android(U1/U2/U3)` | WS | 均收到 `GiftReceived` L2 广播 |
| 13 | `Android(U1/U2/U3)` | UI | 公屏出现金色弹幕 "U1 送给 U2 🌹 x1"；U2 主播麦头像出现金色光晕脉冲 2 秒 |
| 14 | `Android(U2)` | 打开"我的" → WalletScreen | 余额大字显示 `100 💎`，流水列表顶部一条绿色 `+100`（礼物打赏） |
| 15 | `AdminWeb` | 访问 U1 详情 → "流水" Tab | 最新一条 expense 100，类型"礼物打赏" |
| 16 | `Android(U1)` | 打开 RankingScreen → 切"财富-日" | 自己头像出现在第 1 位，分数 100 |

**【数据清理】**
- 回滚：余额/魅力/ZSet/wallet_transactions/gift_records。
