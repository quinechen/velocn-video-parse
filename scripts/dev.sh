#!/bin/bash

# ============================================
# 本地开发服务器脚本
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

log_step "启动本地开发服务器"

cd_project_root

# 对于 Go 版本，我们需要在本地编译并运行
# 检查 Go 是否安装
if ! check_command go; then
    error_exit "Go 未安装，请运行: make install-deps"
fi

log_info "编译 Go 程序（本地运行版本）..."
cd "${GO_PROJECT}"

# 本地编译（使用当前系统架构）
# 编译整个包，而不是只编译 main.go
if ! go build -o "${OUTPUT_DIR}/${BINARY_NAME}" .; then
    error_exit "编译失败"
fi

chmod +x "${OUTPUT_DIR}/${BINARY_NAME}"

if [ ! -f "${OUTPUT_DIR}/${BINARY_NAME}" ]; then
    error_exit "二进制文件不存在: ${OUTPUT_DIR}/${BINARY_NAME}"
fi

# 检查端口9000是否被占用
PORT=9000
log_info "检查端口 ${PORT} 是否可用..."

# 使用 lsof 检查端口（macOS 和 Linux 都支持）
if command -v lsof >/dev/null 2>&1; then
    if lsof -Pi :${PORT} -sTCP:LISTEN -t >/dev/null 2>&1; then
        PID=$(lsof -Pi :${PORT} -sTCP:LISTEN -t 2>/dev/null | head -n1)
        error_exit "端口 ${PORT} 已被占用 (PID: ${PID})，请先停止占用该端口的进程或使用其他端口"
    fi
# 如果没有 lsof，尝试使用 netstat (Linux)
elif command -v netstat >/dev/null 2>&1; then
    if netstat -tuln 2>/dev/null | grep -q ":${PORT} "; then
        error_exit "端口 ${PORT} 已被占用，请先停止占用该端口的进程或使用其他端口"
    fi
# 如果都没有，尝试使用 nc (netcat)
elif command -v nc >/dev/null 2>&1; then
    if nc -z localhost ${PORT} 2>/dev/null; then
        error_exit "端口 ${PORT} 已被占用，请先停止占用该端口的进程或使用其他端口"
    fi
else
    log_warn "无法检查端口占用情况（未找到 lsof/netstat/nc），继续启动..."
fi

log_success "端口 ${PORT} 可用"

echo "[shell] 服务器地址: http://0.0.0.0:9000"
echo ""
echo "[shell] 可用端点:"
echo "[shell]   • 健康检查:    GET  http://localhost:9000/health"
echo "[shell]   • OSS事件处理: POST http://localhost:9000/invoke"
echo "[shell]   • 直接处理:    POST http://localhost:9000/process"
echo "[shell]   • 查询处理:    GET  http://localhost:9000/process/query?input=<path>"
echo ""
echo "[shell] 按 Ctrl+C 停止服务器"
echo "=========================================="
echo ""

# 启动服务器
# 注意：Go 版本使用函数计算 SDK，需要设置环境变量模拟 FC 环境
export FC_FUNCTION_NAME="local-dev"
"${OUTPUT_DIR}/${BINARY_NAME}"
