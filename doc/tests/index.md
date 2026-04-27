# E2E 测试文档索引

> **负责人**：QA / Infrastructure Team  
> **最后更新**：2026-06-03

---

## 文档清单

| 文档 | 内容 | 关联 Task |
|------|------|----------|
| [MIDSCENE_SETUP.md](./MIDSCENE_SETUP.md) | Midscene LLM 三形态配置指南（OpenAI 直连/Azure/中转）+ GitHub Actions Secret 注入 + 缺 Key 自动 skip + runtime json 脱敏安全规约 | T-0000K |
| [E2E_RUNBOOK.md](./E2E_RUNBOOK.md) | E2E 启动 SOP：冷启动 5 步 + 一键命令矩阵 + preflight 5 端排查表 + staging/prod-safe 凭据流程 + CI Secrets + FAQ | T-0000L |

---

## 快速开始

### 本地 WEB 用例执行（5 分钟）

1. **填入 Midscene Key**：参考 [MIDSCENE_SETUP.md §1](./MIDSCENE_SETUP.md#一-三形态字段冻结表) 选择部署形态，将 API Key 填入 `tests/scripts/env/.env.local`
2. **一键启动**：
   ```bash
   npm run preflight    # 5 端健康检查
   npm run e2e:local    # 运行本地 E2E 用例
   ```
3. **缺 Key 时**：WEB 用例自动 skip，无需修改代码

### CI 环境接入

参考 [MIDSCENE_SETUP.md §3](./MIDSCENE_SETUP.md#三-github-actions-secret-注入与-ci-流程) GitHub Actions Secret 注入示例

---

## 测试用例分类

- **API 用例** (`tests/scripts/API/`): 后端接口功能测试
- **WEB 用例** (`tests/scripts/WEB/`): 前端界面交互测试 + Midscene 辅助
- **Admin WEB 用例** (`tests/scripts/ADMIN_WEB/`): 管理后台界面测试

## 用例编写约定（必读）

> **新增/修改任何 TC-*.md 用例前，先读 [cases/_README.md](./cases/_README.md)**：声明全局隐式前置（preflight + seed + profile）、URL/Token 占位符 → env 字段映射、profile 切换矩阵、作者检查清单。该约定让现有用例无需逐个重写就能受益于模块 9 的多环境基建。

## 模块 9（E2E 测试基建）专项用例

> 模块 9 自身的测试基建（env 模板 / Seed/Reset/Preflight / globalSetup / 多端 config / npm scripts / Midscene / RUNBOOK）作为 CLI/脚本级集成测试，统一收口于 [cases/API/TC-INFRA-E2E.md](./cases/API/TC-INFRA-E2E.md)，共 20 条用例覆盖 T-0000E~L + T-00040 + T-10020 + T-20020 + T-30050。
>
> 模块 0 工程基建（Docker Compose / shared crate / DB 权限 / CI）见 [cases/API/TC-INFRA.md](./cases/API/TC-INFRA.md)。

---

## 关键约定

| 约定 | 说明 |
|------|------|
| **Key 缺失行为** | 仅对 WEB 用例生效：自动 skip 而非 fail；skip reason = `'[MIDSCENE] api key missing — skipped'` |
| **Security** | API Key 永不入 `.e2e-runtime.json`；CI 日志脱敏；错误信息 Key 脱敏 |
| **多环境支持** | local/staging/prod 三档环境，缺 Key 时整套 WEB 用例自动 skip（不影响其他用例） |
