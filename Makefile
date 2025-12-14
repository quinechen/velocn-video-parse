.PHONY: dev test-server docker docker-push deploy install-deps build help

# ============================================
# 项目配置
# ============================================

# 脚本目录
SCRIPTS_DIR := scripts

# ============================================
# 默认目标
# ============================================

.DEFAULT_GOAL := help

# ============================================
# 帮助信息
# ============================================

help:
	@echo "=========================================="
	@echo "Video Parse (Go) - Makefile 命令"
	@echo "=========================================="
	@echo ""
	@echo "可用命令:"
	@echo "  make install-deps    安装 Go、FFmpeg 和编译依赖"
	@echo "  make build           本地编译 Go 二进制文件"
	@echo "  make dev             启动本地开发服务器 (http://localhost:9000)"
	@echo "  make test-server     测试服务器启动和健康检查"
	@echo "  make docker          构建 Docker 镜像"
	@echo "  make docker-push     推送镜像到容器镜像服务"
	@echo "  make deploy          部署到阿里云函数计算（使用 Serverless Devs）"
	@echo ""
	@echo "示例工作流:"
	@echo "  1. 首次使用: make install-deps"
	@echo "  2. 测试服务器: make test-server"
	@echo "  3. 本地开发: make dev"
	@echo "  4. 部署上线: make deploy"
	@echo ""

# ============================================
# 任务定义（调用脚本）
# ============================================

install-deps:
	@$(SCRIPTS_DIR)/install-deps.sh

build:
	@$(SCRIPTS_DIR)/build.sh

dev: build
	@$(SCRIPTS_DIR)/dev.sh

test-server: build
	@$(SCRIPTS_DIR)/test-server.sh

docker:
	@$(SCRIPTS_DIR)/docker.sh

docker-push:
	@$(SCRIPTS_DIR)/docker-push.sh

deploy:
	@$(SCRIPTS_DIR)/deploy.sh
