# 测试套件：新用户首次旅程闭环（🚨 已下线，等真实代码摸清后重写）

> **本文件 v1 因虚构内容被下线**：
> - 假设 SMS Provider 固定验证码 123456，实际 [RUNBOOK.md L587](../../../RUNBOOK.md) 明确说"OTP 每次随机生成"，需 `redis-cli HSET sms:code:<phone> code 123456` 显式注入；
> - 假设注册成功后由 DB trigger 给新用户充值 1000 钻石，**项目无此 trigger**；
> - 引用了 `events` 表 + AdminWeb "行为流" Tab + 导出 CSV 等多处未验证存在的功能；
> - 提到 `is_new=true` 字段、`consent_full` 事件、`gift_send_success` 事件等，均未对照真实代码核实。
>
> **重写计划**：将由 `E2E/TC-MAIN-FLOW.md` 重新承担「登录 → 大厅 → 进房 → 上麦 → 送礼」串联主流程，前置条件全部映射到 RUNBOOK §11 已存在的 seed 资源（User A/B + super_admin）与 redis-cli HSET 显式注入。

<!-- 历史 v1 内容已废弃，禁止参照执行 -->

