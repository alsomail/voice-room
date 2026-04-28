# E2E 启动 SOP（E2E_RUNBOOK）

> **负责人**：QA / Infrastructure Team  
> **真值源**：[`package.json:scripts`](../../package.json) · [`playwright.config.ts`](../../playwright.config.ts) · [`docker-compose.yml`](../../docker-compose.yml) · [`tests/scripts/env/.env.local.example`](../../tests/scripts/env/.env.local.example)  
> **关联 Task**：T-0000L（本文档）/ 上游 T-0000E/F/G/H/I/J/K  
> **目标**：新人从 `git clone` 到 `npm run e2e:local` 首条 smoke 用例全绿 **≤ 5 分钟**，单文档闭环。

---

## §1 前置依赖

| 项 | 要求 | 自检命令 |
|---|---|---|
| Node.js | `>= 20.x`（与 `playwright@^1.59` 兼容） | `node --version` |
| npm | `>= 10.x` | `npm --version` |
| Docker | `>= 24.x`（含 Compose v2） | `docker --version && docker compose version` |
| Rust toolchain | 由 `rust-toolchain.toml` 自动锁定 | `rustup show` |
| Android SDK | API 34（仅跑 Android E2E 时） | `sdkmanager --list` |
| 操作系统 | macOS / Linux 一等公民；Windows 走 WSL2 或 Git Bash + `cross-env` | — |

> **时长预算（首次冷启动 vs 复跑）**：
> - **首次冷启动 8~18min**：`npm install` ≈ 2min（依网速）+ `cargo build --workspace` 首次 5~15min（Rust 工作区冷编译）+ docker 拉镜像 ≈ 30s + 服务起齐 ≈ 30s + smoke ≈ 30s。
> - **复跑 ≤ 5min**：`npm install` 命中缓存 ≤ 5s + cargo 增量 ≤ 30s + 服务起齐 ≈ 30s + 首条 smoke ≈ 30s ⇒ ≤ 5 分钟。
> - **加速建议**：把 `cargo build --workspace` 作为「准备工作」前置（详见 §2 设计取舍），把 `5min` 预算严格收口到「服务起齐 → 首条 smoke 全绿」段。

---

## §2 冷启动 5 步（local 全量）

> 严格编号、复制即用；每步独立可重入。

> **设计取舍（缺陷 6 修复）**：本仓库 `docker-compose.yml` **仅托管 Postgres + Redis** 两个有状态依赖；
> 业务服务（AppServer / AdminServer / Web）在 dev 期一律用 `cargo run` / `vite` 本地起，
> 便于热重载与断点调试。如需「一键起全栈」聚合，使用 `npm run e2e:up`（详见 §3 命令矩阵）。
> 该取舍意味着：纯 `docker compose up` 不会拉起 5 端，请按下方 Step 3+4 顺序启动，或直接 `npm run e2e:up`。

> **预热建议（缺陷 4 修复）**：首次冷启动前先单独跑一次 `cargo build --workspace`（5~15min，依机器），
> 把 Rust 编译耗时移出「5min 预算」窗口；之后 `cargo run -p server` 才能秒级启动。

```
1. git clone + 安装依赖
   $ git clone <repo-url>
   $ cd voice-room
   $ npm install                         # 根 + tests/scripts 依赖一并装好
   $ cargo build --workspace             # 推荐预热（首次 5~15min；后续增量秒级）

2. 复制三档 env 模板（local 端）
   $ cp tests/scripts/env/.env.local.example tests/scripts/env/.env.local
   #   staging / prod 端凭据流程见 §5（不在冷启动 5 步内）

3. 启动 docker 依赖（Postgres :5432 + Redis :6379）
   $ docker compose up -d postgres redis

4. 启动业务服务（按需开 3 个终端，对应 5 端中的 3 个进程端）
   终端 A:  APP_PROFILE=dev   cargo run -p server          # AppServer  → :3000
   终端 B:  ADMIN_PROFILE=dev cargo run -p admin-server    # AdminServer → :3001
   终端 C:  npm --prefix app/web run dev                   # Web         → :5173
   # 或一键聚合（缺陷 6 修复）：
   $ npm run e2e:up                                        # 等价于 docker up + 3 服务后台 + wait-on

5. 跑 E2E（首次推荐 smoke 子集 ≈ 30s 验证链路）
   $ npm run preflight                              # 5 端健康检查 ≤ 1s
   $ npm run e2e:local -- --list                    # 列出 local 全量用例（dry-run，不真跑）
   $ npm run e2e:local -- --grep "@prod-safe"       # 仅跑 @prod-safe smoke 子集（最小验证链路）
   $ npm run e2e:local                              # 本地全量 ≈ 5min
   # 注：`npm run e2e:prod-smoke` 需要已配 `.env.prod`（含 7 项 token），见 §5 远端凭据流程。
```

