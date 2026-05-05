# 协议路径绑定审计工具 (Protocol Binding Audit)

> **版本**: v1.0 | **Task**: T-0000T | **脚本**: `scripts/audit/protocol-binding-audit.ts`

## 🔌 协议入口索引

> N/A — 本工具为纯基础设施审计脚本，无跨端协议路径。
> 脚本本身读取 `doc/protocol/index.md` 作为协议锚点参考，不新增任何 HTTP REST / WebSocket 通信入口。
>
> 协议源文件：[doc/protocol/index.md](../../protocol/index.md)

## 脚本契约

### 输入

| 输入项 | 来源 | 说明 |
|--------|------|------|
| TDS 文件 | `doc/tds/**/T-*.md` | 自动扫描所有 TDS，提取第二节「🔌 协议路径绑定表」 |
| Server 源码 | `app/server/src/` | grep `Router::route` / `match.*envelope.*r#type` |
| Android 源码 | `app/android/` | grep `wsClient.send` / `.sendEnvelope` / Retrofit `@POST` |
| Web 源码 | `app/web/src/` | grep `apiClient.*` |

### 输出

| 输出项 | 路径 | 说明 |
|--------|------|------|
| JSON 报告 | `tests/protocol-audit/report-YYYY-MM-DD.json` | 结构化审计结果 |
| Markdown 报告 | `tests/protocol-audit/report-YYYY-MM-DD.md` | 人类可读报告 |

### 退出码

| 退出码 | 含义 |
|--------|------|
| `0` | 无 P0 错误（可能有 P1/P2 警告）|
| `1` | 存在 P0 错误（CI 门禁阻断合并）|

## 核心函数

| 函数 | 签名 | 职责 |
|------|------|------|
| `parseBindingTable` | `(content: string, path: string) → ProtocolBinding[]` | 解析 TDS 绑定表（Markdown 表格 + N/A 识别）|
| `auditBindings` | `(bindings, serverGrep, clientGrep) → AuditFinding[]` | 三方对账，生成 P0/P1/P2 发现 |
| `generateReport` | `(findings, meta) → AuditReport` | 生成结构化报告 |
| `main` | `() → Promise<void>` | 入口函数，`--dry-run` 模式不写文件、不 exit(1) |

## 使用方法

```bash
# 正常运行（会写报告文件，P0 时 exit 1）
npm run audit:protocol

# Dry-run 模式（仅输出摘要，不写文件）
npm run audit:protocol -- --dry-run

# CI 接入（已配置在 .github/workflows/）
# PR 触发 audit:protocol，P0 时阻断合并
```

## P0/P1/P2 分级规则

| 级别 | 触发条件 | 对 CI 影响 |
|------|---------|-----------|
| **P0** | Server 实现未找到 / Client 调用未找到 | **阻断合并** |
| **P1** | Protocol 锚点失效 / 信令名不匹配 | 警告（不阻断）|
| **P2** | 绑定表格式不标准 / 路径信息不完整 | 信息（不阻断）|

## 另见

- [协议契约总索引](../../protocol/index.md)
- [TDS 模板](../../tds/_template.md) — 「🔌 协议路径绑定表」格式规范
- [审计脚本源码](../../../scripts/audit/protocol-binding-audit.ts)
- [测试文件](../../../scripts/audit/__tests__/protocol-binding-audit.test.ts)
