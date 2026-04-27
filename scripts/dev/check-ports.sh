#!/usr/bin/env bash
# 端口冲突检测脚本（T-0000Q）
# 用法：bash scripts/dev/check-ports.sh
# 退出码：0=所有端口可用，1=有端口冲突
set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 函数：获取端口占用的 PID（跨平台）
get_port_pid() {
  local port=$1
  local pid=""
  
  if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS: 使用 lsof
    pid=$(lsof -ti ":$port" 2>/dev/null || true)
  else
    # Linux: 使用 ss 或 netstat
    if command -v ss >/dev/null 2>&1; then
      # 优先使用 ss (现代工具)
      pid=$(ss -tulnp 2>/dev/null | grep ":$port " | awk '{print $7}' | grep -oP 'pid=\K[0-9]+' || true)
    elif command -v netstat >/dev/null 2>&1; then
      # fallback 到 netstat (legacy)
      pid=$(netstat -tulnp 2>/dev/null | grep ":$port " | awk '{print $7}' | cut -d'/' -f1 || true)
    fi
  fi
  
  echo "$pid"
}

# 函数：检测单个端口是否被占用
check_port() {
  local port=$1
  local service_name=$2
  local pid
  
  pid=$(get_port_pid "$port")

  if [[ -n "$pid" ]]; then
    # 获取进程名称
    local process_name
    process_name=$(ps -p "$pid" -o comm= 2>/dev/null || echo "unknown")
    echo -e "${RED}✗ Port $port ($service_name) is already in use by PID $pid ($process_name)${NC}"
    return 1
  else
    echo -e "${GREEN}✓ Port $port ($service_name) is available${NC}"
    return 0
  fi
}

# 主流程：检测所有端口
conflicts=0
conflict_info=()

# 按顺序检测所有关键端口
if ! check_port 5432 "PostgreSQL"; then
  conflicts=$((conflicts + 1))
  pid=$(get_port_pid 5432)
  if [[ -n "$pid" ]]; then
    conflict_info+=("$pid|5432|PostgreSQL")
  fi
fi

if ! check_port 6379 "Redis"; then
  conflicts=$((conflicts + 1))
  pid=$(get_port_pid 6379)
  if [[ -n "$pid" ]]; then
    conflict_info+=("$pid|6379|Redis")
  fi
fi

if ! check_port 3000 "AppServer"; then
  conflicts=$((conflicts + 1))
  pid=$(get_port_pid 3000)
  if [[ -n "$pid" ]]; then
    conflict_info+=("$pid|3000|AppServer")
  fi
fi

if ! check_port 3001 "AdminServer"; then
  conflicts=$((conflicts + 1))
  pid=$(get_port_pid 3001)
  if [[ -n "$pid" ]]; then
    conflict_info+=("$pid|3001|AdminServer")
  fi
fi

if ! check_port 5173 "Web"; then
  conflicts=$((conflicts + 1))
  pid=$(get_port_pid 5173)
  if [[ -n "$pid" ]]; then
    conflict_info+=("$pid|5173|Web")
  fi
fi

# 如果有冲突，输出错误信息
if [[ $conflicts -gt 0 ]]; then
  echo "" >&2
  echo -e "${RED}ERROR: $conflicts port(s) are already in use.${NC}" >&2
  echo -e "${YELLOW}Please stop the conflicting processes manually:${NC}" >&2
  echo "" >&2
  
  # 输出 kill 命令提示
  for info in "${conflict_info[@]}"; do
    IFS='|' read -r pid port service <<< "$info"
    echo "  kill -9 $pid  # $service (port $port)" >&2
  done
  echo "" >&2
  exit 1
fi

exit 0

