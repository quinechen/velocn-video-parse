# Docker 编译指南

## 概述

项目已配置为**默认使用 Docker 编译**，确保编译出的二进制文件是 Linux x86_64 格式，可以在函数计算上运行。

## 为什么使用 Docker 编译？

1. **跨平台兼容性**
   - macOS/Windows 编译的二进制文件无法在 Linux（函数计算）上运行
   - Docker 确保编译环境与运行环境一致

2. **依赖管理**
   - 自动包含 FFmpeg 开发库
   - 无需在本地安装依赖

3. **环境一致性**
   - 每次编译使用相同的环境
   - 避免"在我机器上能跑"的问题

## 使用方法

### 默认编译（推荐）

```bash
make video-parse
```

这会自动使用 Docker 编译，生成 Linux x86_64 二进制文件。

### 手动使用 Docker

```bash
# 构建 Docker 镜像
docker build -f Dockerfile.build -t video-parse-builder .

# 编译并复制二进制文件
docker run --rm \
  -v $(pwd)/code/target:/output \
  video-parse-builder \
  sh -c "cp /workspace/target/release/video-parse /output/main && chmod +x /output/main"
```

### 本地编译（仅用于开发）

如果需要本地编译（例如调试），可以使用：

```bash
make build-local
```

**注意**：macOS/Windows 编译的二进制文件无法在函数计算上运行！

## 编译流程

1. **构建 Docker 镜像**
   - 基于 `rust:1.75-slim`
   - 安装 FFmpeg 开发库
   - 配置编译环境

2. **编译依赖**
   - 利用 Docker 缓存层
   - 只编译一次依赖，加快后续编译

3. **编译项目**
   - 复制源代码
   - 编译 Rust 项目
   - 生成 Linux x86_64 二进制

4. **输出文件**
   - 复制到 `code/target/main`
   - 设置执行权限

## 验证编译结果

编译完成后，验证二进制文件：

```bash
# 检查文件类型
file code/target/main

# 应该显示：
# ELF 64-bit LSB executable, x86-64, version 1 (SYSV), dynamically linked, ...
```

## 故障排查

### 问题：Docker 未安装

**错误信息**：
```
错误: Docker 未安装或未启动，请先安装 Docker
```

**解决方法**：
1. 安装 Docker Desktop（macOS/Windows）
2. 或安装 Docker Engine（Linux）
3. 确保 Docker 服务正在运行

### 问题：Docker 构建失败

**可能原因**：
- 网络问题（下载依赖失败）
- Docker 镜像拉取失败
- FFmpeg 安装失败

**解决方法**：
```bash
# 查看详细错误信息
docker build -f Dockerfile.build -t video-parse-builder . --no-cache

# 检查 Docker 是否正常运行
docker ps
```

### 问题：二进制文件不存在

**可能原因**：
- 编译失败
- 文件复制失败
- 路径问题

**解决方法**：
```bash
# 检查编译日志
docker build -f Dockerfile.build -t video-parse-builder . 2>&1 | tee build.log

# 手动运行容器检查
docker run --rm -it video-parse-builder sh
# 在容器内检查：
# ls -lh /workspace/target/release/video-parse
```

### 问题：编译速度慢

**优化方法**：
1. **利用 Docker 缓存**（已配置）
   - 依赖只编译一次
   - 只有源代码变化时才重新编译

2. **使用本地缓存**：
   ```bash
   # Docker 会自动使用缓存层
   # 如果 Cargo.toml 未变化，依赖不会重新编译
   ```

3. **并行编译**：
   - Docker 构建已使用多核编译
   - 可以通过环境变量调整：
     ```bash
     docker build --build-arg CARGO_BUILD_JOBS=4 ...
     ```

## 性能优化

### Docker 缓存策略

Dockerfile.build 使用了多层缓存：

1. **基础镜像层**：Rust 和系统依赖
2. **依赖编译层**：Cargo 依赖（只在 Cargo.toml 变化时重新编译）
3. **源代码编译层**：项目代码（只在源代码变化时重新编译）

### 加快首次编译

首次编译会较慢，因为需要：
- 下载 Rust 镜像
- 安装 FFmpeg 开发库
- 编译所有依赖

后续编译会快很多，因为：
- 依赖已缓存
- 只编译变化的代码

## 与 s.yaml 集成

`s.yaml` 中的 `pre-deploy` 会自动调用：

```yaml
actions:    
  pre-${regex('deploy|local')}:
    - run: make video-parse
```

这会自动使用 Docker 编译，确保部署的二进制文件是正确的格式。

## 环境要求

- **Docker**：版本 20.10+
- **磁盘空间**：至少 2GB（用于 Docker 镜像和编译缓存）
- **内存**：建议 4GB+（编译需要内存）

## 总结

- ✅ **默认使用 Docker 编译**：`make video-parse`
- ✅ **自动生成 Linux 二进制**：可在函数计算上运行
- ✅ **无需本地依赖**：FFmpeg 在 Docker 中自动安装
- ✅ **利用缓存加速**：后续编译更快

只需运行 `make video-parse`，即可获得可在函数计算上运行的二进制文件！
