#!/bin/bash

# ============================================
# 安装依赖脚本 (Go 版本)
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

log_step "安装 Go、FFmpeg 和编译依赖"

OS=$(uname -s)

if [ "${OS}" = "Darwin" ]; then
    log_info "检测到 macOS 系统，使用 Homebrew 安装..."
    
    if ! check_command brew; then
        error_exit "未检测到 Homebrew\n请先安装 Homebrew:\n  /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
    fi
    log_success "Homebrew 已安装"
    
    echo ""
    log_info "检查 Go..."
    if ! check_command go; then
        log_info "正在安装 Go..."
        if ! brew install go; then
            error_exit "Go 安装失败"
        fi
    else
        log_success "Go 已安装: $(go version)"
    fi
    
    echo ""
    log_info "检查 FFmpeg..."
    if ! check_command ffmpeg; then
        log_info "正在安装 FFmpeg..."
        if ! brew install ffmpeg; then
            error_exit "FFmpeg 安装失败"
        fi
    else
        log_success "FFmpeg 已安装: $(ffmpeg -version | head -1)"
    fi
    
    echo ""
    log_success "macOS 依赖安装完成"
    
elif command -v apt-get > /dev/null 2>&1; then
    log_info "检测到 Ubuntu/Debian 系统，使用 apt-get 安装..."
    
    if ! sudo apt-get update; then
        error_exit "apt-get update 失败"
    fi
    
    log_info "安装 Go..."
    if ! check_command go; then
        # 安装 Go（使用官方方法或 apt）
        if ! sudo apt-get install -y golang-go; then
            log_warn "apt-get 安装 Go 失败，请手动安装"
            log_info "访问 https://go.dev/dl/ 下载并安装 Go"
        fi
    else
        log_success "Go 已安装: $(go version)"
    fi
    
    log_info "安装 FFmpeg..."
    if ! sudo apt-get install -y \
        ffmpeg \
        ca-certificates; then
        error_exit "FFmpeg 安装失败"
    fi
    
    echo ""
    log_info "验证安装..."
    if command -v go > /dev/null 2>&1; then
        log_success "Go 已正确安装"
        go version
    else
        log_warn "无法验证 Go 安装"
    fi
    
    if command -v ffmpeg > /dev/null 2>&1; then
        log_success "FFmpeg 已正确安装"
        ffmpeg -version | head -1
    else
        log_warn "无法验证 FFmpeg 安装"
    fi
    
    log_success "Ubuntu/Debian 依赖安装完成"
    
else
    error_exit "未检测到支持的包管理器（apt-get 或 brew）\n请手动安装以下依赖:\n  - Go (https://go.dev/dl/)\n  - FFmpeg"
fi

echo ""
log_step "依赖安装完成！"
echo ""
log_info "现在可以运行: make build"
