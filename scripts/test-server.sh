#!/bin/bash

# ============================================
# 测试服务器启动脚本
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

log_step "测试服务器启动"

cd_project_root

# 检查 Go 是否安装
if ! check_command go; then
    error_exit "Go 未安装，请运行: make install-deps"
fi

log_info "编译 Go 程序（测试版本）..."
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

# 使用一个测试端口
TEST_PORT=9999
TEST_ADDR="0.0.0.0:${TEST_PORT}"

log_info "使用测试端口: ${TEST_PORT}"

# 检查测试端口是否被占用
if command -v lsof >/dev/null 2>&1; then
    if lsof -Pi :${TEST_PORT} -sTCP:LISTEN -t >/dev/null 2>&1; then
        PID=$(lsof -Pi :${TEST_PORT} -sTCP:LISTEN -t 2>/dev/null | head -n1)
        error_exit "测试端口 ${TEST_PORT} 已被占用 (PID: ${PID})"
    fi
fi

log_info "启动服务器进行测试..."

# 在后台启动服务器
# 注意：Go 版本使用函数计算 SDK，需要设置环境变量模拟 FC 环境
export FC_FUNCTION_NAME="test-server"
export FC_SERVER_PORT="${TEST_PORT}"
"${OUTPUT_DIR}/${BINARY_NAME}" > /tmp/test-server.log 2>&1 &
SERVER_PID=$!

log_info "服务器进程 ID: ${SERVER_PID}"

# 等待服务器启动（最多等待5秒）
MAX_WAIT=5
WAIT_COUNT=0
while [ $WAIT_COUNT -lt $MAX_WAIT ]; do
    sleep 1
    WAIT_COUNT=$((WAIT_COUNT + 1))
    
    # 检查进程是否还在运行
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        log_error "服务器进程已退出"
        log_error "服务器日志:"
        cat /tmp/test-server.log
        exit 1
    fi
    
    # 检查端口是否在监听
    if command -v lsof >/dev/null 2>&1; then
        if lsof -Pi :${TEST_PORT} -sTCP:LISTEN -t >/dev/null 2>&1; then
            log_success "服务器已启动并监听端口 ${TEST_PORT}"
            break
        fi
    fi
    
    log_info "等待服务器启动... (${WAIT_COUNT}/${MAX_WAIT})"
done

# 测试健康检查端点
log_info "测试健康检查端点..."
if curl -s -f "http://localhost:${TEST_PORT}/health" > /dev/null 2>&1; then
    log_success "健康检查端点响应正常"
    curl -s "http://localhost:${TEST_PORT}/health" | head -5
    echo ""
else
    log_error "健康检查端点无响应"
    log_error "服务器日志:"
    cat /tmp/test-server.log
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

# 停止服务器
log_info "停止测试服务器..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

log_success "服务器测试完成"

