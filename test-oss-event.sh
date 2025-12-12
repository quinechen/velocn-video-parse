#!/bin/bash

# ============================================
# OSS 事件测试脚本
# ============================================
# 功能：
#   1. 构建或运行 Docker 镜像
#   2. 构造 OSS Event JSON
#   3. 发送请求到 /invoke 端点
#   4. 支持通过环境变量注入 STS token
#
# 使用方法：
#   ./test-oss-event.sh --bucket my-bucket --key path/to/video.mp4 --region cn-hangzhou
#
# 环境变量：
#   ALIBABA_CLOUD_ACCESS_KEY_ID      - 阿里云 Access Key ID
#   ALIBABA_CLOUD_ACCESS_KEY_SECRET  - 阿里云 Access Key Secret
#   ALIBABA_CLOUD_SECURITY_TOKEN     - STS Security Token（可选）
#   DOCKER_IMAGE_NAME                 - Docker 镜像名称（默认：video-parse:latest）
#   DOCKER_PORT                       - Docker 端口映射（默认：9000:9000）
#   FC_REQUEST_ID                     - 请求ID（默认：自动生成）
# ============================================

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 默认配置
DOCKER_IMAGE_NAME="${DOCKER_IMAGE_NAME:-video-parse:latest}"
DOCKER_PORT="${DOCKER_PORT:-9000:9000}"
CONTAINER_NAME="video-parse-test-$(date +%s)"
FC_REQUEST_ID="${FC_REQUEST_ID:-$(uuidgen 2>/dev/null || echo "test-$(date +%s)")}"

# 解析命令行参数
BUCKET=""
OBJECT_KEY=""
REGION=""
EVENT_NAME="ObjectCreated:Put"
BUILD_IMAGE=false
RUN_CONTAINER=false
STOP_CONTAINER=false
CLEANUP=false

# 显示帮助信息
show_help() {
    cat << EOF
用法: $0 [选项]

选项:
  -b, --bucket BUCKET            OSS Bucket 名称（必需）
  -k, --key KEY                  OSS Object Key（文件路径，必需）
  -r, --region REGION            OSS Region（必需，例如：cn-hangzhou）
  -e, --event EVENT_NAME         事件名称（默认：ObjectCreated:Put）
  -i, --image IMAGE_NAME         Docker 镜像名称（默认：video-parse:latest）
  -p, --port PORT                端口映射（默认：9000:9000）
  --build                        构建 Docker 镜像
  --run                          运行 Docker 容器（后台）
  --stop                         停止并删除容器
  --cleanup                      清理容器和镜像
  -h, --help                     显示此帮助信息

环境变量:
  ALIBABA_CLOUD_ACCESS_KEY_ID     阿里云 Access Key ID（必需）
  ALIBABA_CLOUD_ACCESS_KEY_SECRET 阿里云 Access Key Secret（必需）
  ALIBABA_CLOUD_SECURITY_TOKEN    STS Security Token（可选）

示例:
  # 构建镜像
  $0 --build

  # 运行容器并发送测试事件
  $0 --run --bucket my-bucket --key videos/test.mp4 --region cn-hangzhou

  # 仅发送测试事件（容器已运行）
  $0 --bucket my-bucket --key videos/test.mp4 --region cn-hangzhou

  # 停止容器
  $0 --stop

  # 清理所有资源
  $0 --cleanup
EOF
}

# 解析参数
while [[ $# -gt 0 ]]; do
    case $1 in
        -b|--bucket)
            BUCKET="$2"
            shift 2
            ;;
        -k|--key)
            OBJECT_KEY="$2"
            shift 2
            ;;
        -r|--region)
            REGION="$2"
            shift 2
            ;;
        -e|--event)
            EVENT_NAME="$2"
            shift 2
            ;;
        -i|--image)
            DOCKER_IMAGE_NAME="$2"
            shift 2
            ;;
        -p|--port)
            DOCKER_PORT="$2"
            shift 2
            ;;
        --build)
            BUILD_IMAGE=true
            shift
            ;;
        --run)
            RUN_CONTAINER=true
            shift
            ;;
        --stop)
            STOP_CONTAINER=true
            shift
            ;;
        --cleanup)
            CLEANUP=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo -e "${RED}错误: 未知参数 $1${NC}"
            show_help
            exit 1
            ;;
    esac
done