> **5 端口字面备忘**：AppServer `3000` / AdminServer `3001` / Web `5173` / Postgres `5432` / Redis `6379`（与 `docker-compose.yml` 端口映射严格对齐）。如本地存在 8080/8081 反向代理或 IDE 旧端口残留，请按 §4 排查。

---

## §3 一键命令矩阵（与 `package.json:scripts` 严格对账）

> **真值源**：[`package.json:scripts`](../../package.json)（T-0000I 冻结）。本表命令名与 scripts 1:1 相等；如不一致以 `package.json` 为准并修订本表。

| 命令 | 用途 | 前置 | 时长 | 来源契约 |
|---|---|---|---|---|
| `npm run preflight` | 5 端健康检查（Postgres/Redis/AppServer/AdminServer/Web） | docker + 服务起齐 | ≤ 1s | T-0000I §2.3 / T-0000G `scripts/dev/preflight.sh` |
| `npm run e2e:up` | 一键起全栈（docker postgres+redis → cargo server → cargo admin-server → vite web → wait-on 五端健康） | npm install + cargo build 预热 | 首次 5~15min（cargo 冷编译），复跑 ≤ 30s | 缺陷 6 修复（batch-e2e-foundation-01 第 1 轮） |
| `npm run e2e:local` | 本地全量 E2E（API + WEB + ADMIN_WEB） | 5 端启动 + `tests/scripts/env/.env.local` | ≈ 5 min | T-0000I §2.3 |
| `npm run e2e:staging` | staging 全量 E2E（远端） | `tests/scripts/env/.env.staging` 凭据 | — | T-0000I §2.3（凭据见 §5） |
| `npm run e2e:prod-smoke` | prod 仅 `@prod-safe` 标签子集（read-only smoke） | `tests/scripts/env/.env.prod` 凭据 | — | T-0000I §2.3 + T-0000J §2.4 |
| `npm run db:seed` | 注入 E2E 测试数据（local profile） | local profile + Postgres 起 | ≤ 5s | T-0000I §2.3 / T-0000G |
| `npm run db:reset` | 清空 E2E 测试数据（local profile） | local profile | ≤ 3s | T-0000I §2.3 / T-0000G |

> **防幻觉清单（不存在的命令）**：~~`npm run e2e:smoke`~~（请用 `npm run e2e:prod-smoke` 或 `npm run e2e:local -- --grep "@prod-safe"`）/ ~~`npm run e2e:prod-safe`~~（请用 `npm run e2e:prod-smoke`）/ ~~`npm run test`~~（顶级未注册）。

---

## §4 preflight 5 端故障排查表

> **结构**：端 / 症状 / 检查命令 / 修复指南 / preflight 退出码（来自 T-0000G §2.6 退出码 11~15 + envLoader 78）。

