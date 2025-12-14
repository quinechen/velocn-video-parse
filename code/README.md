# 视频拉片工具 - Go 版本

这是从 Rust 版本转换而来的 Go 实现，用于分析视频内容，提取关键帧和场景信息。

## 功能特性

- 🎬 视频帧提取：使用 FFmpeg 提取视频帧
- 🔍 场景检测：自动检测视频中的场景变化
- 📸 关键帧提取：为每个场景提取代表性关键帧
- 🎵 音频提取：从视频中提取音频文件
- 📊 元数据生成：生成包含场景信息的 JSON 元数据
- ☁️ OSS 集成：支持处理阿里云 OSS 事件

## 项目结构

```
code/
├── main.go              # 主入口文件
├── config.go            # 配置管理
├── oss_event.go         # OSS 事件数据结构
├── oss_client.go        # OSS 客户端
├── video_processor.go   # 视频处理
├── scene_detector.go    # 场景检测
├── audio_extractor.go   # 音频提取
├── metadata.go          # 元数据结构
├── processor.go         # 视频处理流程
├── handler.go           # HTTP 请求处理
├── go.mod               # Go 模块依赖
├── s.yaml               # Serverless Devs 配置文件
└── README.md            # 本文件
```

## 依赖要求

- Go 1.19+
- FFmpeg（需要在系统 PATH 中）
- 阿里云函数计算环境（用于 OSS 事件处理）

## 安装依赖

```bash
cd code
go mod tidy
```

## 本地开发

### 1. 编译

```bash
GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build -o target/main main.go
```

### 2. 运行

```bash
./target/main
```

## 部署到阿里云函数计算

### 1. 配置 s.yaml

编辑 `s.yaml` 文件，设置以下变量：
- `access`: 阿里云访问密钥别名
- `region`: 区域（如：cn-hangzhou）
- `functionName`: 函数名称
- `runtime`: 运行时（如：custom.debian10）

### 2. 部署

```bash
s deploy
```

## 环境变量配置

可以通过环境变量或配置文件设置以下参数：

- `VIDEO_PARSE_THRESHOLD`: 场景变化检测阈值（默认：0.35）
- `VIDEO_PARSE_MIN_SCENE_DURATION`: 最小场景持续时间，单位秒（默认：0.8）
- `VIDEO_PARSE_SAMPLE_RATE`: 帧采样率，每秒采样多少帧（默认：0.5）
- `VIDEO_PARSE_WEBHOOK_URL`: Webhook 回调 URL（可选）
- `DEBUG`: 调试模式（true/false）
- `OUTPUT_PATH`: 输出路径
- `DESTINATION_BUCKET`: 目标 OSS Bucket
- `DESTINATION_REGION`: 目标 OSS Region
- `DESTINATION_PREFIX`: 目标 OSS 路径前缀
- `LOG_LEVEL`: 日志级别（trace/debug/info/warn/error）

## API 端点

### 健康检查

```
GET /health
```

### 处理 OSS 事件

```
POST /process
Content-Type: application/json

{
  "events": [
    {
      "eventName": "ObjectCreated:Put",
      "oss": {
        "bucket": {
          "name": "your-bucket"
        },
        "object": {
          "key": "path/to/video.mp4"
        }
      },
      "region": "cn-hangzhou"
    }
  ]
}
```

### 直接处理视频

```
POST /process/direct
Content-Type: application/json

{
  "input": "/path/to/video.mp4",
  "output": "/path/to/output",
  "threshold": 0.35,
  "min_scene_duration": 0.8,
  "sample_rate": 0.5
}
```

## 输出文件

处理完成后，会在输出目录生成以下文件：

- `keyframe_0000.jpg`, `keyframe_0001.jpg`, ... - 关键帧图片
- `audio.aac` - 提取的音频文件
- `metadata.json` - 视频元数据

## 与 Rust 版本的差异

1. **图像处理**：Go 版本使用标准库的 `image` 包进行图像处理，场景检测算法进行了简化
2. **FFmpeg 调用**：使用 `exec.Command` 调用 FFmpeg 命令，而不是使用 FFmpeg 绑定库
3. **HTTP 服务器**：使用阿里云 FC SDK 处理 HTTP 请求，而不是独立的 HTTP 服务器
4. **配置管理**：使用 `go-ini/ini` 库解析配置文件

## 注意事项

1. 确保 FFmpeg 已安装并在系统 PATH 中
2. 函数计算环境需要足够的内存（建议 2048MB）和超时时间（建议 600 秒）
3. OSS 事件处理需要配置相应的环境变量（AccessKey ID/Secret）
4. 处理大视频文件时可能需要较长时间，建议使用异步处理

## 许可证

与原 Rust 版本保持一致。

