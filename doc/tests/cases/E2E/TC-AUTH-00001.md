# 测试套件：E2E 端到端 - 登录闭环（Android + AppServer + DB）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

---

## TC-AUTH-00013：新用户 E2E 注册登录闭环
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Integration`
- **回归级别**：`P0`

**【前置条件】**
1. DB 无 phone=`+966500000500` 的用户。
2. SMS Provider 为 Mock，固定验证码 123456。
3. App 全新安装，无 JWT。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Android` | 启动 App，Splash 后进入 LoginScreen | 登录页显示 |
| 2 | `Android` | 手机号框输入 `500000500` → 点击"获取验证码" | 按钮倒计时 60s |
| 3 | `AppServer` | 日志 | POST `/api/v1/auth/verification-codes` 200 |
| 4 | `Redis` | `GET sms:code:+966500000500` | 存在 6 位数字，TTL<=300 |
| 5 | `Android` | 验证码框输入 `123456` → 点击"登录" | 按钮 Loading |
| 6 | `AppServer` | 日志 | POST `/api/v1/auth/login` 200，返回 token + is_new=true |
| 7 | `DB` | `SELECT id,coin_balance FROM users WHERE phone='+966500000500'` | 存在 1 行，coin_balance=0 |
| 8 | `Android` | 2s 内跳转 MainScreen | 大厅 Tab 显示房间列表 |
| 9 | `Android` | 进入"我的" Tab | 显示昵称（自动生成 User_xxxx）、ID、余额 0 |
| 10 | `Android` | 冷启 App | Splash 800ms 后直接进 MainScreen，不回 LoginScreen（JWT 持久化） |

**【数据清理】**
- DELETE users WHERE phone='+966500000500'。
- 清 Redis 相关键。
