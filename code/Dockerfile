# 通用 Dockerfile - 支持本地运行和云上部署 (Go 版本)
# 
# 构建镜像：
#   docker build -t video-parse-go:latest .
#
# 本地运行：
#   docker run -p 9000:9000 video-parse-go:latest
#
# 推送到阿里云容器镜像服务：
#   docker tag video-parse-go:latest registry.cn-hangzhou.aliyuncs.com/<namespace>/video-parse-go:latest
#   docker push registry.cn-hangzhou.aliyuncs.com/<namespace>/video-parse-go:latest

# ============================================
# 第一阶段：编译阶段
# ============================================
FROM golang:1.21-alpine AS builder

# 安装编译工具
RUN apk add --no-cache git make

WORKDIR /workspace

# 复制 go.mod 和 go.sum（利用 Docker 缓存层）
COPY go.mod go.sum* ./

# 下载依赖（利用 Docker 缓存）
RUN go mod download

# 复制源代码
COPY . .

# 编译 Go 程序（Linux amd64，禁用 CGO）
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o /workspace/main main.go

# ============================================
# 第二阶段：运行阶段
# ============================================
FROM debian:bookworm-slim

# 安装 FFmpeg 运行时库
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ffmpeg \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 验证 FFmpeg 安装
RUN ffmpeg -version | head -1

# 创建应用目录
WORKDIR /code

# 从编译阶段复制二进制文件
COPY --from=builder /workspace/main /code/main

# 确保二进制文件有执行权限
RUN chmod +x /code/main

# 设置环境变量
# FC_SERVER_PORT: 函数计算环境变量，如果未设置则使用默认值 9000
ENV FC_SERVER_PORT=9000

# 暴露端口
# 本地运行：映射到主机端口
# 云上部署：函数计算会自动处理
EXPOSE 9000

# 启动命令
# 注意：Go 版本使用函数计算 SDK，会自动处理 HTTP 请求
CMD ["/code/main"]