# 清理函数
cleanup() {
    echo -e "${YELLOW}正在清理...${NC}"
    
    # 停止并删除所有匹配的容器（因为容器名是动态生成的）
    local containers=$(docker ps -a --format '{{.Names}}' | grep "^video-parse-test-" || true)
    if [[ -n "$containers" ]]; then
        echo "$containers" | while read -r container; do
            echo -e "${BLUE}停止并删除容器: ${container}${NC}"
            docker stop "${container}" >/dev/null 2>&1 || true
            docker rm "${container}" >/dev/null 2>&1 || true
        done
    fi
    
    # 如果指定了容器名，也尝试清理
    if [[ -n "${CONTAINER_NAME}" ]]; then
        if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
            echo -e "${BLUE}停止容器: ${CONTAINER_NAME}${NC}"
            docker stop "${CONTAINER_NAME}" >/dev/null 2>&1 || true
            docker rm "${CONTAINER_NAME}" >/dev/null 2>&1 || true
        fi
    fi
    
    # 删除镜像（可选）
    if [[ "$CLEANUP" == true ]]; then
        if docker images --format '{{.Repository}}:{{.Tag}}' | grep -q "^${DOCKER_IMAGE_NAME}$"; then
            echo -e "${BLUE}删除镜像: ${DOCKER_IMAGE_NAME}${NC}"
            docker rmi "${DOCKER_IMAGE_NAME}" >/dev/null 2>&1 || true
        fi
    fi
    
    echo -e "${GREEN}清理完成${NC}"
}

# 停止容器
stop_container() {
    # 查找所有匹配的容器
    local containers=$(docker ps -a --format '{{.Names}}' | grep "^video-parse-test-" || true)
    
    if [[ -n "$containers" ]]; then
        echo "$containers" | while read -r container; do
            echo -e "${BLUE}停止并删除容器: ${container}${NC}"
            docker stop "${container}" 2>/dev/null || true
            docker rm "${container}" 2>/dev/null || true
        done
        echo -e "${GREEN}所有测试容器已停止并删除${NC}"
    else
        echo -e "${YELLOW}未找到运行中的测试容器${NC}"
    fi
    exit 0
}

# 构建 Docker 镜像
build_image() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}构建 Docker 镜像: ${DOCKER_IMAGE_NAME}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    # 检查 Dockerfile 是否存在
    if [[ ! -f "Dockerfile" ]]; then
        echo -e "${RED}错误: 未找到 Dockerfile${NC}"
        exit 1
    fi
    
    docker build -t "${DOCKER_IMAGE_NAME}" .
    
    echo -e "${GREEN}镜像构建完成: ${DOCKER_IMAGE_NAME}${NC}"
}