| # | 端（端口） | 症状 | 检查命令 | 修复指南 | rc |
|---|---|---|---|---|---|
| 1 | Postgres `:5432` | 连接拒绝 / `pg_isready` 失败 | `pg_isready -h 127.0.0.1 -p 5432` | `docker compose up -d postgres`；端口被占 → `lsof -nP -iTCP:5432` 杀进程或改 `.env` `DATABASE_URL` 端口 | 11 |
| 2 | Redis `:6379` | `PING` 不通 / NOAUTH | `redis-cli -h 127.0.0.1 -p 6379 PING`（带 AUTH 加 `-a ***`） | `docker compose up -d redis`；AUTH 失败检查 `.env` `REDIS_URL` 密码段 | 12 |
| 3 | AppServer `:3000` | `/health` 非 200 | `curl -fsS http://127.0.0.1:3000/health` | 终端 A `APP_PROFILE=dev cargo run -p server`；exit 78（CONFIG ERROR）→ 检查 `JWT_SECRET / DATABASE_URL` 是否齐全 | 13 |
| 4 | AdminServer `:3001` | `/health` 非 200 | `curl -fsS http://127.0.0.1:3001/health` | 终端 B `ADMIN_PROFILE=dev cargo run -p admin-server`；exit 78 → 缺关键字段 | 14 |
| 5 | Web `:5173` | 端口不通 | `curl -fsS http://127.0.0.1:5173/` | 终端 C `npm --prefix app/web run dev`；端口冲突 → `lsof -nP -iTCP:5173`（注意区别于 8080 / 8081 反向代理端口） | 15 |
| 6 | envLoader（fail-fast） | 启动期 `MissingEnvError: <字段名>` / `[CONFIG ERROR]` | `npm run preflight 2>&1 \| head -20` | `.env.local` 缺字段 → 对照 `tests/scripts/env/.env.local.example` 补齐；**禁止** `process.env.X ?? '默认值'` 兜底（T-0000J 已禁） | 78 |
| 7 | Midscene（WEB 用例） | 日志显示 `[MIDSCENE] api key missing — skipped` | `grep MIDSCENE_MODEL_API_KEY tests/scripts/env/.env.local` | 缺 Key 行为符合预期（自动 skip）；如需跑 WEB 用例，参考 [`./MIDSCENE_SETUP.md`](./MIDSCENE_SETUP.md) §1 §4.1 注入 Key | skip |
| **11** | **端口冲突预检（5 端）** | **`docker compose up` 启动时报 `Bind for 0.0.0.0:<port> failed: port is already allocated`** | **`bash scripts/dev/check-ports.sh`（可单独执行，输出占用进程 PID/名称）** | **e2e-up.sh 已集成预检（Step 0）；若手动启动，冲突时运行 `kill -9 <PID>`；详细排查步骤见 [T-0000Q TDS](../tds/infra/T-0000Q.md) §五** | 1 |

> **退出码备忘**：rc 11~15 = preflight 5 端；rc 78 = `EX_CONFIG`（envLoader 与 Rust config 共享语义，T-0000E §4.3 / T-00040 / T-10020）。

> **baseURL 双 key fallback（来自 T-0000J §2.4）**：globalSetup 同时写入 `ADMIN_WEB_URL` 与 `_E2E_RUNTIME_ADMIN_WEB_URL` 双 key，`playwright.config.ts` 在 `use.baseURL` 处取 `process.env._E2E_RUNTIME_ADMIN_WEB_URL ?? process.env.ADMIN_WEB_URL`。`.e2e-runtime.json` 残留参见 §7 FAQ Q6。

---

## §5 staging / prod-safe 远端凭据流程

> **占位 SOP**（具体凭据渠道由团队 SRE / QA Lead 落地时填入）：

1. **凭据获取**
   - 1Password 团队保险库 `voice-room-e2e` → `staging-runner` 条目；或开 SRE 工单（标签 `e2e/credentials`）申请；
   - 凭据字段（参考 T-0000F §2.3 + T-0000K §1）：`DATABASE_URL` / `REDIS_URL` / `E2E_VALID_TOKEN` / `E2E_ADMIN_TOKEN` / `MIDSCENE_MODEL_API_KEY`（如启用 WEB 用例）等；
   - **绝对禁止**：`echo` 到终端、粘贴聊天工具、commit 进 Git、贴入本 RUNBOOK 示例。

2. **写入 env**
   ```bash
   cp tests/scripts/env/.env.staging.example tests/scripts/env/.env.staging
   # 用编辑器（vim / VSCode）填入 <placeholder> 字段；占位形如 <YOUR_TOKEN_HERE> 或 ***
   ```

