# Docker 镜像部署指南

本文档说明如何使用 Docker 镜像方式部署视频拉片工具到阿里云函数计算。

## 概述

项目已从 Custom Runtime 模式迁移到容器镜像模式，具有以下优势：

- ✅ **更灵活**：可以完全控制运行环境
- ✅ **更简单**：不需要配置 layers 和 customRuntimeConfig
- ✅ **更可靠**：所有依赖都打包在镜像中，环境一致性好

## 前置要求

1. **Docker**：已安装并运行 Docker
2. **阿里云容器镜像服务**：已开通并创建命名空间
3. **函数计算权限**：具有部署函数的权限

## 快速开始

### 1. 配置容器镜像服务命名空间

编辑 `s.yaml`，修改命名空间：

```yaml
vars:
  namespace: "your-namespace"  # 改为你的命名空间
```

### 2. 构建 Docker 镜像

使用统一的构建命令：

```bash
# 一步完成编译和镜像构建（多阶段构建）
make build-image
```

**说明**：
- Dockerfile 使用多阶段构建，自动完成编译和镜像构建
- 不需要预先编译二进制文件
- 支持本地运行和云上部署

### 3. 推送镜像到容器镜像服务

```bash
# 推送镜像
make push-image
```

**注意**：推送前需要先登录容器镜像服务：

```bash
# 登录阿里云容器镜像服务
docker login --username=<your-username> registry.cn-hangzhou.aliyuncs.com
# 输入密码（或使用访问凭证）
```

### 4. 部署函数

```bash
# 部署到函数计算
s deploy -y
```

## 详细说明

### Dockerfile 说明

项目使用**统一的 Dockerfile**，采用多阶段构建方式：

- **第一阶段**：编译 Rust 项目（包含 FFmpeg 开发库）
- **第二阶段**：运行环境（只包含 FFmpeg 运行时库）

**特点**：
- ✅ 支持本地运行和云上部署
- ✅ 一步完成编译和镜像构建
- ✅ 镜像体积小（只包含运行时依赖）
- ✅ 环境一致性好

### Makefile 命令说明

| 命令 | 说明 |
|------|------|
| `make build-docker` | 编译 Linux 二进制文件到 `dist/main`（用于 Custom Runtime） |
| `make build-image` | 构建 Docker 镜像（多阶段构建，包含编译） |
| `make push-image` | 推送镜像到容器镜像服务 |

### 镜像配置

在 `Makefile` 中可以配置镜像相关参数：

```makefile
# Docker 运行镜像名称
DOCKER_RUNTIME_IMAGE := video-parse
# Docker 镜像标签
DOCKER_TAG := latest
# 容器镜像服务地址
CONTAINER_REGISTRY := registry.cn-hangzhou.aliyuncs.com
# 命名空间（需要根据实际情况修改）
CONTAINER_NAMESPACE := your-namespace
```

### s.yaml 配置说明

使用容器镜像后，`s.yaml` 配置如下：

```yaml
props:
  # 使用容器镜像（替换原来的 code 和 customRuntimeConfig）
  image: registry.cn-hangzhou.aliyuncs.com/${vars.namespace}/video-parse:latest
  
  # 不再需要以下配置：
  # - code: dist
  # - customRuntimeConfig
  # - layers（FFmpeg 已包含在镜像中）
```

## 镜像内容

Docker 镜像包含：

- **基础镜像**：Debian Bookworm Slim
- **FFmpeg**：运行时库（用于视频处理）
- **二进制文件**：`/code/main`（video-parse 程序）
- **启动命令**：`/code/main serve`

## 环境变量

函数计算的环境变量配置保持不变：

```yaml
environmentVariables:
  DEBUG: "true"  # 调试模式
  DESTINATION_BUCKET: "your-destination-bucket"
  DESTINATION_REGION: "cn-hangzhou"
```

## 故障排查

### 镜像构建失败

1. **检查 Docker 是否运行**：
   ```bash
   docker ps
   ```

2. **检查磁盘空间**：
   ```bash
   df -h
   ```

3. **清理 Docker 缓存**：
   ```bash
   docker system prune -a
   ```

### 镜像推送失败

1. **检查登录状态**：
   ```bash
   docker login registry.cn-hangzhou.aliyuncs.com
   ```

2. **检查命名空间是否存在**：
   登录阿里云控制台，确认命名空间已创建

3. **检查权限**：
   确认账号具有推送镜像的权限

### 函数部署失败

1. **检查镜像地址**：
   确认 `s.yaml` 中的镜像地址正确

2. **检查镜像是否存在**：
   在容器镜像服务控制台查看镜像是否已推送

3. **检查函数配置**：
   确认 `runtime` 设置为 `custom`

## 从 Custom Runtime 迁移

如果你之前使用 Custom Runtime 模式，迁移步骤：

1. ✅ 构建 Docker 镜像（已完成）
2. ✅ 更新 `s.yaml` 配置（已完成）
3. ⚠️ 推送镜像到容器镜像服务
4. ⚠️ 更新 `s.yaml` 中的命名空间
5. ⚠️ 部署函数

**注意**：迁移后，不再需要：
- `code` 配置
- `customRuntimeConfig` 配置
- `layers` 配置（FFmpeg 已在镜像中）

## 最佳实践

1. **使用统一的 Dockerfile**：项目使用统一的 Dockerfile，支持本地运行和云上部署

2. **版本标签**：使用版本号作为标签，而不是 `latest`：
   ```bash
   DOCKER_TAG := v1.0.0
   ```

3. **CI/CD 集成**：在 CI/CD 流程中自动构建和推送镜像

4. **镜像大小优化**：使用多阶段构建可以减少最终镜像大小

5. **安全扫描**：定期扫描镜像中的安全漏洞

## 相关文档

- [构建指南](./build_guide.md)
- [配置文档](./configuration.md)

