# 视频拉片工具 (Video Parse)

一个用 Rust 编写的视频分析工具，用于自动检测视频中的场景变化（镜头切换），提取关键帧，并生成详细的元数据。

## 功能特性

- 🎬 **场景检测**：自动检测视频中的场景变化（镜头切换）
- 🖼️ **关键帧提取**：从每个场景中提取代表性关键帧
- 🎵 **音频提取**：提取视频中的音频轨道
- 📊 **元数据生成**：生成包含场景信息、时间戳等详细元数据
- ☁️ **OSS 集成**：支持阿里云 OSS 事件触发，自动处理上传的视频
- 🌐 **HTTP API**：提供 RESTful API 接口，支持本地和云端部署

## 项目结构

```
velocn-video-parse/
├── lib-video-parse/          # 源代码目录
│   ├── src/                  # Rust 源代码
│   ├── scripts/              # 工具脚本
│   ├── dist/                 # 编译输出目录
│   └── Cargo.toml            # Rust 项目配置
├── debug/                    # 测试和调试文件
│   ├── input.mp4            # 测试输入视频
│   ├── output/               # 测试输出目录
│   └── examples/             # 示例文件
├── docs/                     # 项目文档
├── Dockerfile                # Docker 镜像构建文件
├── Makefile                  # 构建脚本
└── s.yaml                    # Serverless Devs 配置文件
```

## 快速开始

### 1. 安装依赖

```bash
# 安装 FFmpeg 和编译依赖
make install-deps
```

### 2. 编译项目

```bash
# 本地编译（默认）
make build
```

### 3. 运行测试

```bash
# 运行单元测试
make test
```

### 4. 演示处理视频

```bash
# 处理测试视频文件
# 输入: debug/input.mp4
# 输出: debug/output/
make demo
```

### 5. 启动 HTTP API 服务器

有两种方式启动本地服务器：

#### 方式1: 直接运行（开发模式）

```bash
# 启动本地服务器（默认端口 9000）
make serve
```

服务器启动后，可以通过以下端点访问：

- **健康检查**: `GET http://localhost:9000/health`
- **OSS事件处理**: `POST http://localhost:9000/process`
- **直接处理**: `POST http://localhost:9000/process/direct`
- **查询处理**: `GET http://localhost:9000/process/query?input=<path>`

#### 方式2: 使用 Serverless Devs 本地调试（函数计算模式）

```bash
# 使用函数计算本地调试环境（模拟云端环境）
make local
```

这种方式会：
- 使用 Docker 容器运行函数（与云端环境一致）
- 自动注入函数计算环境变量
- 支持 OSS 事件触发
- 提供函数计算格式的 HTTP 端点

