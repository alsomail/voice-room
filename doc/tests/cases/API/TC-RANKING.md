# 测试套件：RANKING 榜单（API）

> **需求模糊点 (Ambiguity Notes)**：
> - 时区按 UTC+3（沙特/麦加时间）计算 day/week 键。

覆盖 Task：T-00019（榜单查询 API）。

---

## TC-RANKING-00001：GET /ranking - 参数矩阵
**【元数据】**
- **归属模块**：`RANKING`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. Redis ZSet `ranking:charm:day:{today}` 内有 5 条记录 (U1..U5 分数 500..100 递减)。
2. TOKEN_U1 有效。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/api/v1/ranking?type=charm&period=day&limit=3` Bearer TOKEN_U1 | 200，items 长度=3；items[0].user_id=U1 score=500 rank=1；items[1].medal=`silver`；items[2].medal=`bronze` |
| 2 | `AppServer` | items[0].medal | `gold` |
| 3 | `AppServer` | GET `type=wealth&period=week&limit=50` | 200，items ≤50，按分数降序 |
| 4 | `AppServer` | GET `type=charm&period=day&limit=100` | 200 |
| 5 | `AppServer` | GET `limit=101`（超限） | 400 code=40003 |
| 6 | `AppServer` | GET `limit=0` | 400 code=40003 |
| 7 | `AppServer` | GET `type=unknown` | 400 |
| 8 | `AppServer` | GET `period=month` | 400（不支持） |
| 9 | `AppServer` | 未带 token | 401 |

**【数据清理】**
- 无。

---

## TC-RANKING-00002：GET /ranking - me.rank 未上榜时为 null
**【元数据】**
- **归属模块**：`RANKING`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. U_X 不在 ZSet 中（score=0 或未出现）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | GET `/ranking?type=charm&period=day` Bearer TOKEN_U_X | 200，`data.me.rank=null`，`data.me.score=0` |
| 2 | `AppServer` | 往 ZSet 加入 U_X score=50 | `data.me.rank` 变为实际排名 |

**【数据清理】**
- 移除 ZSet 中的 U_X。

---

## TC-RANKING-00003：性能 - p95 ≤100ms
**【元数据】**
- **归属模块**：`RANKING`
- **测试类型**：`Performance`
- **回归级别**：`P1`

**【前置条件】**
1. ZSet 预填 10 万条数据。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AppServer` | 100 QPS 压测 30s | p95 响应时间 ≤100ms，错误率 0% |

**【数据清理】**
- 清理压测数据。

---

## TC-RANKING-00004：日/周轮转 - 归档键
**【元数据】**
- **归属模块**：`RANKING`
- **测试类型**：`Functional`
- **回归级别**：`P2`

**【前置条件】**
1. 可操控系统时间或 Redis 键名的日期段。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Redis` | 到达 UTC+3 00:00 时刻 | `ranking:charm:day:{yesterday}` 保留为归档；新的 `ranking:charm:day:{today}` 从空开始写入 |
| 2 | `AppServer` | 昨日 00:05 查询 `period=day` | 返回今日键的数据（目前为空/新） |
| 3 | `Redis` | 周日 24:00 到周一 00:00 | `ranking:charm:week:{last_iso_week}` 归档保留 |

**【数据清理】**
- 归档键按产品保留策略清理。
