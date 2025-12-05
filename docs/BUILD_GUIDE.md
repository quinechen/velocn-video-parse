# 编译问题解决方案

## 问题分析

编译错误是因为 `ffmpeg-sys-next` 需要 FFmpeg 开发库，但本地环境缺少这些库。

**错误信息**：
```
The system library `libavutil` required by crate `ffmpeg-sys-next` was not found.
```

## 解决方案

### 方案 1：安装 FFmpeg 开发库（推荐用于本地开发）

#### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libavfilter-dev \
    libavdevice-dev \
    libswscale-dev \
    libswresample-dev \
    pkg-config
```

#### macOS

```bash
brew install ffmpeg pkg-config
```

#### 验证安装

```bash
pkg-config --modversion libavutil
ffmpeg -version
```

### 方案 2：使用 Docker 编译（推荐用于生产部署）

使用 Docker 可以确保编译环境与函数计算环境一致。

#### 创建编译 Dockerfile

创建 `Dockerfile.build`：

```dockerfile
FROM rust:1.75-slim

# 安装 FFmpeg 开发库
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
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# 复制项目文件
COPY video-parse/Cargo.toml video-parse/Cargo.lock ./
COPY video-parse/src ./src

# 编译项目
RUN cargo build --release

# 输出二进制文件
CMD ["cp", "target/release/video-parse", "/output/main"]
```

#### 使用 Docker 编译

```bash
# 编译
docker build -f Dockerfile.build -t video-parse-builder .

# 复制二进制文件
docker create --name temp-container video-parse-builder
docker cp temp-container:/workspace/target/release/video-parse ./code/target/main
docker rm temp-container
chmod +x ./code/target/main
```

### 方案 3：使用 Makefile 的 Docker 编译目标

更新 Makefile 添加 Docker 编译支持。

## 快速解决方案（如果只想测试 DEBUG 模式）

如果你只是想测试 DEBUG 模式（跳过实际处理），可以：

1. **在函数计算环境中编译**（如果支持）
2. **使用预编译的二进制文件**
3. **安装 FFmpeg 开发库**（最快）

### 快速安装（Ubuntu/Debian）

```bash
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
    pkg-config
```

然后重新编译：

```bash
make video-parse
```

## 推荐方案

### 开发环境

**使用方案 1**：安装 FFmpeg 开发库，方便本地开发和调试。

### 生产部署

**使用方案 2**：使用 Docker 编译，确保环境一致性。

## 注意事项

1. **DEBUG 模式仍然需要编译**
   - 即使启用 DEBUG 模式，代码仍然需要编译
   - DEBUG 模式只是跳过运行时处理，不跳过编译

2. **函数计算环境**
   - 函数计算运行时环境已经有 FFmpeg（通过层提供）
   - 但编译时仍然需要 FFmpeg 开发库

3. **交叉编译**
   - 如果本地是 macOS/Windows，需要交叉编译到 Linux
   - 使用 Docker 可以避免交叉编译的复杂性

## 验证编译

编译成功后，验证二进制文件：

```bash
# 检查文件是否存在
ls -lh code/target/main

# 检查文件类型
file code/target/main

# 应该显示：ELF 64-bit LSB executable, x86-64
```

## 故障排查

### 问题：pkg-config 找不到库

**解决方法**：
```bash
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig
```

### 问题：库版本不匹配

**解决方法**：
- 确保安装的 FFmpeg 版本与 `ffmpeg-next = "6.1"` 兼容
- 或者更新 Cargo.toml 中的版本

### 问题：Docker 编译失败

**解决方法**：
- 检查 Docker 是否正常运行
- 检查网络连接
- 查看详细的错误日志