3. **跑用例**
   ```bash
   # macOS / Linux / WSL2 / Git Bash
   npm run e2e:staging
   # 仅跑 @prod-safe 标签（read-only smoke 验证远端连通性）
   npm run e2e:staging -- --grep "@prod-safe"
   ```

4. **prod-safe 子集（生产环境只读探针）**
   ```bash
   cp tests/scripts/env/.env.prod.example tests/scripts/env/.env.prod
   npm run e2e:prod-smoke
   ```

> **⚠️ Windows shell 双引号警示**
> - PowerShell / cmd.exe 下 `--grep '@prod-safe'`（**单引号**）会被解释为字面量 `'@prod-safe'`（含引号本身），**永远 0 命中** → 静默通过假绿；
> - 统一使用 **双引号** `--grep "@prod-safe"`，与 `package.json` `e2e:prod-smoke` 字面 `\"@prod-safe\"` 严格对齐；
> - 推荐 Windows 用户走 **WSL2 / Git Bash** 规避 shell 差异。

> **🔒 安全红线**（T-0000E §2.9 + T-0000K §3.1）：
> - `tests/scripts/env/.env.{local,staging,prod}` 全量 gitignore，永不入库；
> - 凭据永不写入 `tests/scripts/.e2e-runtime.json`（T-0000K `sanitizeEnvForRuntimeJson` 已强制脱敏 `midscene.apiKey`）；
> - 错误日志中 Key 自动 `***` 替换（envLoader 已实现）。

---

## §6 CI 引用（GitHub Actions Secrets）

> **必备 Secrets 清单**（CI workflow 引用 `${{ secrets.<NAME> }}`，禁止明文）：

| Secret 名 | 用途 | 来源契约 |
|---|---|---|
| `MIDSCENE_MODEL_API_KEY` | Midscene LLM API Key | T-0000K §3 |
| `MIDSCENE_OPENAI_BASE_URL` | OpenAI 中转 / Azure 部署形态 | T-0000K §1（形态 B/C） |
| `AZURE_OPENAI_ENDPOINT` / `AZURE_OPENAI_DEPLOYMENT` / `AZURE_OPENAI_API_VERSION` | Azure 形态 | T-0000K §1 形态 B |
| `E2E_BASE_URL_STAGING` / `E2E_BASE_URL_PROD` | 远端 baseURL（双 key fallback 起点） | T-0000F §2.3 + T-0000J |
| `E2E_VALID_TOKEN` / `E2E_ADMIN_TOKEN` | 受测身份 token | T-0000G §2.5 + T-0000F §2.3 |
| `DATABASE_URL_STAGING` / `REDIS_URL_STAGING` | 远端 staging 直连（如启用） | T-0000F §2.3 |

> **完整 yaml 注入示例**：参见 [`./MIDSCENE_SETUP.md`](./MIDSCENE_SETUP.md) §3（`${{ secrets.MIDSCENE_MODEL_API_KEY }}` 引用 + `set -x` 警示已冻结）。

> **Android E2E 注入路径（缺陷 2 修复，batch-e2e-foundation-01 第 1 轮）**：
>
> ```
> 根 .env.{profile}  →  envLoader.loadE2EEnv  →  writeProcessEnv
>                                                      │
>                                                      ▼
>                       process.env.VOICE_ROOM_API_BASE_URL / WS_URL / ANALYTICS_ENDPOINT
>                                                      │
>                                                      ▼
>                       gradlew (resolveConfigValue 读 env)  →  BuildConfig.{API_BASE_URL, WS_URL, ANALYTICS_ENDPOINT}
> ```
>
> - 三档 flavor (`local` / `staging` / `prod`) 全部通过 `resolveConfigValue` 读 env，无任何字面 URL 硬编码；
> - 切换 profile 仅需 `cp tests/scripts/env/.env.staging.example .env.staging` 并 `npm run e2e:up` 启 E2E 链路，gradlew 自动从 process.env 拾取；
> - 也可显式覆盖：`VOICE_ROOM_API_BASE_URL=https://my.example.com/api ./gradlew :app:assembleStagingDebug`；
> - 默认值（`https://stg-api.example.com/api` 等）仅作 0 回归占位，不应作为生产入口。
> - 验证命令：`./gradlew :app:assembleStagingDebug` 在仅 env 切换、未改 Kotlin 源码的前提下产出新 staging APK。

