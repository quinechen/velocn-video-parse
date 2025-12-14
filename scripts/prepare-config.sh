#!/bin/bash

# ============================================
# 准备配置文件脚本（内部辅助脚本）
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

cd_project_root

if [ -f "video-parse.ini" ]; then
    log_success "使用项目根目录的配置文件: video-parse.ini"
elif [ -f "lib-video-parse/video-parse.ini" ]; then
    log_info "找到 lib-video-parse 目录的配置文件，复制到项目根目录..."
    cp lib-video-parse/video-parse.ini video-parse.ini
    log_success "已复制配置文件到项目根目录"
elif [ -f "lib-video-parse/video-parse.ini.example" ]; then
    log_warn "未找到配置文件，从示例文件创建..."
    cp lib-video-parse/video-parse.ini.example video-parse.ini
    log_success "已从示例文件创建配置文件: video-parse.ini"
    echo "  提示: 请根据需要修改配置文件"
else
    error_exit "未找到配置文件或示例文件\n请确保 lib-video-parse/video-parse.ini.example 存在"
fi