# 运行 Docker 容器
run_container() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}运行 Docker 容器${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    # 检查镜像是否存在
    if ! docker images --format '{{.Repository}}:{{.Tag}}' | grep -q "^${DOCKER_IMAGE_NAME}$"; then
        echo -e "${YELLOW}镜像 ${DOCKER_IMAGE_NAME} 不存在，正在构建...${NC}"
        build_image
    fi
    
    # 检查容器是否已运行
    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo -e "${YELLOW}容器 ${CONTAINER_NAME} 已在运行${NC}"
        return
    fi
    
    # 检查必需的环境变量
    if [[ -z "${ALIBABA_CLOUD_ACCESS_KEY_ID}" ]]; then
        echo -e "${RED}错误: 未设置 ALIBABA_CLOUD_ACCESS_KEY_ID 环境变量${NC}"
        exit 1
    fi
    
    if [[ -z "${ALIBABA_CLOUD_ACCESS_KEY_SECRET}" ]]; then
        echo -e "${RED}错误: 未设置 ALIBABA_CLOUD_ACCESS_KEY_SECRET 环境变量${NC}"
        exit 1
    fi
    
    # 构建环境变量参数
    ENV_ARGS=(
        -e "ALIBABA_CLOUD_ACCESS_KEY_ID=${ALIBABA_CLOUD_ACCESS_KEY_ID}"
        -e "ALIBABA_CLOUD_ACCESS_KEY_SECRET=${ALIBABA_CLOUD_ACCESS_KEY_SECRET}"
    )
    
    # 如果提供了 STS token，添加到环境变量
    if [[ -n "${ALIBABA_CLOUD_SECURITY_TOKEN}" ]]; then
        ENV_ARGS+=(-e "ALIBABA_CLOUD_SECURITY_TOKEN=${ALIBABA_CLOUD_SECURITY_TOKEN}")
        echo -e "${GREEN}✓ 已配置 STS Token${NC}"
    fi
    
    # 运行容器
    echo -e "${BLUE}启动容器: ${CONTAINER_NAME}${NC}"
    echo -e "${BLUE}端口映射: ${DOCKER_PORT}${NC}"
    
    docker run -d \
        --name "${CONTAINER_NAME}" \
        -p "${DOCKER_PORT}" \
        "${ENV_ARGS[@]}" \
        "${DOCKER_IMAGE_NAME}"
    
    # 等待容器启动并检查健康状态
    echo -e "${YELLOW}等待容器启动...${NC}"
    local max_attempts=30
    local attempt=0
    local host_port=$(docker port "${CONTAINER_NAME}" 9000/tcp 2>/dev/null | cut -d: -f2)
    
    while [[ $attempt -lt $max_attempts ]]; do
        sleep 1
        attempt=$((attempt + 1))
        
        # 检查容器是否运行
        if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
            echo -e "${RED}✗ 容器启动失败${NC}"
            docker logs "${CONTAINER_NAME}" 2>&1 | tail -20
            exit 1
        fi
        
        # 检查健康端点（如果端口可用）
        if [[ -n "${host_port}" ]]; then
            if curl -s -f "http://localhost:${host_port}/health" >/dev/null 2>&1; then
                echo -e "${GREEN}✓ 容器健康检查通过${NC}"
                break
            fi
        fi
        
        if [[ $attempt -eq $max_attempts ]]; then
            echo -e "${YELLOW}⚠ 健康检查超时，但容器已启动${NC}"
        fi
    done
    
    # 检查容器状态
    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo -e "${GREEN}✓ 容器已启动: ${CONTAINER_NAME}${NC}"
        echo -e "${BLUE}容器日志:${NC}"
        docker logs "${CONTAINER_NAME}" | tail -10
    else
        echo -e "${RED}✗ 容器启动失败${NC}"
        docker logs "${CONTAINER_NAME}" 2>&1 | tail -20
        exit 1
    fi
}

