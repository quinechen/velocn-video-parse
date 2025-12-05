# 视频拉片工具 (Video Parse)

一个用 Rust 编写的视频分析工具，用于自动检测视频中的场景变化（镜头切换），提取关键帧，并生成详细的元数据。

## 功能特性

- 🎬 **场景检测**：自动检测视频中的镜头切换点
- 🖼️ **关键帧提取**：提取每个场景的关键帧并保存为图片
- 📊 **元数据生成**：生成包含场景时间信息的 JSON 文件
- 🎵 **音频提取**：从视频中提取音频文件
- ⚡ **高性能**：使用 FFmpeg 进行高效的视频处理

## 输出内容

处理完成后，会在输出目录生成：

1. **关键帧图片** (`keyframe_0000.jpg`, `keyframe_0001.jpg`, ...)
   - 每个场景的代表帧
   - JPG 格式

2. **元数据 JSON** (`metadata.json`)
   - 包含每个场景的开始时间、结束时间、持续时间
   - 视频基本信息（分辨率、帧率、总时长等）

3. **音频文件** (`audio.aac`)
   - 从视频中提取的音频流

## 安装要求

### 系统依赖

需要安装 FFmpeg 及其开发库：

```bash
# Ubuntu/Debian
sudo apt-get install ffmpeg libavcodec-dev libavformat-dev libavutil-dev libavfilter-dev libavdevice-dev libswscale-dev libswresample-dev

# macOS
brew install ffmpeg

# Windows
# 从 https://ffmpeg.org/download.html 下载并添加到 PATH
# 或者使用 vcpkg: vcpkg install ffmpeg
```

### 编译项目

```bash
cargo build --release
```

## 使用方法

本工具支持两种运行模式：**CLI 模式**和**Web 服务模式**。

### CLI 模式

处理本地视频文件。

#### 基本用法

```bash
cargo run --release -- process --input video.mp4 --output ./output
```

#### 命令行参数

- `-i, --input <FILE>`: 输入视频文件路径（必需）
- `-o, --output <DIR>`: 输出目录（默认：`./output`）
- `--threshold <VALUE>`: 场景变化检测阈值，范围 0.0-1.0（默认：0.3）
  - 值越大，检测越敏感（更容易检测到场景变化）
  - 值越小，检测越保守（只检测明显的场景变化）
- `--min-scene-duration <SECONDS>`: 最小场景持续时间（秒）（默认：1.0）
- `--sample-rate <FPS>`: 用于分析的帧采样率，每秒采样多少帧（默认：2.0）
  - 较高的值会提高准确性但增加处理时间
  - 较低的值会加快处理但可能遗漏快速场景切换

#### CLI 示例

```bash
# 使用默认参数
cargo run --release -- process --input movie.mp4

# 自定义输出目录和检测阈值
cargo run --release -- process \
  --input movie.mp4 \
  --output ./my_output \
  --threshold 0.4 \
  --min-scene-duration 2.0

# 高精度模式（更慢但更准确）
cargo run --release -- process \
  --input movie.mp4 \
  --sample-rate 5.0 \
  --threshold 0.25
```

### Web 服务模式

启动 HTTP 服务器，接收阿里云函数计算的 OSS event，自动处理视频。

#### 启动服务器

```bash
# 使用默认地址 (0.0.0.0:8080)
cargo run --release -- serve

# 自定义监听地址
cargo run --release -- serve --bind 0.0.0.0:3000
```

#### API 端点

- `GET /` 或 `GET /health`: 健康检查
- `POST /process`: 处理 OSS event

#### OSS Event 格式

服务器接收的 JSON 请求格式应符合阿里云函数计算的 OSS event 格式：

