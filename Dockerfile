# 通用 Dockerfile - 支持本地运行和云上部署
# 
# 构建镜像：
#   docker build -t video-parse:latest .
#
# 本地运行：
#   docker run -p 9000:9000 video-parse:latest
#
# 推送到阿里云容器镜像服务：
#   docker tag video-parse:latest registry.cn-hangzhou.aliyuncs.com/<namespace>/video-parse:latest
#   docker push registry.cn-hangzhou.aliyuncs.com/<namespace>/video-parse:latest

# ============================================
# 第一阶段：编译阶段
# ============================================
FROM rust:1.88-slim AS builder

# 安装 FFmpeg 开发库和编译工具
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ffmpeg \
        libavcodec-dev \
        libavformat-dev \
        libavutil-dev \
        libavfilter-dev \
        libavdevice-dev \
        libswscale-dev \
        libswresample-dev \
        pkg-config \
        ca-certificates \
        build-essential \
        libssl-dev \
        libclang-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# 先复制 Cargo 文件（利用 Docker 缓存层）
COPY lib-video-parse/Cargo.toml ./
COPY lib-video-parse/Cargo.lock* ./

# 创建空的 src 目录用于依赖缓存
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# 编译依赖（利用 Docker 缓存）
RUN cargo build --release 2>&1 || ( \
        echo "警告: Cargo.lock 版本不兼容，删除并重新生成..." && \
        rm -f Cargo.lock && \
        cargo build --release \
    ) && \
    rm -rf src

# 复制实际源代码
COPY lib-video-parse/src ./src

# 重新编译（只编译我们的代码，依赖已缓存）
RUN touch src/main.rs src/lib.rs && \
    cargo build --release

# ============================================
# 第二阶段：运行阶段
# ============================================
FROM debian:bookworm-slim

# 安装 FFmpeg 运行时库（不需要开发库）
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ffmpeg \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 验证 FFmpeg 安装
RUN ffmpeg -version | head -1

# 创建应用目录
WORKDIR /code

# 从编译阶段复制二进制文件
COPY --from=builder /workspace/target/release/video-parse /code/main

# 确保二进制文件有执行权限
RUN chmod +x /code/main

# 复制配置文件到容器内的 /code 目录
# 注意：配置文件应该在构建时已经存在于项目根目录（由 Makefile 的 prepare-config 目标处理）
# 如果文件不存在，构建会失败，请确保运行 make deploy 或 make build-image（会自动准备配置文件）
COPY video-parse.ini /code/video-parse.ini

# 设置环境变量
# FC_SERVER_PORT: 函数计算环境变量，如果未设置则使用默认值 9000
ENV FC_SERVER_PORT=9000

# 暴露端口
# 本地运行：映射到主机端口
# 云上部署：函数计算会自动处理
EXPOSE 9000

# 启动命令：运行 serve 子命令
# 程序会自动从环境变量 FC_SERVER_PORT 读取端口，或使用默认值 9000
CMD ["/code/main", "serve"]
