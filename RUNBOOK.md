# Voice Room — 运维 / 部署 RUNBOOK

> Monorepo：Rust AppServer + Rust AdminServer + React AdminWeb + Kotlin Android。
> 本文档是**手动运维**的单一入口，包含本地 / Staging / Prod 三套环境的启动、状态查看、停服、故障排查。

---

## 0. TL;DR — 我只想跑起来

```bash
# 第一次：克隆后的依赖
npm install
cargo build --workspace        # 可选：预热 Rust 编译

# 智能启动（已运行的服务自动跳过）
npm run start

# 看状态
npm run status

# 停服
npm run stop
```

| 命令              | 行为                                                                 |
| ----------------- | -------------------------------------------------------------------- |
| `npm run start`   | 智能启动 PG/Redis/AppServer/AdminServer/Web，已健康者跳过           |
| `npm run status`  | 10 项状态总览（Docker/容器/PG/Redis/三端 HTTP/PID）                  |
| `npm run stop`    | 按 `.e2e-up.pids` 停业务进程，docker 容器保留                        |
| `npm run e2e:up`  | **严格模式**：端口被占用即 fail（CI/干净环境用）                     |
| `npm run preflight` | fail-fast 健康检查，10s 内出结果                                   |
| `npm run db:seed` | 灌 E2E 测试种子数据                                                   |
| `npm run db:reset`| 清理 E2E 测试数据                                                     |

---

## 1. 端口与服务全景

| 服务         | 端口 | 进程/容器                         | 健康端点                 |
| ------------ | ---- | --------------------------------- | ------------------------ |
| PostgreSQL   | 5432 | `vr-postgres` (docker)            | `pg_isready`             |
| Redis        | 6379 | `vr-redis` (docker)               | `redis-cli ping → PONG`  |
| AppServer    | 3000 | `cargo run -p voice-room-server`  | `GET /health → 200`      |
| AdminServer  | 3001 | `cargo run -p voice-room-admin-server` | `GET /health → 200` |
| AdminWeb     | 5173 | `vite` (npm run dev)              | `GET / → 200/301/302`    |

**默认 Docker 容器名**：`vr-postgres` / `vr-redis`（见 `docker-compose.yml`）。

---

## 2. 本地环境（local / dev）

### 2.1 前置依赖

| 工具      | 最低版本 | 验证命令               |
| --------- | -------- | ---------------------- |
| Docker    | 24.x     | `docker info`          |
| Node      | 20.x     | `node -v`              |
| Rust      | 见 `rust-toolchain.toml` | `cargo --version` |
| psql / nc | 任意     | `psql --version`       |

### 2.2 第一次启动

```bash
git clone <repo>
cd voice-room
npm install
```

可选：把根目录 `.env.example` 复制为 `.env` 并按需修改密码、JWT 密钥等。

### 2.3 启动

**推荐：智能启动（重复调用安全）**

```bash
npm run start
# 等价：bash scripts/dev/start-local.sh
```

特性：
- 已健康的服务自动跳过（不会重复启动）
- 缺失的部分启动后写入 `.e2e-up.pids`
- 全部就绪前阻塞最多 180 s

参数：

```bash
bash scripts/dev/start-local.sh --no-web        # 仅起后端三件套
bash scripts/dev/start-local.sh --skip admin    # 跳过 AdminServer
bash scripts/dev/start-local.sh --force         # 先 stop 再 up
```

**严格启动（CI / 干净盘）**

```bash
npm run e2e:up
# 端口被占用 → 直接 fail（适合 CI 保证干净）
```

### 2.4 状态查看

```bash
npm run status            # 人类友好输出
npm run status -- --json  # 机器可读 JSON
```

输出 10 项：
- `docker` — Docker daemon 是否运行
- `pg_container` / `redis_container` — 容器运行 + 健康状态
- `pg_tcp` / `pg_db` — PG TCP 可达 + voiceroom 业务库可连接
- `redis_ping` — Redis PONG
- `app_server` / `admin_server` / `web` — HTTP 健康端点
- `pids` — `.e2e-up.pids` 中进程的存活率

退出码采用位掩码（0=全绿；1+2+...=各项失败位）。

### 2.5 停服

```bash
npm run stop                       # 停业务进程（docker 保留）
docker compose down                # 停 docker（保留数据卷）
docker compose down -v             # 停 docker 并清空数据卷（重置 DB）
```

### 2.6 日志

