#!/bin/bash

# ============================================
# 本地编译脚本 (Go 版本)
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

log_step "本地编译 Go 项目"

cd_project_root

log_info "检测编译环境..."

OS=$(uname -s)

if [ "${OS}" = "Darwin" ]; then
    log_success "检测到 macOS 系统"
    echo ""
    
    log_info "检查必要依赖..."
    if ! check_command go; then
        error_exit "Go 未安装\n请运行: make install-deps 或访问 https://go.dev/dl/"
    fi
    log_success "Go 已安装: $(go version)"
    
    if ! check_command ffmpeg; then
        error_exit "FFmpeg 未安装\n请运行: make install-deps"
    fi
    log_success "FFmpeg 已安装: $(ffmpeg -version | head -1)"
    echo ""
fi

log_info "开始编译..."
mkdir -p "${OUTPUT_DIR}"

cd "${GO_PROJECT}"

log_info "安装 Go 依赖..."
if ! go mod tidy; then
    error_exit "go mod tidy 失败"
fi

log_info "编译 Go 程序..."
# 编译整个包，而不是只编译 main.go
if ! GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build -o "${OUTPUT_DIR}/${BINARY_NAME}" .; then
    error_exit "编译失败"
fi

if [ ! -f "${OUTPUT_DIR}/${BINARY_NAME}" ]; then
    error_exit "编译后的二进制文件不存在\n请检查上面的编译错误信息"
fi

chmod +x "${OUTPUT_DIR}/${BINARY_NAME}"

echo ""
log_success "编译成功！"
echo "二进制文件: ${OUTPUT_DIR}/${BINARY_NAME}"
file "${OUTPUT_DIR}/${BINARY_NAME}" || true
ls -lh "${OUTPUT_DIR}/${BINARY_NAME}"
echo ""

if [ "${OS}" = "Darwin" ]; then
    log_warn "这是在 macOS 上编译的 Linux 二进制文件"
    echo "   此二进制文件无法在 macOS 上直接运行"
    echo "   如需本地测试，请使用: make dev"
    echo "   如需部署到函数计算，请使用: make deploy"
fi