```json
{
  "events": [
    {
      "eventName": "ObjectCreated:Put",
      "eventSource": "acs:oss",
      "eventTime": "2023-01-01T00:00:00.000Z",
      "eventVersion": "1.0",
      "oss": {
        "bucket": {
          "arn": "acs:oss:cn-hangzhou:123456789:bucket-name",
          "name": "bucket-name",
          "ownerIdentity": {
            "principalId": "123456789"
          },
          "virtualHostedBucketName": "bucket-name.oss-cn-hangzhou.aliyuncs.com"
        },
        "object": {
          "key": "videos/example.mp4",
          "size": 1024000,
          "eTag": "abc123",
          "deltaSize": 1024000
        },
        "ossSchemaVersion": "1.0",
        "ruleId": "rule-123"
      },
      "region": "cn-hangzhou",
      "requestParameters": {
        "sourceIPAddress": "192.168.1.1"
      },
      "responseElements": {
        "requestId": "req-123"
      },
      "userIdentity": {
        "principalId": "123456789"
      }
    }
  ]
}
```

#### 处理流程

1. 服务器接收 OSS event
2. 从 event 中提取 bucket 和 object key
3. 从 OSS 下载视频文件到临时目录
4. 处理视频（提取关键帧、检测场景、提取音频）
5. 返回处理结果

#### 响应格式

```json
{
  "success": true,
  "message": "成功处理视频，检测到 15 个场景",
  "result": {
    "video_file": "/tmp/video-parse/1234567890_uuid/video.mp4",
    "output_dir": "/tmp/video-parse/1234567890_uuid/output",
    "scene_count": 15,
    "keyframes": [
      "keyframe_0000.jpg",
      "keyframe_0001.jpg",
      ...
    ],
    "audio_file": "audio.aac",
    "metadata_file": "metadata.json"
  }
}
```

#### 环境变量配置

- `OSS_ENDPOINT`: OSS endpoint（默认：`oss-cn-hangzhou.aliyuncs.com`）

#### Web 服务示例

```bash
# 启动服务器
cargo run --release -- serve --bind 0.0.0.0:8080

# 在另一个终端测试
curl -X POST http://localhost:8080/process \
  -H "Content-Type: application/json" \
  -d @oss_event.json

# 健康检查
curl http://localhost:8080/health
```

## 输出格式

### metadata.json 示例

```json
{
  "input_video": "video.mp4",
  "total_duration": 120.5,
  "fps": 30.0,
  "resolution": "1920x1080",
  "scene_count": 15,
  "audio_file": "audio.aac",
  "scenes": [
    {
      "scene_id": 0,
      "keyframe_file": "keyframe_0000.jpg",
      "start_time": 0.0,
      "end_time": 5.2,
      "duration": 5.2
    },
    {
      "scene_id": 1,
      "keyframe_file": "keyframe_0001.jpg",
      "start_time": 5.2,
      "end_time": 12.8,
      "duration": 7.6
    }
  ]
}
```

## 工作原理

1. **视频解码**：使用 FFmpeg 解码视频并提取帧
2. **帧采样**：按指定采样率提取帧进行分析（减少计算量）
3. **场景检测**：
   - 计算相邻帧之间的差异（使用直方图差异和像素差异）
   - 当差异超过阈值时，判定为场景切换
   - 应用最小场景持续时间过滤，避免误检
4. **关键帧提取**：在每个场景的开始位置提取关键帧
5. **音频提取**：使用 FFmpeg 提取音频流
6. **元数据生成**：汇总所有信息生成 JSON 文件

## 性能优化建议

- **采样率**：对于长视频，可以降低采样率（如 1.0 fps）以提高速度
- **阈值调整**：根据视频类型调整阈值
  - 电影/电视剧：0.3-0.4
  - 快速剪辑视频：0.2-0.3
  - 静态场景较多的视频：0.4-0.5

## 项目结构

```
src/
├── main.rs              # 主程序入口（支持 CLI 和 Web 服务模式）
├── lib.rs               # 库入口
├── video_processor.rs   # 视频解码和帧提取
├── scene_detector.rs    # 场景变化检测算法
├── audio_extractor.rs   # 音频提取
├── metadata.rs          # 元数据结构定义
├── processor.rs         # 视频处理逻辑（可复用）
├── oss_event.rs         # OSS event 数据结构
├── oss_client.rs        # OSS 客户端（下载文件）
└── handler.rs           # HTTP handler（处理 OSS event）
```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！