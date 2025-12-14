#!/bin/bash

# ============================================
# 推送 Docker 镜像脚本
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

log_step "推送 Docker 镜像到容器镜像服务"

cd_project_root

# 检查 Docker
if ! check_command docker; then
    error_exit "Docker 未安装或未启动"
fi

# 检查本地镜像是否存在
if ! docker images "${DOCKER_IMAGE}:${DOCKER_TAG}" | grep -q "${DOCKER_IMAGE}"; then
    error_exit "镜像 ${DOCKER_IMAGE}:${DOCKER_TAG} 不存在\n请先运行: make docker"
fi
log_success "本地镜像已存在"

echo ""
log_info "标记镜像: ${FULL_IMAGE_NAME}"
if ! docker tag "${DOCKER_IMAGE}:${DOCKER_TAG}" "${FULL_IMAGE_NAME}"; then
    error_exit "镜像标记失败"
fi
log_success "镜像标记成功"

echo ""
log_info "推送镜像到 ${CONTAINER_REGISTRY}..."
if ! docker push "${FULL_IMAGE_NAME}"; then
    error_exit "镜像推送失败，请检查 Docker 登录状态"
fi

echo ""
log_success "镜像推送成功！"
echo "镜像地址: ${FULL_IMAGE_NAME}"
echo ""
log_info "提示: 请确保 s.yaml 中配置了正确的镜像地址"
