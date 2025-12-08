.PHONY: install-deps build test build-image deploy demo serve local

# Rust 项目路径
RUST_PROJECT := lib-video-parse
# 输出目录（二进制文件输出到 lib-video-parse/dist）
OUTPUT_DIR := lib-video-parse/dist
# 输出二进制文件名
BINARY_NAME := main
# Rust 编译后的二进制文件名（根据 Cargo.toml 中的 package name）
RUST_BINARY := video-parse
# Docker 构建镜像名称
DOCKER_BUILDER_IMAGE := video-parse-builder
# Docker 运行镜像名称
DOCKER_RUNTIME_IMAGE := video-parse
# Docker 镜像标签
DOCKER_TAG := latest
# 容器镜像服务地址（需要根据实际情况修改）
# 格式：registry.cn-hangzhou.aliyuncs.com/<namespace>/<image-name>
CONTAINER_REGISTRY := registry.cn-hangzhou.aliyuncs.com
# 命名空间（需要根据实际情况修改）
CONTAINER_NAMESPACE := your-namespace
# 完整镜像地址
FULL_IMAGE_NAME := $(CONTAINER_REGISTRY)/$(CONTAINER_NAMESPACE)/$(DOCKER_RUNTIME_IMAGE):$(DOCKER_TAG)

# 默认目标：本地编译
.DEFAULT_GOAL := build

# 本地编译（默认）
build: build-local

# 本地编译（仅用于开发，macOS/Windows 编译的二进制无法在函数计算上运行）
build-local:
	@echo "=========================================="
	@echo "本地编译 Rust 项目（macOS 优化版）"
	@echo "=========================================="
	@echo "检测编译环境..."
	@OS=$$(uname -s); \
	if [ "$$OS" = "Darwin" ]; then \
		echo "✓ 检测到 macOS 系统"; \
		echo ""; \
		echo "检查必要依赖..."; \
		if ! command -v brew > /dev/null; then \
			echo "错误: 未检测到 Homebrew，请先安装 Homebrew"; \
			echo "安装命令: /bin/bash -c \"\$$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""; \
			exit 1; \
		fi; \
		echo "✓ Homebrew 已安装"; \
		if ! command -v pkg-config > /dev/null; then \
			echo "警告: pkg-config 未安装，正在安装..."; \
			brew install pkg-config || (echo "错误: pkg-config 安装失败" && exit 1); \
		fi; \
		echo "✓ pkg-config 已安装"; \
		if ! pkg-config --exists libavutil 2>/dev/null; then \
			echo "警告: FFmpeg 开发库未找到或未正确配置"; \
			echo "正在检查 FFmpeg 安装状态..."; \
			if ! command -v ffmpeg > /dev/null; then \
				echo "错误: FFmpeg 未安装"; \
				echo ""; \
				echo "请运行以下命令安装 FFmpeg:"; \
				echo "  make install-deps"; \
				echo "或手动安装:"; \
				echo "  brew install ffmpeg pkg-config"; \
				exit 1; \
			fi; \
			echo "✓ FFmpeg 已安装，但 pkg-config 无法找到开发库"; \
			echo "尝试设置 PKG_CONFIG_PATH..."; \
			BREW_PREFIX=$$(brew --prefix); \
			if [ -d "$$BREW_PREFIX/lib/pkgconfig" ]; then \
				export PKG_CONFIG_PATH="$$BREW_PREFIX/lib/pkgconfig:$$PKG_CONFIG_PATH"; \
				echo "✓ 已设置 PKG_CONFIG_PATH=$$PKG_CONFIG_PATH"; \
			fi; \
			if ! pkg-config --exists libavutil 2>/dev/null; then \
				echo "警告: 仍然无法找到 libavutil，继续编译（可能会失败）"; \
			fi; \
		else \
			echo "✓ FFmpeg 开发库已正确配置"; \
		fi; \
		if ! command -v clang > /dev/null; then \
			echo "警告: clang 未找到，ffmpeg-next 可能需要 clang 用于 bindgen"; \
			echo "如果编译失败，请安装 Xcode Command Line Tools:"; \
			echo "  xcode-select --install"; \
		else \
			echo "✓ clang 已安装"; \
		fi; \
		echo ""; \
		echo "设置编译环境变量..."; \
		BREW_PREFIX=$$(brew --prefix); \
		export PKG_CONFIG_PATH="$$BREW_PREFIX/lib/pkgconfig:$$PKG_CONFIG_PATH"; \
		export LIBRARY_PATH="$$BREW_PREFIX/lib:$$LIBRARY_PATH"; \
		export C_INCLUDE_PATH="$$BREW_PREFIX/include:$$C_INCLUDE_PATH"; \
		echo "✓ PKG_CONFIG_PATH=$$PKG_CONFIG_PATH"; \
		echo "✓ LIBRARY_PATH=$$LIBRARY_PATH"; \
		echo "✓ C_INCLUDE_PATH=$$C_INCLUDE_PATH"; \
		echo ""; \
	fi
	@echo "开始编译..."
	@mkdir -p $(OUTPUT_DIR)
	@OS=$$(uname -s); \
	if [ "$$OS" = "Darwin" ]; then \
		BREW_PREFIX=$$(brew --prefix 2>/dev/null || echo "/opt/homebrew"); \
		PKG_CONFIG_PATH="$$BREW_PREFIX/lib/pkgconfig:$$PKG_CONFIG_PATH" \
		LIBRARY_PATH="$$BREW_PREFIX/lib:$$LIBRARY_PATH" \
		C_INCLUDE_PATH="$$BREW_PREFIX/include:$$C_INCLUDE_PATH" \
		cd $(RUST_PROJECT) && cargo build --release; \
	else \
		cd $(RUST_PROJECT) && cargo build --release; \
	fi
	@if [ ! -f $(RUST_PROJECT)/target/release/$(RUST_BINARY) ]; then \
		echo ""; \
		echo "错误: 编译后的二进制文件不存在"; \
		echo ""; \
		OS=$$(uname -s); \
		if [ "$$OS" = "Darwin" ]; then \
			echo "macOS 编译故障排查:"; \
			echo "  1. 确保已安装 FFmpeg: make install-deps"; \
			echo "  2. 确保已安装 Xcode Command Line Tools: xcode-select --install"; \
			echo "  3. 检查环境变量是否正确设置"; \
			echo "  4. 查看上面的编译错误信息"; \
		else \
			echo "提示: 如果遇到 FFmpeg 相关错误，请尝试:"; \
			echo "  1. 安装 FFmpeg 开发库: make install-deps"; \
		fi; \
		exit 1; \
	fi
	@cp $(RUST_PROJECT)/target/release/$(RUST_BINARY) $(OUTPUT_DIR)/$(BINARY_NAME)
	@chmod +x $(OUTPUT_DIR)/$(BINARY_NAME)
	@echo ""
	@echo "✓ 编译成功！"
	@echo "二进制文件: $(OUTPUT_DIR)/$(BINARY_NAME)"
	@file $(OUTPUT_DIR)/$(BINARY_NAME) || true
	@ls -lh $(OUTPUT_DIR)/$(BINARY_NAME)
	@echo ""
	@OS=$$(uname -s); \
	if [ "$$OS" = "Darwin" ]; then \
		echo "⚠️  警告: 这是在 macOS 上编译的二进制文件"; \
		echo "   架构: $$(file $(OUTPUT_DIR)/$(BINARY_NAME) | grep -o 'Mach-O.*' || echo '未知')"; \
		echo "   此二进制文件无法在函数计算（Linux）上运行"; \
		echo "   如需部署到函数计算，请使用: make deploy"; \
	else \
		echo "警告: 如果是在 macOS 或 Windows 上编译，此二进制文件无法在函数计算（Linux）上运行"; \
		echo "建议使用: make deploy"; \
	fi

