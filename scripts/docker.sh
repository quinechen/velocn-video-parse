#!/bin/bash

# ============================================
# 构建 Docker 镜像脚本
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

log_step "构建 Docker 镜像"

cd_project_root

# 检查 Docker
if ! check_command docker; then
    error_exit "Docker 未安装或未启动"
fi
log_success "Docker 已安装"

echo ""
log_info "构建镜像: ${DOCKER_IMAGE}:${DOCKER_TAG}"
log_info "平台: linux/amd64"
log_info "使用 Go 版本的 Dockerfile"
echo ""

# 切换到 code 目录构建
cd "${GO_PROJECT}"

if ! docker build --platform linux/amd64 -t "${DOCKER_IMAGE}:${DOCKER_TAG}" .; then
    error_exit "Docker 镜像构建失败"
fi

echo ""
log_success "Docker 镜像构建成功！"
echo "镜像名称: ${DOCKER_IMAGE}:${DOCKER_TAG}"
docker images "${DOCKER_IMAGE}:${DOCKER_TAG}" | tail -1
echo ""
echo "本地测试运行:"
echo "  docker run -p 9000:9000 \\"
echo "    -e ALIBABA_CLOUD_ACCESS_KEY_ID=\"your-key\" \\"
echo "    -e ALIBABA_CLOUD_ACCESS_KEY_SECRET=\"your-secret\" \\"
echo "    ${DOCKER_IMAGE}:${DOCKER_TAG}"
echo ""
echo "推送镜像:"
echo "  make docker-push"
