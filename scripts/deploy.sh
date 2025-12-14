#!/bin/bash

# ============================================
# 部署到函数计算脚本
# ============================================

set -euo pipefail

# 加载通用函数
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

log_step "部署到阿里云函数计算"

cd_project_root

echo ""
log_info "步骤 1/4: 检查环境..."

# 检查 Docker
if ! check_command docker; then
    error_exit "Docker 未安装或未启动"
fi
log_success "Docker 已安装"

# 检查 Serverless Devs CLI
if ! check_command s; then
    error_exit "Serverless Devs CLI 未安装\n安装命令: npm install -g @serverless-devs/s"
fi
log_success "Serverless Devs CLI 已安装"

echo ""
log_info "步骤 2/4: 检查 Go 代码..."
cd "${GO_PROJECT}"
if ! go mod tidy; then
    error_exit "go mod tidy 失败"
fi
log_success "Go 依赖检查完成"

echo ""
log_info "步骤 3/4: 使用 Serverless Devs 部署..."
log_info "Serverless Devs 会自动编译和部署 Go 代码"
echo ""

# 回到项目根目录执行部署（s.yaml 在根目录）
cd_project_root

echo ""
log_info "步骤 4/4: 部署函数..."
if ! s deploy -y; then
    error_exit "函数部署失败"
fi

echo ""
log_step "部署完成！"
echo ""
echo "查看函数信息:"
echo "  s info"
echo ""
echo "查看函数日志:"
echo "  s logs --tail 50"
echo ""