# 安装 FFmpeg 开发库（支持 Ubuntu/Debian 和 macOS）
install-deps:
	@echo "=========================================="
	@echo "安装 FFmpeg 开发库和编译依赖"
	@echo "=========================================="
	@OS=$$(uname -s); \
	if [ "$$OS" = "Darwin" ]; then \
		echo "检测到 macOS 系统，使用 Homebrew 安装..."; \
		if ! command -v brew > /dev/null; then \
			echo "错误: 未检测到 Homebrew"; \
			echo "请先安装 Homebrew:"; \
			echo '  /bin/bash -c "$$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'; \
			exit 1; \
		fi; \
		echo "✓ Homebrew 已安装"; \
		echo ""; \
		echo "检查 Xcode Command Line Tools..."; \
		if ! command -v clang > /dev/null; then \
			echo "警告: clang 未找到，ffmpeg-next 需要 clang 用于 bindgen"; \
			echo "正在安装 Xcode Command Line Tools..."; \
			xcode-select --install || echo "请手动运行: xcode-select --install"; \
		else \
			echo "✓ Xcode Command Line Tools 已安装"; \
		fi; \
		echo ""; \
		echo "安装 FFmpeg 和 pkg-config..."; \
		brew install ffmpeg pkg-config || (echo "错误: FFmpeg 安装失败" && exit 1); \
		echo ""; \
		echo "验证安装..."; \
		BREW_PREFIX=$$(brew --prefix); \
		export PKG_CONFIG_PATH="$$BREW_PREFIX/lib/pkgconfig:$$PKG_CONFIG_PATH"; \
		if pkg-config --exists libavutil; then \
			echo "✓ FFmpeg 开发库已正确安装"; \
			pkg-config --modversion libavutil; \
		else \
			echo "警告: pkg-config 无法找到 libavutil"; \
			echo "尝试设置 PKG_CONFIG_PATH..."; \
			echo "请将以下内容添加到您的 shell 配置文件中（~/.zshrc 或 ~/.bash_profile）:"; \
			echo "  export PKG_CONFIG_PATH=\"$$BREW_PREFIX/lib/pkgconfig:\$$PKG_CONFIG_PATH\""; \
			echo "  export LIBRARY_PATH=\"$$BREW_PREFIX/lib:\$$LIBRARY_PATH\""; \
			echo "  export C_INCLUDE_PATH=\"$$BREW_PREFIX/include:\$$C_INCLUDE_PATH\""; \
		fi; \
		echo ""; \
		echo "✓ macOS 依赖安装完成"; \
	elif command -v apt-get > /dev/null; then \
		echo "检测到 Ubuntu/Debian 系统，使用 apt-get 安装..."; \
		sudo apt-get update && \
		sudo apt-get install -y \
			ffmpeg \
			libavcodec-dev \
			libavformat-dev \
			libavutil-dev \
			libavfilter-dev \
			libavdevice-dev \
			libswscale-dev \
			libswresample-dev \
			pkg-config \
			build-essential \
			libclang-dev || (echo "错误: 依赖安装失败" && exit 1); \
		echo ""; \
		echo "验证安装..."; \
		pkg-config --modversion libavutil || echo "警告: 无法验证 libavutil 版本"; \
		echo "✓ Ubuntu/Debian 依赖安装完成"; \
	else \
		echo "错误: 未检测到支持的包管理器（apt-get 或 brew）"; \
		echo "请手动安装以下依赖:"; \
		echo "  - FFmpeg 开发库"; \
		echo "  - pkg-config"; \
		echo "  - clang (用于 bindgen)"; \
		exit 1; \
	fi
	@echo ""
	@echo "=========================================="
	@echo "依赖安装完成！"
	@echo "=========================================="
	@echo "现在可以运行: make build"

