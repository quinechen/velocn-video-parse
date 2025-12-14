#!/bin/bash

# ============================================
# 通用函数库
# ============================================

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 获取脚本所在目录的父目录（项目根目录）
if [ -n "${BASH_SOURCE[0]:-}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
fi
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 项目配置
GO_PROJECT="${PROJECT_ROOT}/code"
OUTPUT_DIR="${GO_PROJECT}/target"
BINARY_NAME="main"
GO_BINARY="main"
DOCKER_IMAGE="video-parse-go"
DOCKER_TAG="latest"
CONTAINER_REGISTRY="registry.cn-hangzhou.aliyuncs.com"
CONTAINER_NAMESPACE="dockerhacker"
FULL_IMAGE_NAME="${CONTAINER_REGISTRY}/${CONTAINER_NAMESPACE}/${DOCKER_IMAGE}:${DOCKER_TAG}"

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_step() {
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# 检查命令是否存在
check_command() {
    if ! command -v "$1" > /dev/null 2>&1; then
        log_error "命令 '$1' 未安装或不在 PATH 中"
        return 1
    fi
    return 0
}

# 检查文件是否存在
check_file() {
    if [ ! -f "$1" ]; then
        log_error "文件不存在: $1"
        return 1
    fi
    return 0
}

# 检查目录是否存在
check_dir() {
    if [ ! -d "$1" ]; then
        log_error "目录不存在: $1"
        return 1
    fi
    return 0
}

# 错误处理
error_exit() {
    log_error "$1"
    exit 1
}

# 切换到项目根目录
cd_project_root() {
    cd "${PROJECT_ROOT}" || error_exit "无法切换到项目根目录: ${PROJECT_ROOT}"
}
