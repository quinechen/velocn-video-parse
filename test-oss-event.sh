#!/bin/bash

set -e
# ============================================
# OSS 事件测试脚本
# ============================================
# 功能：
#   1. 查找运行中的容器
#   2. 构造 OSS Event JSON
#   3. 发送请求到 /invoke 端点
#   4. 显示处理日志
#
# 使用方法：
#   ./test-oss-event.sh --bucket my-bucket --key path/to/video.mp4 --region cn-hangzhou
#
# 环境变量：
#   DOCKER_PORT                       - Docker 端口映射（默认：9000:9000）
#   CONTAINER_NAME                    - 容器名称（如果指定，将使用该容器；否则自动查找）
#   FC_REQUEST_ID                     - 请求ID（默认：自动生成）
#
# 注意：容器管理已迁移到 Makefile，使用以下命令：
#   make container-build              # 构建镜像
#   make container-run                # 启动容器
#   make container-stop               # 停止容器
#   make container-cleanup            # 清理容器和镜像
#   make container-logs               # 查看容器日志
# ============================================

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 默认配置
DOCKER_PORT="${DOCKER_PORT:-9000:9000}"
FC_REQUEST_ID="${FC_REQUEST_ID:-$(uuidgen 2>/dev/null || echo "test-$(date +%s)")}"

# 解析命令行参数
BUCKET=""
OBJECT_KEY=""
REGION=""
EVENT_NAME="ObjectCreated:Put"
CONTAINER_NAME=""

# 显示帮助信息
show_help() {
    cat << EOF
用法: $0 [选项]

选项:
  -b, --bucket BUCKET            OSS Bucket 名称（必需）
  -k, --key KEY                  OSS Object Key（文件路径，必需）
  -r, --region REGION            OSS Region（必需，例如：cn-hangzhou）
  -e, --event EVENT_NAME         事件名称（默认：ObjectCreated:Put）
  -c, --container CONTAINER_NAME 容器名称（可选，默认自动查找）
  -h, --help                     显示此帮助信息

环境变量:
  DOCKER_PORT                    端口映射（默认：9000:9000）
  CONTAINER_NAME                 容器名称（如果设置，将使用该容器）
  FC_REQUEST_ID                  请求ID（默认：自动生成）

容器管理（使用 Makefile）:
  make container-build            构建 Docker 镜像
  make container-run              启动容器（需要设置 ALIBABA_CLOUD_ACCESS_KEY_ID 和 ALIBABA_CLOUD_ACCESS_KEY_SECRET）
  make container-stop             停止并删除容器
  make container-cleanup          清理容器和镜像
  make container-logs             查看容器实时日志

示例:
  # 发送测试事件（自动查找容器）
  $0 --bucket my-bucket --key videos/test.mp4 --region cn-hangzhou

  # 指定容器名称
  $0 --bucket my-bucket --key videos/test.mp4 --region cn-hangzhou --container video-parse-test-1234567890
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
        -c|--container)
            CONTAINER_NAME="$2"
            shift 2
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