# 运行测试
test:
	@echo "=========================================="
	@echo "运行 Rust 单元测试"
	@echo "=========================================="
	@cd $(RUST_PROJECT) && cargo test --lib
	@echo ""
	@echo "✓ 测试完成"

# 构建 Docker 镜像（多阶段构建，包含编译步骤）
# 支持本地运行和云上部署
build-image:
	@echo "=========================================="
	@echo "构建 Docker 镜像（多阶段构建）"
	@echo "支持本地运行和云上部署"
	@echo "=========================================="
	@echo "检查 Docker 是否可用..."
	@command -v docker > /dev/null || (echo "错误: Docker 未安装或未启动，请先安装 Docker" && exit 1)
	@echo "构建 Docker 镜像（多阶段构建，包含编译）..."
	@docker build -t $(DOCKER_RUNTIME_IMAGE):$(DOCKER_TAG) . || (echo "错误: Docker 镜像构建失败" && exit 1)
	@echo ""
	@echo "✓ Docker 镜像构建成功！"
	@echo "镜像名称: $(DOCKER_RUNTIME_IMAGE):$(DOCKER_TAG)"
	@docker images $(DOCKER_RUNTIME_IMAGE):$(DOCKER_TAG) | tail -1
	@echo ""
	@echo "本地运行:"
	@echo "  docker run -p 9000:9000 $(DOCKER_RUNTIME_IMAGE):$(DOCKER_TAG)"
	@echo ""
	@echo "部署到云服务:"
	@echo "  make deploy"

# 推送镜像到容器镜像服务（内部任务，由 deploy 调用）
push-image:
	@echo "=========================================="
	@echo "推送 Docker 镜像到容器镜像服务"
	@echo "=========================================="
	@command -v docker > /dev/null || (echo "错误: Docker 未安装或未启动，请先安装 Docker" && exit 1)
	@if ! docker images $(DOCKER_RUNTIME_IMAGE):$(DOCKER_TAG) | grep -q $(DOCKER_RUNTIME_IMAGE); then \
		echo "错误: 镜像不存在，请先运行 make build-image"; \
		exit 1; \
	fi
	@echo "标记镜像为容器镜像服务地址..."
	@docker tag $(DOCKER_RUNTIME_IMAGE):$(DOCKER_TAG) $(FULL_IMAGE_NAME) || (echo "错误: 镜像标记失败" && exit 1)
	@echo "推送镜像到 $(CONTAINER_REGISTRY)..."
	@docker push $(FULL_IMAGE_NAME) || (echo "错误: 镜像推送失败" && exit 1)
	@echo ""
	@echo "✓ 镜像推送成功！"
	@echo "镜像地址: $(FULL_IMAGE_NAME)"
	@echo ""
	@echo "提示: 请在 s.yaml 中配置正确的镜像地址"