---

## §7 故障 FAQ

### Q1. 端口被占（5173 / 3000 / 3001 / 5432 / 6379 / 8080 / 8081）

`lsof -nP -iTCP:<port>` 找占用进程；杀进程或改 `.env` 对应字段；docker 起的服务用 `docker compose down` 全部回收后重启。

### Q2. Docker 拉镜像慢 / 超时

配置国内镜像源（`/etc/docker/daemon.json` `registry-mirrors`），或预先 `docker pull postgres:16-alpine redis:7-alpine`。首次冷启可能突破 5 分钟预算（与网速强相关）。

### Q3. envLoader fail-fast `MissingEnvError` / exit 78（CONFIG ERROR）

对照 `tests/scripts/env/.env.local.example` 补齐缺失字段；**切勿**用 `process.env.X ?? '默认值'` 绕过（T-0000J U-3/U-4 已禁止该模式，单测会零容忍报错）。

### Q4. Windows shell 单引号假绿（`--grep '@prod-safe'` 0 命中静默通过）

PowerShell / cmd.exe 单引号不剥离，导致 `--grep` 收到字面 `'@prod-safe'` ⇒ 0 命中且**静默退出码 0**（看似全绿）。统一改用 **双引号** `--grep "@prod-safe"`，详见 §5。推荐 Windows 用户切换至 **WSL2 / Git Bash**。

### Q5. Midscene 401 / 429 / 超时 / 限流

参考 [`./MIDSCENE_SETUP.md`](./MIDSCENE_SETUP.md) §5 限流回退表 + §6 FAQ。**Key 配额耗尽不会自动 skip**（避免静默通过），需更换 Key 或减并发；缺 Key 才会按 T-0000K 策略对 WEB 用例自动 skip。

### Q6. `.e2e-runtime.json` 脏残留导致 worker 复用旧值

删除 `tests/scripts/.e2e-runtime.json` 重跑；该文件由 globalSetup 写入（T-0000H），**不应**手工编辑或 commit（已 gitignore）。残留典型表现：切换 profile 后 `_E2E_RUNTIME_ADMIN_WEB_URL` 仍指向旧 baseURL。

### Q7. `@prod-safe` 标签 0 命中（拼写漂移）

检查 fuzzy 拼写（`@prod_safe` / `@prodsafe` / `@prod-save`，T-0000J U-11 守护）；统一字面 `@prod-safe`（中划线 + 全小写）。

### Q8. 首次 `npm install` 慢

跨平台依赖 `cross-env` / `@playwright/test` / `@midscene/web` 体积较大；建议 `npm config set registry https://registry.npmmirror.com` 切换镜像。

### Q9. `cargo test -p voice-room-server` 中 `r08_response_time_under_100ms` perf flake

