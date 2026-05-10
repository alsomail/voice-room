# 测试套件：Web Dashboard（🚨 已下线，等真实代码摸清后重写）

> **本文件 v1 因虚构内容被下线**：
> - 设计了 `GET /admin/stats/timeseries` 但 AdminServer **不存在**该接口（仅 `/stats/overview`）。
> - 设计了 7d/30d 时间窗切换、ECharts tooltip，但 `overview.trend` 字段已被改为 optional（详见 [doc/review/QA回归遗留改动审查.md L150](../../review/QA回归遗留改动审查.md)）。
>
> **重写计划**：等主流程套件 `E2E/TC-MAIN-FLOW.md` 落地后，按真实代码（仅 `/stats/overview` 单接口、StatCards 渲染、自动刷新与组件卸载取消）重新发散。

<!-- 历史 v1 内容已废弃，禁止参照执行 -->