# 部署到云服务（一键构建、推送、部署）
deploy:
	@echo "=========================================="
	@echo "一键部署到阿里云函数计算"
	@echo "=========================================="
	@echo ""
	@echo "步骤 1/3: 检查环境..."
	@command -v docker > /dev/null || (echo "错误: Docker 未安装或未启动，请先安装 Docker" && exit 1)
	@echo "✓ Docker 已安装"
	@command -v s > /dev/null || (echo "错误: Serverless Devs CLI 未安装，请先安装: npm install -g @serverless-devs/s" && exit 1)
	@echo "✓ Serverless Devs CLI 已安装"
	@echo ""
	@echo "步骤 2/3: 构建并推送 Docker 镜像..."
	@echo "构建 Docker 镜像..."
	@$(MAKE) build-image || (echo "错误: Docker 镜像构建失败" && exit 1)
	@echo ""
	@echo "推送镜像到容器镜像服务..."
	@$(MAKE) push-image || (echo "错误: 镜像推送失败" && exit 1)
	@echo ""
	@echo "步骤 3/3: 部署函数..."
	@s deploy -y || (echo "错误: 函数部署失败" && exit 1)
	@echo ""
	@echo "=========================================="
	@echo "✓ 部署完成！"
	@echo "=========================================="
	@echo ""
	@echo "提示: 可以使用以下命令查看函数信息:"
	@echo "  s info"
	@echo ""
	@echo "提示: 可以使用以下命令查看函数日志:"
	@echo "  s logs --tail 50"

# 演示：处理测试视频文件
demo:
	@echo "=========================================="
	@echo "演示：处理测试视频文件"
	@echo "=========================================="
	@if [ ! -f $(OUTPUT_DIR)/$(BINARY_NAME) ]; then \
		echo "错误: 二进制文件不存在，请先运行 make build"; \
		exit 1; \
	fi
	@if [ ! -f debug/input.mp4 ]; then \
		echo "错误: 输入文件不存在: debug/input.mp4"; \
		echo "提示: 请将测试视频文件放到 debug/input.mp4"; \
		exit 1; \
	fi
	@echo "输入文件: debug/input.mp4"
	@echo "输出目录: debug/output"
	@echo ""
	@mkdir -p debug/output
	@echo "开始处理视频..."
	@$(OUTPUT_DIR)/$(BINARY_NAME) process \
		--input debug/input.mp4 \
		--output debug/output || (echo "" && echo "错误: 视频处理失败" && exit 1)
	@echo ""
	@echo "=========================================="
	@echo "✓ 处理完成！"
	@echo "=========================================="
	@echo ""
	@echo "输出文件:"
	@ls -lh debug/output/ | tail -n +2 | awk '{print "  " $9 " (" $5 ")"}'
	@echo ""
	@if [ -f debug/output/metadata.json ]; then \
		echo "元数据信息:"; \
		cat debug/output/metadata.json | python3 -m json.tool 2>/dev/null || cat debug/output/metadata.json; \
		echo ""; \
	fi

# 启动本地 HTTP API 服务器
serve:
	@echo "=========================================="
	@echo "启动本地 HTTP API 服务器"
	@echo "=========================================="
	@if [ ! -f $(OUTPUT_DIR)/$(BINARY_NAME) ]; then \
		echo "错误: 二进制文件不存在，请先运行 make build"; \
		exit 1; \
	fi
	@echo "服务器地址: http://0.0.0.0:9000"
	@echo ""
	@echo "可用端点:"
	@echo "  • 健康检查: GET  http://localhost:9000/health"
	@echo "  • OSS事件处理: POST http://localhost:9000/process"
	@echo "  • 直接处理: POST http://localhost:9000/process/direct"
	@echo "  • 查询处理: GET  http://localhost:9000/process/query?input=<path>"
	@echo ""
	@echo "按 Ctrl+C 停止服务器"
	@echo "=========================================="
	@echo ""
	@$(OUTPUT_DIR)/$(BINARY_NAME) serve --bind 0.0.0.0:9000

# 本地调试（使用 Serverless Devs local start）
# 用于模拟函数计算环境，支持 OSS 事件触发和 HTTP 调用
local:
	@echo "=========================================="
	@echo "启动本地函数计算调试环境"
	@echo "=========================================="
	@command -v s > /dev/null || (echo "错误: Serverless Devs CLI 未安装，请先安装: npm install -g @serverless-devs/s" && exit 1)
	@echo "✓ Serverless Devs CLI 已安装"
	@echo ""
	@echo "提示: 本地调试会使用 Docker 容器运行函数"
	@echo "提示: 函数计算环境变量会自动注入"
	@echo "提示: 按 Ctrl+C 停止调试"
	@echo ""
	@echo "=========================================="
	@echo ""
	@s local start