# 查找运行中的容器
find_container() {
    # 如果指定了容器名，直接使用
    if [[ -n "${CONTAINER_NAME}" ]]; then
        if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
            echo "${CONTAINER_NAME}"
            return 0
        else
            echo -e "${RED}错误: 指定的容器 ${CONTAINER_NAME} 未运行${NC}"
            return 1
        fi
    fi
    
    # 自动查找运行中的容器
    local containers=$(docker ps --format '{{.Names}}' | grep "^video-parse-test-" || true)
    
    if [[ -z "${containers}" ]]; then
        echo -e "${RED}错误: 未找到运行中的容器${NC}"
        echo -e "${YELLOW}请先启动容器:${NC}"
        echo -e "  make container-run"
        return 1
    fi
    
    # 如果找到多个容器，使用第一个
    local first_container=$(echo "${containers}" | head -n1)
    
    # 如果只有一个容器，直接返回
    local container_count=$(echo "${containers}" | wc -l | tr -d ' ')
    if [[ $container_count -eq 1 ]]; then
        echo "${first_container}"
        return 0
    fi
    
    # 多个容器，提示用户选择
    echo -e "${YELLOW}找到多个运行中的容器:${NC}"
    echo "${containers}" | nl -w2 -s'. '
    echo ""
    echo -e "${YELLOW}使用第一个容器: ${first_container}${NC}"
    echo -e "${YELLOW}提示: 使用 --container 参数指定容器名称${NC}"
    echo "${first_container}"
    return 0
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
    
    # 查找运行中的容器
    local container=$(find_container)
    if [[ $? -ne 0 ]]; then
        exit 1
    fi
    
    CONTAINER_NAME="${container}"
    echo -e "${GREEN}✓ 使用容器: ${CONTAINER_NAME}${NC}"
    echo ""
    
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
    
    # 获取容器端口（处理不同的输出格式）
    # 优先从 DOCKER_PORT 环境变量解析（格式: host_port:container_port）
    local host_port=""
    if [[ -n "${DOCKER_PORT}" ]]; then
        host_port=$(echo "${DOCKER_PORT}" | cut -d: -f1)
    fi
    
    # 如果无法从环境变量获取，尝试从 docker port 命令获取
    local port_info=""
    if [[ -z "${host_port}" ]]; then
        port_info=$(docker port "${CONTAINER_NAME}" 9000/tcp 2>/dev/null | head -n1)
        if [[ -n "${port_info}" ]]; then
            # docker port 输出格式: 9000/tcp -> 0.0.0.0:9000 或 9000/tcp -> [::]:9000
            # 提取端口号（最后一个冒号后的数字）
            host_port=$(echo "${port_info}" | grep -oE ':[0-9]+$' | cut -d: -f2)
        fi
    else
        # 如果从环境变量获取成功，也获取端口映射详情用于显示
        port_info=$(docker port "${CONTAINER_NAME}" 9000/tcp 2>/dev/null | head -n1)
    fi
    
    if [[ -z "${host_port}" ]]; then
        echo -e "${RED}错误: 无法获取容器端口${NC}"
        echo -e "${YELLOW}请检查容器是否正常运行:${NC}"
        docker ps -a | grep "${CONTAINER_NAME}" || echo "容器不存在"
        echo ""
        echo -e "${YELLOW}调试信息:${NC}"
        echo "端口映射信息: ${port_info:-无}"
        docker port "${CONTAINER_NAME}" 2>&1 || echo "无法获取端口映射"
        exit 1
    fi
    
    local url="http://localhost:${host_port}/invoke"
    
    echo -e "${BLUE}发送 POST 请求到: ${url}${NC}"
    echo -e "${BLUE}容器端口映射: 9000 -> ${host_port}${NC}"
    if [[ -n "${port_info}" ]]; then
        echo -e "${BLUE}端口映射详情: ${port_info}${NC}"
    fi
    echo ""
    
    # 检查 curl 是否可用
    if ! command -v curl &> /dev/null; then
        echo -e "${RED}错误: curl 未安装${NC}"
        exit 1
    fi
    
    # 先测试连接是否正常（增加重试机制）
    echo -e "${YELLOW}测试连接（最多重试5次）...${NC}"
    local connect_success=false
    for i in {1..5}; do
        if curl -s -f --max-time 3 "http://localhost:${host_port}/health" >/dev/null 2>&1; then
            connect_success=true
            break
        fi
        if [[ $i -lt 5 ]]; then
            echo -e "${YELLOW}  重试 ${i}/5...${NC}"
            sleep 1
        fi
    done
    
    if [[ "$connect_success" != "true" ]]; then
        echo -e "${RED}错误: 无法连接到容器服务${NC}"
        echo ""
        echo -e "${YELLOW}诊断信息:${NC}"
        echo -e "  1. 容器状态:"
        docker ps -a | grep "${CONTAINER_NAME}" || echo "    容器不存在"
        echo ""
        echo -e "  2. 端口映射:"
        docker port "${CONTAINER_NAME}" 2>&1 || echo "    无法获取端口映射"
        echo ""
        echo -e "  3. 容器日志（最后20行）:"
        docker logs "${CONTAINER_NAME}" --tail 20 2>&1 || echo "    无法获取日志"
        echo ""
        echo -e "  4. 手动测试连接:"
        echo -e "    curl -v http://localhost:${host_port}/health"
        echo ""
        exit 1
    fi
    echo -e "${GREEN}✓ 连接测试通过${NC}"
    echo ""
    
    # 发送请求（使用临时文件分离响应体和状态码，兼容 macOS）
    local temp_response=$(mktemp)
    local temp_code=$(mktemp)
    local curl_exit_code=0
    
    # 发送请求，将响应体保存到文件，状态码单独保存
    curl -s -w "%{http_code}" \
        --max-time 300 \
        -X POST \
        -H "Content-Type: application/json" \
        -H "x-fc-request-id: ${FC_REQUEST_ID}" \
        -d "${event_json}" \
        -o "${temp_response}" \
        "${url}" > "${temp_code}" 2>&1 || curl_exit_code=$?
    
    # 读取状态码和响应体
    local http_code=$(cat "${temp_code}" | tr -d '\n\r' || echo "000")
    local response_body=$(cat "${temp_response}")
    
    # 清理临时文件
    rm -f "${temp_response}" "${temp_code}"
    
    # 如果 curl 失败，http_code 可能是 000
    if [[ $curl_exit_code -ne 0 ]] || [[ -z "${http_code}" ]] || [[ "${http_code}" == "000" ]]; then
        echo -e "${RED}⚠ curl 请求可能失败，退出码: ${curl_exit_code}${NC}"
        if [[ -n "${response_body}" ]]; then
            echo -e "${YELLOW}响应内容:${NC}"
            echo "${response_body}"
        fi
        echo ""
        # 如果状态码为空或000，尝试从响应中提取
        if [[ -z "${http_code}" ]] || [[ "${http_code}" == "000" ]]; then
            http_code="000"
        fi
    fi
    
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
    
    # 显示容器日志（最后50行，确保能看到完整的处理日志）
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}容器日志（请求后立即查看，最后50行）${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    docker logs "${CONTAINER_NAME}" --tail 50
    echo ""
    
    # 如果是异步处理，等待一段时间让日志输出
    if [[ "${http_code}" == "200" ]]; then
        echo -e "${GREEN}✓ 请求成功${NC}"
        echo -e "${YELLOW}等待异步任务执行并输出日志（5秒）...${NC}"
        sleep 5
        
        echo ""
        echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${BLUE}容器日志（等待后，最后100行）${NC}"
        echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        docker logs "${CONTAINER_NAME}" --tail 100
        echo ""
        echo -e "${YELLOW}提示: 使用以下命令实时查看日志:${NC}"
        echo -e "  docker logs -f ${CONTAINER_NAME}"
    else
        echo -e "${RED}✗ 请求失败 (HTTP ${http_code})${NC}"
        exit 1
    fi
}

# 主函数
main() {
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

# 运行主函数
main