| 文件                              | 内容               |
| --------------------------------- | ------------------ |
| `.e2e-logs/app-server.log`        | AppServer stdout   |
| `.e2e-logs/admin-server.log`      | AdminServer stdout |
| `.e2e-logs/web.log`               | Vite dev server    |
| `docker logs vr-postgres -f`      | PostgreSQL         |
| `docker logs vr-redis -f`         | Redis              |

---

## 3. PostgreSQL — 连接信息与常用操作

### 3.1 连接矩阵（本地默认）

| 用途                     | URI                                                                              |
| ------------------------ | -------------------------------------------------------------------------------- |
| 超级用户（运维）         | `postgres://postgres:postgres@localhost:5432/voiceroom`                          |
| AppServer（受限账号）    | `postgres://app_server_user:app_server_pass@localhost:5432/voiceroom`            |
| AdminServer（全权账号）  | `postgres://admin_server_user:admin_server_pass@localhost:5432/voiceroom`        |

> 密码可由 `.env` 中的 `POSTGRES_PASSWORD` / `APP_SERVER_PASS` / `ADMIN_SERVER_PASS` 覆盖。

### 3.2 进入交互终端

```bash
# 容器内 psql（推荐，无需本机装 psql）
docker exec -it vr-postgres psql -U postgres -d voiceroom

# 本机 psql（需安装 postgresql-client）
psql "postgres://postgres:postgres@localhost:5432/voiceroom"
```

### 3.3 状态 / 健康检查

```bash
docker exec vr-postgres pg_isready -U postgres                # 端口探活
docker exec vr-postgres psql -U postgres -d voiceroom -c '\dt'  # 列表
docker exec vr-postgres psql -U postgres -c 'SELECT version()'
```

### 3.4 备份 / 还原

```bash
# 备份
docker exec vr-postgres pg_dump -U postgres voiceroom \
  | gzip > backup-$(date +%Y%m%d-%H%M%S).sql.gz

# 还原
gunzip -c backup-XXXXXX.sql.gz \
  | docker exec -i vr-postgres psql -U postgres -d voiceroom
```

### 3.5 迁移

| 端          | 迁移目录                             | 触发方式                                  |
| ----------- | ------------------------------------ | ----------------------------------------- |
| AppServer   | `app/server/migrations/`             | 服务启动时自动运行（sqlx）                |
| AdminServer | `app/adminServer/migrations/`        | 服务启动时自动运行                        |

权限初始化（首次启动 docker 时自动执行）：`scripts/dev/init-db.sh` —— 创建受限/全权两个角色并 GRANT。

### 3.6 重置数据库（本地）

```bash
docker compose down -v             # 删除数据卷
docker compose up -d postgres      # 重建（init-db.sh 重新执行）
# 业务表会在下次 cargo run 时由 sqlx migrate 自动建立
```

### 3.7 灌测试种子

```bash
npm run db:seed       # 幂等，输出 .seed-output.env（含 JWT token）
npm run db:reset      # 清理（仅 local profile 允许）
```

---

## 4. Redis — 连接信息与常用操作

### 4.1 连接信息（本地默认）

| 项     | 值                       |
| ------ | ------------------------ |
| URI    | `redis://localhost:6379` |
| 认证   | 无                       |
| 持久化 | `appendonly yes`         |

### 4.2 进入 CLI

```bash
docker exec -it vr-redis redis-cli            # 容器内
redis-cli -h 127.0.0.1 -p 6379                # 本机
```

### 4.3 状态检查

```bash
docker exec vr-redis redis-cli ping           # → PONG
docker exec vr-redis redis-cli info keyspace
docker exec vr-redis redis-cli dbsize
docker exec vr-redis redis-cli keys '*'       # 仅本地用，生产严禁
```

### 4.4 重置

```bash
docker exec vr-redis redis-cli flushall       # 清空所有 key（不删数据卷）
docker compose down -v                        # 连数据卷一并删
```

---

## 5. Staging（测试环境）

### 5.1 关键差异

| 项               | 本地                | Staging                         |
| ---------------- | ------------------- | ------------------------------- |
| AppServer 域名   | `localhost:3000`    | `https://stg-api.example.com`   |
| 数据库           | docker container    | 云托管 PostgreSQL（RDS）        |
| Redis            | docker container    | 云托管 Redis                    |
| Web              | `vite dev :5173`    | 预构建产物 + Nginx              |
| 配置 profile     | `dev`               | `staging`                       |

### 5.2 服务端启动

