#!/usr/bin/env bash
# 缺陷 6 修复 — 配套停服脚本：按 .e2e-up.pids 停 cargo / vite，docker compose 保留。
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PIDS_FILE="$REPO_ROOT/.e2e-up.pids"
if [[ -f "$PIDS_FILE" ]]; then
  while read -r pid; do
    [[ -z "$pid" ]] && continue
    if kill -0 "$pid" 2>/dev/null; then
      echo "[e2e:down] kill $pid"
      kill "$pid" 2>/dev/null || true
    fi
  done < "$PIDS_FILE"
  rm -f "$PIDS_FILE"
fi
echo "[e2e:down] 业务进程已停。docker postgres+redis 保留，如需停跑 'docker compose down'。"
