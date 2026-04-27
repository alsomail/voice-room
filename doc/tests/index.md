# E2E 测试文档索引

> **负责人**：QA / Infrastructure Team  
> **最后更新**：2026-06-03

---

## 文档清单

| 文档 | 内容 | 关联 Task |
|------|------|----------|
| [MIDSCENE_SETUP.md](./MIDSCENE_SETUP.md) | Midscene LLM 三形态配置指南（OpenAI 直连/Azure/中转）+ GitHub Actions Secret 注入 + 缺 Key 自动 skip + runtime json 脱敏安全规约 | T-0000K |
| [E2E_RUNBOOK.md](./E2E_RUNBOOK.md) | E2E 启动 SOP：三环境切换命令矩阵、常见故障排查表、CI 接入示例 | T-0000L（待编写） |

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

---

## 关键约定

| 约定 | 说明 |
|------|------|
| **Key 缺失行为** | 仅对 WEB 用例生效：自动 skip 而非 fail；skip reason = `'[MIDSCENE] api key missing — skipped'` |
| **Security** | API Key 永不入 `.e2e-runtime.json`；CI 日志脱敏；错误信息 Key 脱敏 |
| **多环境支持** | local/staging/prod 三档环境，缺 Key 时整套 WEB 用例自动 skip（不影响其他用例） |