```bash
# AppServer
APP_PROFILE=staging \
DATABASE_URL='postgres://USER:PASS@stg-db.host:5432/voiceroom' \
REDIS_URL='redis://stg-redis.host:6379' \
APP_JWT_SECRET=<from-secret-store> \
AGORA_APP_CERT=<from-secret-store> \
cargo run --release -p voice-room-server

# AdminServer
ADMIN_PROFILE=staging \
DATABASE_URL='postgres://admin_server_user:PASS@stg-db.host:5432/voiceroom' \
ADMIN_JWT_SECRET=<from-secret-store> \
cargo run --release -p voice-room-admin-server
```

### 5.3 Web 构建 / 部署

```bash
cd app/web
VITE_API_BASE_URL=https://stg-api.example.com \
VITE_WS_URL=wss://stg-api.example.com/ws \
VITE_ADMIN_API_BASE_URL=https://stg-admin-api.example.com \
npm run build
# 产物：app/web/dist/  → 部署到 Nginx / CDN
```

### 5.4 Android Staging 包

```bash
./gradlew :app:assembleStaging
# 或通过 ENV / local.properties 注入：
VOICE_ROOM_API_BASE_URL=https://stg-api.example.com/api \
VOICE_ROOM_WS_URL=wss://stg-api.example.com/ws \
./gradlew :app:assembleStaging
```

### 5.5 健康检查（远端）

```bash
curl -fsS https://stg-api.example.com/health        && echo APP_OK
curl -fsS https://stg-admin-api.example.com/health  && echo ADMIN_OK
```

> ⚠️ Staging 域名为占位符（`*.example.com`），需要替换为团队实际域名。

---

## 6. Production（生产环境）

### 6.1 与 Staging 的核心差异

- 数据库连接池：`db.max_connections=50`，`connect_timeout_secs=10`
- 日志：JSON，对接日志聚合（ELK / Loki）
- TLS：强制走反向代理（Nginx / API Gateway），后端不直接对外
- 配置：所有敏感值通过 secret store / K8s Secret 注入，**严禁写入文件**

### 6.2 推荐启动方式（systemd 示例）

```ini
# /etc/systemd/system/voice-room-app.service
[Unit]
Description=Voice Room AppServer
After=network.target

[Service]
EnvironmentFile=/etc/voice-room/app.env
ExecStart=/opt/voice-room/voice-room-server
Restart=always
RestartSec=5
User=voice-room

[Install]
WantedBy=multi-user.target
```

`/etc/voice-room/app.env`：

```ini
APP_PROFILE=prod
DATABASE_URL=postgres://app_server_user:***@prod-db.host:5432/voiceroom
REDIS_URL=redis://prod-redis.host:6379
APP_JWT_SECRET=***
AGORA_APP_CERT=***
RUST_LOG=info
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now voice-room-app
sudo systemctl status voice-room-app
sudo journalctl -u voice-room-app -f
```

AdminServer 同理（端口 3001、`ADMIN_PROFILE=prod`、`admin_server_user` 账号）。

### 6.3 反向代理（Nginx 摘要）

```nginx
upstream voice_app   { server 127.0.0.1:3000; }
upstream voice_admin { server 127.0.0.1:3001; }

server {
  listen 443 ssl http2;
  server_name api.example.com;

  location /health { proxy_pass http://voice_app; }
  location /ws     { proxy_pass http://voice_app;
                     proxy_http_version 1.1;
                     proxy_set_header Upgrade $http_upgrade;
                     proxy_set_header Connection "upgrade"; }
  location /       { proxy_pass http://voice_app; }
}

server {
  listen 443 ssl http2;
  server_name admin-api.example.com;
  location / { proxy_pass http://voice_admin; }
}
```

### 6.4 生产部署不建议用 docker-compose.yml

仓库当前 `docker-compose.yml` 仅用于**本地开发**（PG/Redis 本地 docker），生产应使用：

- 托管数据库（AWS RDS / 阿里云 PolarDB / GCP CloudSQL）
- 托管 Redis（AWS ElastiCache / 阿里云 Redis）
- 应用层走 K8s / Nomad / systemd

> 如需补充 `Dockerfile` / K8s manifests / Helm chart，请提 Task 走 Plan→TDD→Review 流程。

### 6.5 生产健康监控

```bash
curl -fsS https://api.example.com/health       || alert
curl -fsS https://admin-api.example.com/health || alert
```

建议接入 Prometheus + Alertmanager 持续轮询。

---

## 7. Android 客户端

### 7.1 构建命令