# 生成 OSS Event JSON
generate_oss_event() {
    local bucket="$1"
    local object_key="$2"
    local region="$3"
    local event_name="$4"
    
    # 获取当前时间（ISO 8601 格式）
    local event_time=$(date -u +"%Y-%m-%dT%H:%M:%S.000Z" 2>/dev/null || date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    # 生成请求ID
    local request_id="${FC_REQUEST_ID}"
    
    # 生成 ETag（模拟）
    local etag=$(echo -n "${object_key}" | md5sum | cut -d' ' -f1)
    
    # 获取文件大小（如果可能）
    local file_size="${FILE_SIZE:-0}"
    
    # 构造 OSS Event JSON
    cat << EOF
{
  "events": [
    {
      "eventName": "${event_name}",
      "eventSource": "acs:oss",
      "eventTime": "${event_time}",
      "eventVersion": "1.0",
      "oss": {
        "bucket": {
          "arn": "acs:oss:${region}:*:${bucket}",
          "name": "${bucket}",
          "ownerIdentity": {
            "principalId": "test-user-id"
          },
          "virtualHostedBucketName": "${bucket}.oss-${region}.aliyuncs.com"
        },
        "object": {
          "deltaSize": null,
          "eTag": "${etag}",
          "key": "${object_key}",
          "size": ${file_size}
        },
        "ossSchemaVersion": "1.0",
        "ruleId": "test-rule-id"
      },
      "region": "${region}",
      "requestParameters": {
        "sourceIPAddress": "127.0.0.1"
      },
      "responseElements": {
        "requestId": "${request_id}"
      },
      "userIdentity": {
        "principalId": "test-user-id"
      }
    }
  ]
}
EOF
}

# 发送请求到 /invoke 端点
send_request() {
    local bucket="$1"
    local object_key="$2"
    local region="$3"
    local event_name="$4"
    
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}发送 OSS Event 到 /invoke 端点${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    # 检查容器是否运行
    if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo -e "${YELLOW}容器未运行，正在启动...${NC}"
        run_container
    fi
    
    # 生成 OSS Event JSON
    local event_json=$(generate_oss_event "${bucket}" "${object_key}" "${region}" "${event_name}")
    
    # 显示请求信息
    echo -e "${GREEN}请求信息:${NC}"
    echo -e "  • Bucket: ${bucket}"
    echo -e "  • Object Key: ${object_key}"
    echo -e "  • Region: ${region}"
    echo -e "  • Event Name: ${event_name}"
    echo -e "  • Request ID: ${FC_REQUEST_ID}"
    echo ""
    
    # 显示请求体（前500字符）
    echo -e "${BLUE}请求体（前500字符）:${NC}"
    echo "${event_json}" | head -c 500
    echo "..."
    echo ""
    
    # 获取容器端口
    local host_port=$(docker port "${CONTAINER_NAME}" 9000/tcp 2>/dev/null | cut -d: -f2)
    
    if [[ -z "${host_port}" ]]; then
        echo -e "${RED}错误: 无法获取容器端口${NC}"
        echo -e "${YELLOW}请检查容器是否正常运行:${NC}"
        docker ps -a | grep "${CONTAINER_NAME}" || echo "容器不存在"
        exit 1
    fi
    
    local url="http://localhost:${host_port}/invoke"
    
    echo -e "${BLUE}发送 POST 请求到: ${url}${NC}"
    echo ""
    
    # 检查 curl 是否可用
    if ! command -v curl &> /dev/null; then
        echo -e "${RED}错误: curl 未安装${NC}"
        exit 1
    fi
    
    # 发送请求（增加超时设置）
    local response=$(curl -s -w "\n%{http_code}" \
        --max-time 300 \
        -X POST \
        -H "Content-Type: application/json" \
        -H "x-fc-request-id: ${FC_REQUEST_ID}" \
        -d "${event_json}" \
        "${url}" 2>&1)
    
    # 分离响应体和状态码
    local http_code=$(echo "${response}" | tail -n1)
    local response_body=$(echo "${response}" | head -n-1)
    
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}响应结果${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "HTTP 状态码: ${http_code}"
    echo ""
    echo -e "${GREEN}响应体:${NC}"
    if command -v jq &> /dev/null; then
        echo "${response_body}" | jq '.' 2>/dev/null || echo "${response_body}"
    else
        echo "${response_body}"
        echo -e "${YELLOW}提示: 安装 jq 可以获得更好的 JSON 格式化输出${NC}"
    fi
    echo ""
    
    # 显示容器日志（最后20行）
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}容器日志（最后20行）${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    docker logs "${CONTAINER_NAME}" --tail 20
    echo ""
    
    if [[ "${http_code}" == "200" ]]; then
        echo -e "${GREEN}✓ 请求成功${NC}"
    else
        echo -e "${RED}✗ 请求失败 (HTTP ${http_code})${NC}"
        exit 1
    fi
}

# 主函数
main() {
    # 处理清理操作
    if [[ "$CLEANUP" == true ]]; then
        cleanup
        exit 0
    fi
    
    # 处理停止操作
    if [[ "$STOP_CONTAINER" == true ]]; then
        stop_container
    fi
    
    # 处理构建操作
    if [[ "$BUILD_IMAGE" == true ]]; then
        build_image
    fi
    
    # 处理运行容器操作
    if [[ "$RUN_CONTAINER" == true ]]; then
        run_container
        exit 0
    fi
    
    # 发送请求需要参数
    if [[ -z "$BUCKET" ]] || [[ -z "$OBJECT_KEY" ]] || [[ -z "$REGION" ]]; then
        echo -e "${YELLOW}提示: 发送请求需要提供 --bucket, --key, --region 参数${NC}"
        echo ""
        show_help
        exit 0
    fi
    
    # 发送请求
    send_request "${BUCKET}" "${OBJECT_KEY}" "${REGION}" "${EVENT_NAME}"
}

# 设置退出时清理（仅在非交互模式下）
# 注意：如果用户手动停止（Ctrl+C），也会触发清理
# 如果需要保留容器，可以在运行前设置 CLEANUP_ON_EXIT=false
if [[ "${CLEANUP_ON_EXIT:-true}" == "true" ]]; then
    trap 'if [[ $? -ne 0 ]] && [[ "$CLEANUP" != true ]] && [[ "$STOP_CONTAINER" != true ]]; then cleanup; fi' EXIT
fi

# 运行主函数
main
