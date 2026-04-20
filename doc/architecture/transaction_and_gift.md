# 9. 商业化强一致性与送礼事务

送礼是核心收入链路，必须使用 SQLx 数据库事务实现强一致性。

## 9.1 事务边界

一次送礼必须在同一事务内完成：
1. 校验房间、用户、礼物合法性
2. 锁定送钱用户钱包余额
3. 扣除金币
4. 增加主播/房间收益
5. 写入礼物订单
6. 写入钱包流水
7. 写入收益账单
8. 提交事务

禁止事项：
- 先扣费再异步写流水
- 扣费成功但收益写入失败
- 使用客户端结果作为扣费依据
- 使用 WS 广播成功作为事务完成标志

## 9.2 推荐表结构

- `wallet_account`
- `wallet_ledger`
- `gift_catalog`
- `gift_order`
- `anchor_income_account`
- `income_ledger`
- `billing_statement`
- `transaction_outbox`

## 9.3 广播时机

- 必须先提交事务，再广播 `GIFT_SENT`
- 可采用 Transactional Outbox 解耦广播

## 9.4 幂等保护

- `gift_order.request_id` 唯一
- `wallet_ledger.biz_id` 唯一
- 同一 `request_id` 重试直接返回既有结果
