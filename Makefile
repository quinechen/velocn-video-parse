.PHONY: video-parse clean build-docker build-local install-deps

# Rust 项目路径
RUST_PROJECT := video-parse
# 输出目录
OUTPUT_DIR := code/target
# 输出二进制文件名
BINARY_NAME := main
# Rust 编译后的二进制文件名（根据 Cargo.toml 中的 package name）
RUST_BINARY := video-parse
# Docker 镜像名称
DOCKER_IMAGE := video-parse-builder

# 默认使用 Docker 编译（确保 Linux x86_64 二进制，可在函数计算上运行）
video-parse: build-docker

# 使用 Docker 编译（推荐，确保环境一致性和 Linux 二进制）
build-docker:
	@echo "=========================================="
	@echo "使用 Docker 编译 Rust 项目（Linux x86_64）"
	@echo "=========================================="
	@mkdir -p $(OUTPUT_DIR)
	@echo "检查 Docker 是否可用..."
	@command -v docker > /dev/null || (echo "错误: Docker 未安装或未启动，请先安装 Docker" && exit 1)
	@echo "构建 Docker 镜像..."
	@docker build -f Dockerfile.build -t $(DOCKER_IMAGE) . || (echo "错误: Docker 构建失败" && exit 1)
	@echo "在 Docker 容器中编译..."
	@docker run --rm \
		-v $(PWD)/$(OUTPUT_DIR):/output \
		$(DOCKER_IMAGE) \
		sh -c "cp /workspace/target/release/$(RUST_BINARY) /output/$(BINARY_NAME) && chmod +x /output/$(BINARY_NAME)"
	@if [ ! -f $(OUTPUT_DIR)/$(BINARY_NAME) ]; then \
		echo "错误: Docker 编译失败，二进制文件不存在"; \
		exit 1; \
	fi
	@echo ""
	@echo "✓ 编译成功！"
	@echo "二进制文件: $(OUTPUT_DIR)/$(BINARY_NAME)"
	@file $(OUTPUT_DIR)/$(BINARY_NAME) || true
	@ls -lh $(OUTPUT_DIR)/$(BINARY_NAME)
	@echo ""
	@echo "提示: 这是 Linux x86_64 二进制文件，可以在函数计算上运行"

# 本地编译（仅用于开发，macOS/Windows 编译的二进制无法在函数计算上运行）
build-local:
	@echo "=========================================="
	@echo "本地编译 Rust 项目（警告：macOS/Windows 编译的二进制无法在函数计算上运行）"
	@echo "=========================================="
	@mkdir -p $(OUTPUT_DIR)
	@cd $(RUST_PROJECT) && cargo build --release
	@if [ ! -f $(RUST_PROJECT)/target/release/$(RUST_BINARY) ]; then \
		echo "错误: 编译后的二进制文件不存在"; \
		echo ""; \
		echo "提示: 如果遇到 FFmpeg 相关错误，请尝试:"; \
		echo "  1. 安装 FFmpeg 开发库: make install-deps"; \
		echo "  2. 或使用 Docker 编译: make build-docker（推荐）"; \
		exit 1; \
	fi
	@cp $(RUST_PROJECT)/target/release/$(RUST_BINARY) $(OUTPUT_DIR)/$(BINARY_NAME)
	@chmod +x $(OUTPUT_DIR)/$(BINARY_NAME)
	@echo "编译完成，二进制文件已复制到 $(OUTPUT_DIR)/$(BINARY_NAME)"
	@file $(OUTPUT_DIR)/$(BINARY_NAME) || true
	@ls -lh $(OUTPUT_DIR)/$(BINARY_NAME)
	@echo ""
	@echo "警告: 如果是在 macOS 或 Windows 上编译，此二进制文件无法在函数计算（Linux）上运行"
	@echo "建议使用: make build-docker"

# 安装 FFmpeg 开发库（Ubuntu/Debian）
install-deps:
	@echo "安装 FFmpeg 开发库..."
	@if command -v apt-get > /dev/null; then \
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
			pkg-config; \
	elif command -v brew > /dev/null; then \
		brew install ffmpeg pkg-config; \
	else \
		echo "错误: 未检测到包管理器（apt-get 或 brew）"; \
		echo "请手动安装 FFmpeg 开发库"; \
		exit 1; \
	fi
	@echo "FFmpeg 开发库安装完成"
	@pkg-config --modversion libavutil || echo "警告: 无法验证 libavutil 版本"

clean:
	@echo "清理编译产物..."
	@rm -rf $(OUTPUT_DIR)
	@cd $(RUST_PROJECT) && cargo clean
	@echo "清理完成"
