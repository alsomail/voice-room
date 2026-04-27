#!/usr/bin/env bash
# TDD 测试：e2e-up.sh 端口检测功能
# 测试 check_ports 函数的各种场景

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# 颜色
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 测试计数
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# 临时端口占用进程 PID 列表（测试结束时清理）
CLEANUP_PIDS=()

cleanup() {
  if [[ ${#CLEANUP_PIDS[@]} -gt 0 ]]; then
    for pid in "${CLEANUP_PIDS[@]}"; do
      kill -9 "$pid" 2>/dev/null || true
    done
  fi
  # 等待端口释放
  sleep 1
}
trap cleanup EXIT

# 辅助：占用端口（返回 PID）
occupy_port() {
  local port=$1
  if command -v nc >/dev/null 2>&1; then
    # 使用 nc (netcat)
    nc -l "$port" >/dev/null 2>&1 &
    local pid=$!
  else
    # fallback: python3
    python3 -c "import socket,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); s.bind(('127.0.0.1',$port)); s.listen(1); time.sleep(300)" &
    local pid=$!
  fi
  CLEANUP_PIDS+=("$pid")
  sleep 0.5  # 等待端口绑定
  echo "$pid"
}

# 辅助：检查端口是否被占用
is_port_occupied() {
  local port=$1
  if [[ "$OSTYPE" == "darwin"* ]]; then
    lsof -ti ":$port" >/dev/null 2>&1
  else
    ss -tulnp 2>/dev/null | grep -q ":$port " || netstat -tulnp 2>/dev/null | grep -q ":$port "
  fi
}

# 断言函数
assert_exit_code() {
  local expected=$1
  local actual=$2
  local test_name=$3
  TESTS_RUN=$((TESTS_RUN + 1))
  if [[ "$actual" -eq "$expected" ]]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✓${NC} $test_name (exit code: $actual)"
  else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "${RED}✗${NC} $test_name (expected: $expected, got: $actual)"
  fi
}

assert_output_contains() {
  local needle=$1
  local haystack=$2
  local test_name=$3
  TESTS_RUN=$((TESTS_RUN + 1))
  if echo "$haystack" | grep -q "$needle"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✓${NC} $test_name (contains: $needle)"
  else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "${RED}✗${NC} $test_name (missing: $needle)"
    echo "  Haystack: $haystack"
  fi
}

assert_output_not_contains() {
  local needle=$1
  local haystack=$2
  local test_name=$3
  TESTS_RUN=$((TESTS_RUN + 1))
  if ! echo "$haystack" | grep -q "$needle"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✓${NC} $test_name (not contains: $needle)"
  else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "${RED}✗${NC} $test_name (should not contain: $needle)"
  fi
}

# 等待 check-ports.sh 脚本存在
wait_for_script() {
  local script_path="$REPO_ROOT/scripts/dev/check-ports.sh"
  if [[ ! -f "$script_path" ]]; then
    echo -e "${YELLOW}⚠${NC} check-ports.sh 不存在，测试将失败（这是 TDD RED 阶段预期行为）"
    return 1
  fi
  return 0
}

echo "=========================================="
echo "   TDD 测试：端口检测功能"
echo "=========================================="
echo ""

# ==================== U-1: 所有端口空闲 ====================
echo "【U-1】所有端口空闲 → check_ports 返回 0，输出 5 行 ✓"
if wait_for_script; then
  # 确保测试端口未被占用
  for port in 5432 6379 3000 3001 5173; do
    if is_port_occupied "$port"; then
      echo -e "${RED}✗${NC} 测试前提失败：端口 $port 已被占用，请先清理"
      exit 1
    fi
  done

  set +e
  output=$(bash "$REPO_ROOT/scripts/dev/check-ports.sh" 2>&1)
  exit_code=$?
  set -e

  assert_exit_code 0 "$exit_code" "U-1: 空闲端口 → 退出码 0"
  assert_output_contains "✓ Port 5432" "$output" "U-1: 输出包含 PostgreSQL 可用"
  assert_output_contains "✓ Port 6379" "$output" "U-1: 输出包含 Redis 可用"
  assert_output_contains "✓ Port 3000" "$output" "U-1: 输出包含 AppServer 可用"
  assert_output_contains "✓ Port 3001" "$output" "U-1: 输出包含 AdminServer 可用"
  assert_output_contains "✓ Port 5173" "$output" "U-1: 输出包含 Web 可用"
else
  TESTS_RUN=$((TESTS_RUN + 6))
  TESTS_FAILED=$((TESTS_FAILED + 6))
  echo -e "${RED}✗${NC} U-1: 脚本不存在（RED 阶段）"
fi
echo ""

# ==================== U-2: 占用 5432 ====================
echo "【U-2】占用 5432 → 脚本退出码非 0 + 红色错误 + kill 命令提示"
if wait_for_script; then
  pid=$(occupy_port 5432)
  
  set +e
  output=$(bash "$REPO_ROOT/scripts/dev/check-ports.sh" 2>&1)
  exit_code=$?
  set -e

  assert_exit_code 1 "$exit_code" "U-2: 端口冲突 → 退出码 1"
  assert_output_contains "✗.*5432" "$output" "U-2: 输出包含端口 5432 错误"
  assert_output_contains "kill -9 $pid" "$output" "U-2: 输出包含 kill 命令提示"
  assert_output_contains "ERROR" "$output" "U-2: 输出包含 ERROR 标识"
  
  # 清理
  kill -9 "$pid" 2>/dev/null || true
  sleep 1
else
  TESTS_RUN=$((TESTS_RUN + 4))
  TESTS_FAILED=$((TESTS_FAILED + 4))
  echo -e "${RED}✗${NC} U-2: 脚本不存在（RED 阶段）"
fi
echo ""

# ==================== U-3: 多端口冲突 ====================
echo "【U-3】占用 5432 + 6379 → 错误信息列出 2 个端口 + 2 条 kill 命令"
if wait_for_script; then
  pid1=$(occupy_port 5432)
  pid2=$(occupy_port 6379)
  
  set +e
  output=$(bash "$REPO_ROOT/scripts/dev/check-ports.sh" 2>&1)
  exit_code=$?
  set -e

  assert_exit_code 1 "$exit_code" "U-3: 多端口冲突 → 退出码 1"
  assert_output_contains "✗.*5432" "$output" "U-3: 输出包含端口 5432 错误"
  assert_output_contains "✗.*6379" "$output" "U-3: 输出包含端口 6379 错误"
  assert_output_contains "kill -9 $pid1" "$output" "U-3: 输出包含 PID $pid1 kill 命令"
  assert_output_contains "kill -9 $pid2" "$output" "U-3: 输出包含 PID $pid2 kill 命令"
  
  # 清理
  kill -9 "$pid1" "$pid2" 2>/dev/null || true
  sleep 1
else
  TESTS_RUN=$((TESTS_RUN + 5))
  TESTS_FAILED=$((TESTS_FAILED + 5))
  echo -e "${RED}✗${NC} U-3: 脚本不存在（RED 阶段）"
fi
echo ""

# ==================== U-4: 进程名出现在错误信息 ====================
echo "【U-4】进程名出现在错误信息（PID + 进程名）"
if wait_for_script; then
  pid=$(occupy_port 5432)
  
  set +e
  output=$(bash "$REPO_ROOT/scripts/dev/check-ports.sh" 2>&1)
  set -e
  
  # 获取进程名
  process_name=$(ps -p "$pid" -o comm= 2>/dev/null || echo "unknown")
  
  assert_output_contains "$pid" "$output" "U-4: 输出包含 PID $pid"
  assert_output_contains "$process_name" "$output" "U-4: 输出包含进程名 $process_name"
  
  # 清理
  kill -9 "$pid" 2>/dev/null || true
  sleep 1
else
  TESTS_RUN=$((TESTS_RUN + 2))
  TESTS_FAILED=$((TESTS_FAILED + 2))
  echo -e "${RED}✗${NC} U-4: 脚本不存在（RED 阶段）"
fi
echo ""

# ==================== 平台检测测试 ====================
echo "【P-1/P-2】跨平台检测（macOS 用 lsof / Linux 用 ss 或 netstat）"
if wait_for_script; then
  # 检查脚本内部是否有平台分支逻辑
  script_content=$(cat "$REPO_ROOT/scripts/dev/check-ports.sh")
  
  TESTS_RUN=$((TESTS_RUN + 2))
  if echo "$script_content" | grep -q "OSTYPE.*darwin"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✓${NC} P-1/P-2: 脚本包含 macOS 检测 (OSTYPE darwin)"
  else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "${RED}✗${NC} P-1/P-2: 脚本缺少 macOS 检测"
  fi
  
  if echo "$script_content" | grep -q "lsof" && (echo "$script_content" | grep -q "ss " || echo "$script_content" | grep -q "netstat"); then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✓${NC} P-1/P-2: 脚本同时包含 lsof (macOS) 和 ss/netstat (Linux)"
  else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "${RED}✗${NC} P-1/P-2: 脚本缺少跨平台端口检测工具"
  fi
else
  TESTS_RUN=$((TESTS_RUN + 2))
  TESTS_FAILED=$((TESTS_FAILED + 2))
  echo -e "${RED}✗${NC} P-1/P-2: 脚本不存在（RED 阶段）"
fi
echo ""

# ==================== 测试总结 ====================
echo "=========================================="
echo "   测试结果"
echo "=========================================="
echo "运行: $TESTS_RUN"
echo -e "${GREEN}通过: $TESTS_PASSED${NC}"
if [[ $TESTS_FAILED -gt 0 ]]; then
  echo -e "${RED}失败: $TESTS_FAILED${NC}"
  exit 1
else
  echo -e "${GREEN}✅ 所有测试通过！${NC}"
  exit 0
fi