详细说明请参考 [Serverless Devs 本地调试文档](https://github.com/devsapp/fc/blob/main/docs/zh/command/local.md)。

## Make 命令说明

| 命令 | 说明 |
|------|------|
| `make install-deps` | 安装依赖（FFmpeg 等） |
| `make build` | 本地编译 Rust 项目 |
| `make test` | 运行 Rust 单元测试 |
| `make build-image` | 构建 Docker 镜像 |
| `make deploy` | 一键构建、推送、部署到云服务 |
| `make demo` | 处理测试视频文件 (debug/input.mp4 → debug/output) |
| `make serve` | 启动本地 HTTP API 服务器（开发模式） |
| `make local` | 启动本地函数计算调试环境（使用 s local start） |

## 使用示例

### CLI 模式

```bash
# 处理本地视频文件
./lib-video-parse/dist/main process \
  --input debug/input.mp4 \
  --output debug/output

# 使用自定义参数
./lib-video-parse/dist/main process \
  --input video.mp4 \
  --output output \
  --threshold 0.3 \
  --sample-rate 2.0 \
  --min-scene-duration 2.0
```

### HTTP API 模式

#### 1. 启动服务器

```bash
make serve
```

#### 2. 健康检查

```bash
curl http://localhost:9000/health
```

#### 3. 处理 OSS 事件

```bash
curl -X POST http://localhost:9000/process \
  -H "Content-Type: application/json" \
  -d @debug/examples/oss_event_example.json
```

#### 4. 直接处理视频

```bash
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{
    "input": "debug/input.mp4",
    "output": "debug/output"
  }'
```

#### 5. 查询参数处理

```bash
curl "http://localhost:9000/process/query?input=debug/input.mp4&output=debug/output"
```

## 部署到阿里云函数计算

### 前置要求

1. 安装 [Serverless Devs CLI](https://www.serverless-devs.com/)
2. 配置阿里云访问凭证
3. 配置容器镜像服务命名空间

### 部署步骤

```bash
# 一键部署（构建 + 推送 + 部署）
make deploy
```

部署过程包括：
1. 构建 Docker 镜像
2. 推送镜像到容器镜像服务
3. 部署函数到阿里云函数计算

### 配置说明

在 `s.yaml` 中配置：

- **命名空间**: 修改 `vars.namespace` 为您的容器镜像服务命名空间
- **目标 Bucket**: 配置 `DESTINATION_BUCKET` 和 `DESTINATION_REGION`
- **环境变量**: 根据需要配置其他环境变量

详细配置说明请参考 [docs/docker_image_deployment.md](./docs/docker_image_deployment.md)。

## 项目文档

所有详细文档都在 [docs](./docs/) 目录下：

- **[项目说明](./docs/video_parse_readme.md)** - 完整的功能介绍和使用指南
- **[架构设计](./docs/architecture.md)** - 系统架构和模块设计
- **[Web 服务模式](./docs/web_service.md)** - HTTP 服务器模式使用指南
- **[API 端点](./docs/api_endpoints.md)** - API 接口文档
- **[参数优化指南](./docs/optimize_readme.md)** - 参数优化脚本使用说明
- **[配置文档](./docs/configuration.md)** - 配置文件说明
- **[构建指南](./docs/build_guide.md)** - 编译和构建说明
- **[Docker 镜像部署](./docs/docker_image_deployment.md)** - Docker 镜像部署指南
- **[OSS 事件处理](./docs/oss_event_processing_guide.md)** - OSS 事件处理指南

更多文档请查看 [docs](./docs/) 目录。

## 开发指南

### 本地开发

#### 方式1: 直接运行（快速开发）

```bash
# 1. 安装依赖
make install-deps

# 2. 编译项目
make build

# 3. 运行测试
make test

# 4. 启动服务器
make serve
```

#### 方式2: 函数计算本地调试（模拟云端环境）

```bash
# 1. 构建 Docker 镜像
make build-image

# 2. 启动本地调试环境
make local
```

使用 `make local` 的优势：
- ✅ 完全模拟函数计算环境
- ✅ 自动注入环境变量（如 `FC_SERVER_PORT`、`DESTINATION_BUCKET` 等）
- ✅ 支持 OSS 事件触发测试
- ✅ 与云端部署环境一致，便于问题排查

**本地调试端点**：

启动后，Serverless Devs 会显示本地访问地址，例如：
```
HttpTrigger http://localhost:7001
```

可以通过以下方式调用：

```bash
# 健康检查
curl http://localhost:7001/health

# OSS 事件处理
curl -X POST http://localhost:7001/process \
  -H "Content-Type: application/json" \
  -d @debug/examples/oss_event_example.json

# 直接处理
curl -X POST http://localhost:7001/process/direct \
  -H "Content-Type: application/json" \
  -d '{"input": "debug/input.mp4", "output": "debug/output"}'
```

**注意事项**：
- 本地调试需要 Docker 运行
- 环境变量从 `s.yaml` 中的 `environmentVariables` 读取
- 调试完成后按 `Ctrl+C` 停止

详细说明请参考 [Serverless Devs 本地调试文档](https://github.com/devsapp/fc/blob/main/docs/zh/command/local.md)。

### 测试视频处理

```bash
# 将测试视频放到 debug/input.mp4
# 运行演示
make demo
```

### 调试模式

设置环境变量 `DEBUG=true` 可以启用调试模式，跳过实际处理：

```bash
DEBUG=true make serve
```

## 许可证

MIT License