```bash
./gradlew :app:assembleDebug                # local flavor，BASE_URL=http://10.0.2.2:3000
./gradlew :app:assembleStaging
./gradlew :app:assembleRelease

./gradlew :app:testDebugUnitTest            # JVM 单测
./gradlew :app:connectedDebugAndroidTest    # 设备/模拟器集成测试
```

### 7.2 配置注入优先级

```
local.properties > ENV (VOICE_ROOM_*) > flavor 默认值
```

字段：`VOICE_ROOM_API_BASE_URL`、`VOICE_ROOM_WS_URL`、`VOICE_ROOM_ANALYTICS_ENDPOINT`、`SENTRY_DSN`。

详见 `doc/architecture/environments_cicd.md` 与 `doc/arch/android/index.md`。

---

## 8. 故障排查 SOP

> 完整 SOP 见 `doc/DEBUG_SOP.md`。本节仅速查。

| 现象                                  | 第一行检查命令                                   | 常见原因                                                  |
| ------------------------------------- | ------------------------------------------------ | --------------------------------------------------------- |
| `npm run start` 卡在 wait-on          | `bash scripts/dev/status.sh`                     | docker daemon 没起 / 镜像未拉                             |
| `e2e:up` 报端口冲突                   | `bash scripts/dev/check-ports.sh`                | 之前进程残留，先 `npm run stop` 或 `kill -9 <pid>`        |
| AppServer 启动后立刻退出              | `tail -100 .e2e-logs/app-server.log`             | DB 迁移失败 / `APP_JWT_SECRET` 缺失                       |
| AdminServer `permission denied` 报错  | `bash scripts/dev/verify-permissions.sh`         | 用了 `app_server_user` 连 admin 库，须用 `admin_server_user` |
| Web 加载白屏                          | 浏览器 console + `tail -100 .e2e-logs/web.log`   | `VITE_*` env 缺失，检查 `tests/scripts/env/.env.local`    |
| `cargo run` 抱怨 sqlx 离线模式        | `unset SQLX_OFFLINE && cargo run ...`            | CI 设了 SQLX_OFFLINE=true，本地需取消                     |
| Redis 连不上但容器在跑                | `docker exec vr-redis redis-cli ping`            | 容器健康但端口映射没生效，检查 `docker-compose.yml`        |
| Android 模拟器连不到 AppServer        | 用 `http://10.0.2.2:3000` 而非 `localhost:3000`  | 模拟器内的 localhost 是它自己                             |

### 8.1 排障四步走

1. **观察**：`npm run status` + 看 `.e2e-logs/`
2. **假设**：根据现象列出 ≤3 个可能原因
3. **验证**：用 `curl` / `psql` / `redis-cli` 对每个假设做最小验证
4. **行动**：找到根因后只改一个地方，再回到第 1 步

> 严禁在没有证据时同时改多个地方"碰运气"。

---

## 9. 脚本一览（`scripts/dev/`）

| 脚本                       | 用途                                           |
| -------------------------- | ---------------------------------------------- |
| `start-local.sh`           | 智能启动（已健康跳过）                         |
| `e2e-up.sh`                | 严格启动（端口被占即 fail）                    |
| `e2e-down.sh`              | 按 PID 停业务进程                              |
| `status.sh`                | 全栈状态总览（普通 / `--json`）                |
| `preflight.sh`             | fail-fast 健康检查（≤10 s）                    |
| `check-ports.sh`           | 端口冲突检测                                   |
| `init-db.sh`               | docker PG 首启时执行（创建受限/全权账号）      |
| `seed-e2e.sh` / `.sql`     | E2E 测试种子数据                               |
| `reset-e2e.sh`             | E2E 测试数据清理                               |
| `verify-permissions.sh`    | 验证 PG 权限隔离矩阵                           |
| `grant-permissions.sql`    | 权限隔离 SQL 补丁                              |
| `midscene-env-probe.ts`    | Midscene AI 视觉框架环境探测                   |

---

## 10. 进一步阅读

- 总体架构：`doc/architecture/index.md`
- 环境与 CI/CD：`doc/architecture/environments_cicd.md`
- E2E 测试 RUNBOOK：`doc/tests/E2E_RUNBOOK.md`
- 排障 SOP：`doc/DEBUG_SOP.md`
- Server / AdminServer / Web / Android 各端架构：`doc/arch/<端>/index.md`

---

> 最近更新：见 `git log -- RUNBOOK.md`。如发现命令与实际不符，请提 PR 修正本文档。
