# 测试套件：WALLET 钱包（Android）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-30032（InsufficientBalanceDialog）、WalletScreen（T-30024 入口）、BalanceUpdated 订阅。

---

## TC-WALLET-00001：WalletScreen 展示 + 下拉刷新
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. U1 coin_balance=12345，最近 20 条 wallet_transactions。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 从个人中心点击"💎 钻石余额" | 进入 WalletScreen |
| 2 | `Android` | 顶部大卡片 | 深色卡片，金色大字号显示 `12,345 💎` |
| 3 | `Android` | 下方列表（LazyColumn） | 收入绿色 `+100`，支出红色 `-50`，每行含时间/原因 |
| 4 | `Android` | 下拉列表 | 顶部金色圆形进度圈；服务端返回后余额与流水刷新 |
| 5 | `Android` | 无流水场景 | 列表显示空状态插画 + 文字"暂无交易记录" |

**【数据清理】**
- 无。

---

## TC-WALLET-00002：BalanceUpdated 实时更新
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. U1 WalletScreen 打开中，余额=1000。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 触发 U1 一笔扣款 100 | - |
| 2 | `Android` | 1 秒内 | 顶部余额大数字从 `1,000` 动画滚动到 `900`；流水列表顶部新增一条红色 `-100` |

**【数据清理】**
- 无。

---

## TC-WALLET-00003：充值入口 Phase 1 占位
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Functional`
- **回归级别**：`P2`

**【前置条件】**
1. WalletScreen 打开。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击"充值"按钮 | Toast 显示"充值功能即将上线" |

**【数据清理】**
- 无。

---

## TC-WALLET-00004：InsufficientBalanceDialog (T-30032)
**【元数据】**
- **归属模块**：`WALLET`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. U1 余额=50，在房间内打开礼物面板选择 G1 price=100 count=1（总价 100 > 余额）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 点击"送出"（或面板按钮置灰+强制触发） | 弹出 InsufficientBalanceDialog，标题"余额不足" |
| 2 | `Android` | Dialog 内容 | 显示"当前余额 50 💎  还差 50 💎"，下方两按钮"取消"（灰）/"去充值"（金色） |
| 3 | `Android` | 点击"去充值" | 关闭 Dialog，导航到 WalletScreen |
| 4 | `Android` | 返回礼物面板 | 面板仍保持原选中的礼物与数量 |
| 5 | `Android` | 再次打开 Dialog → 点击"取消" | 关闭 Dialog，保持礼物面板状态 |

**【数据清理】**
- 无。

---

<!-- 🚨 TC-WALLET-00005 / 00006 已下线：
     - 00005 假设 page 参数 1-based + has_more 字段，**未对照** `app/server/src/modules/wallet/` Controller 测试中实际同时出现 page=0 与 page=1 的混乱状况；底部 "已加载全部" 文案 / 弱网失败重试 UI 未读 Compose 真实代码；
     - 00006 假设个人中心"💎 钻石余额"行 + HomeViewModel 监听 BalanceUpdated 流，未读 Android profile/wallet 真实代码，且依赖手工触发服务端充值（env 缺乏直接充值 API）。
     重写计划：先读 `app/server/src/modules/wallet/handlers.rs` + `app/android/.../feature/wallet/` 真实代码与 paging key/字段，再补可执行用例。 -->
