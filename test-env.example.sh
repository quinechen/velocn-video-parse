#!/bin/bash

# ============================================
# OSS 事件测试环境变量配置示例
# ============================================
# 使用方法：
#   1. 复制此文件为 test-env.sh
#   2. 填入你的实际凭证信息
#   3. 运行: source test-env.sh
#   4. 然后运行: ./test-oss-event.sh ...
# ============================================

# 阿里云 Access Key ID（必需）
export ALIBABA_CLOUD_ACCESS_KEY_ID="your-access-key-id"

# 阿里云 Access Key Secret（必需）
export ALIBABA_CLOUD_ACCESS_KEY_SECRET="your-access-key-secret"

# STS Security Token（可选，如果使用临时凭证）
export ALIBABA_CLOUD_SECURITY_TOKEN="your-security-token"

# Docker 镜像名称（可选，默认：video-parse:latest）
# export DOCKER_IMAGE_NAME="video-parse:latest"

# Docker 端口映射（可选，默认：9000:9000）
# export DOCKER_PORT="9000:9000"

# 自定义请求ID（可选，默认会自动生成）
# export FC_REQUEST_ID="custom-request-id"

# 文件大小（可选，用于生成 OSS Event，默认：0）
# export FILE_SIZE=10485760  # 10MB

# 是否在退出时自动清理容器（可选，默认：true）
# export CLEANUP_ON_EXIT=true

echo "环境变量已设置："
echo "  ALIBABA_CLOUD_ACCESS_KEY_ID: ${ALIBABA_CLOUD_ACCESS_KEY_ID:0:10}..."
echo "  ALIBABA_CLOUD_ACCESS_KEY_SECRET: ${ALIBABA_CLOUD_ACCESS_KEY_SECRET:0:10}..."
if [[ -n "${ALIBABA_CLOUD_SECURITY_TOKEN}" ]]; then
    echo "  ALIBABA_CLOUD_SECURITY_TOKEN: ${ALIBABA_CLOUD_SECURITY_TOKEN:0:10}..."
fi