已登记为 known-issue，默认 `#[ignore]` 跳过；详情、手动跑命令与长期方向见 [`./known-issues.md#r08`](./known-issues.md#r08)。

---

## §7.5 特殊角色 Token / redis-cli 自动注入说明（T-0000S）

> 为消除 API 套件长期 26 个 SKIP-KNOWN（"环境/fixture 缺失"而非逻辑漏洞），seed-e2e.sh + globalSetup 已自动注入 USER_B / MUTED 两个特殊角色 token 与容器化 `redis-cli` 调用入口。开发者**无需任何额外配置**即可让原 SKIP 用例默认跑通。

### 三个特殊角色 token 的构造与作用

| Token 环境变量 | sub（uuid5(name) E2E ns） | iss / role | 用途 | 注入路径 |
|---|---|---|---|---|
| `E2E_VALID_TOKEN` | `uuid5(user_a)` = `98026d7e-...` | `voiceroom` / 普通用户 | 通用合法 C 端 token | seed-e2e.sh → .seed-output.env |
| `E2E_USER_B_TOKEN` | `uuid5(user_b)` = `584a89d8-...` | `voiceroom` / 普通用户 | 第二个普通用户，用于"双人房"场景：TC-MIC（4 用例）/ TC-CHAT-00001 / TC-GIFT 双向送礼 | seed-e2e.sh →  .seed-output.env（**T-0000S 新增**） |
| `E2E_MUTED_TOKEN` | `uuid5(user_muted)` = `f1ff1b29-...` | `voiceroom` / 普通用户 | 已禁言用户，用于 TC-CHAT-00004（CHAT_MUTED 40303） | seed-e2e.sh → .seed-output.env（**T-0000S 新增**） |
| `E2E_OP_TOKEN` | `uuid5(admin_op)` | `voiceroom-admin` / `operator` | Admin 运营角色 token | 已由 seed-e2e.sh 注入（T-0000G） |

> **禁言态**真值并非 token 内 `muted` claim（AppClaims 不含此字段），而是 Redis key `chat_muted:{ROOM_ID}:{USER_MUTED_ID}` 的存在性。`scripts/dev/seed-e2e.sh` 末尾通过 `docker exec vr-redis redis-cli SETEX chat_muted:... 86400 1` 写入；TTL 与 token 寿命一致（24h），重跑 seed 即续期。

### redis-cli 容器化（docker exec → native fallback）

API 套件中 TC-AUTH（13）/ TC-WS（3）/ TC-RANKING（1）/ TC-GIFT（1）共 18 个用例需要直接读写 Redis。原实现要求宿主 `brew install redis` 把 `redis-cli` 装到 PATH，否则全部 `SKIP-KNOWN`。T-0000S 改为：

1. **优先** `docker exec vr-redis redis-cli ...`（与 `docker-compose.yml` `container_name: vr-redis` 对接，零额外依赖）；
2. **回退** 宿主 PATH 中的 `redis-cli`（保留传统路径）；
3. 都不可用时 `isRedisCliAvailable() === false` → 用例打 `SKIP-KNOWN-FOLLOWUP`（不再失败）。

实现入口：[`tests/scripts/support/redisCli.ts`](../../tests/scripts/support/redisCli.ts)。`globalSetup` 启动时会日志记录当前 mode（`docker` / `native` / `unavailable`）。

### 解锁进度

- **本 Task 直接解锁**：26 / 29 SKIP-KNOWN 用例（USER_B + MUTED + redis-cli 容器化覆盖区）。
- **剩余 3 个**由 follow-up T-0000T 收口：
  - TC-INFRA-00001 / TC-INFRA-00002（Docker 控制权限：要求 Playwright 进程能 `docker stop vr-postgres / vr-redis` 模拟服务挂掉）
  - TC-INFRA-Q-I-2（干净端口环境：要求 5432/6379 未被占用以验证 preflight 报错）



## §8 附录：相关文档锚点表

| 主题 | 文档锚点 |
|---|---|
| E2E 多环境总设计 | [T-0000E TDS](../tds/infra/T-0000E.md) §2.4 字段冻结 / §2.6 preflight / §2.11 迁移路径 |
| 三档 env 模板 | [T-0000F TDS](../tds/infra/T-0000F.md) §2.3 |
| envLoader / fixtures / globalSetup | [T-0000H TDS](../tds/infra/T-0000H.md) §2.5 §2.6 |
| npm scripts 一键命令 | [T-0000I TDS](../tds/infra/T-0000I.md) §2.3 |
| baseURL 双 key fallback + @prod-safe 标签 | [T-0000J TDS](../tds/infra/T-0000J.md) §2.4.1 五条 read-only 判定 |
| Midscene LLM 三形态配置 | [`./MIDSCENE_SETUP.md`](./MIDSCENE_SETUP.md) §1 §3 §4 §5 §6 |
| 文档总索引 | [`./index.md`](./index.md) |
