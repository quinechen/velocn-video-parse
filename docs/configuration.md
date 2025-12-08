# 配置系统文档

## 概述

视频处理工具支持通过多种方式配置参数，配置优先级从高到低为：

1. **命令行参数**（最高优先级）
2. **环境变量**
3. **INI配置文件**
4. **默认值**（最低优先级）

## 配置参数

| 参数 | 环境变量 | 配置文件键 | 默认值 | 说明 |
|------|---------|-----------|--------|------|
| `threshold` | `VIDEO_PARSE_THRESHOLD` | `threshold` | `0.35` | 场景变化检测阈值 (0.0-1.0) |
| `min_scene_duration` | `VIDEO_PARSE_MIN_SCENE_DURATION` | `min_scene_duration` | `0.8` | 最小场景持续时间（秒） |
| `sample_rate` | `VIDEO_PARSE_SAMPLE_RATE` | `sample_rate` | `0.5` | 帧采样率（每秒采样多少帧） |
| `webhook_url` | `VIDEO_PARSE_WEBHOOK_URL` | `webhook_url` | `None` | Webhook 回调 URL（可选） |

## 配置方式

### 1. 命令行参数

```bash
./dist/main process \
  --input input.mp4 \
  --output output \
  --threshold 0.4 \
  --min-scene-duration 1.0 \
  --sample-rate 0.6 \
  --config /path/to/config.ini  # 可选：指定配置文件
```

### 2. 环境变量

```bash
export VIDEO_PARSE_THRESHOLD=0.4
export VIDEO_PARSE_MIN_SCENE_DURATION=1.0
export VIDEO_PARSE_SAMPLE_RATE=0.6
export VIDEO_PARSE_WEBHOOK_URL=https://your-api.com/webhook/video-processed

./dist/main process --input input.mp4 --output output
```

### 3. INI配置文件

创建配置文件 `video-parse.ini`：

```ini
[video_parse]
threshold = 0.4
min_scene_duration = 1.0
sample_rate = 0.6
webhook_url = https://your-api.com/webhook/video-processed
```

配置文件搜索顺序：

1. 命令行指定的配置文件路径（`--config`）
2. 当前目录的 `video-parse.ini`
3. 当前目录的 `.video-parse.ini`
4. 用户主目录的 `~/.video-parse.ini`
5. `/etc/video-parse.ini`（Linux/macOS）

### 4. 默认值

如果以上方式都未设置，将使用默认值：
- `threshold = 0.35`
- `min_scene_duration = 0.8`
- `sample_rate = 0.5`
- `webhook_url = None`（不启用 webhook）

## 配置文件示例

### 基本配置

```ini
[video_parse]
# 场景变化检测阈值 (0.0-1.0)
# 值越大，场景变化检测越敏感
threshold = 0.35

# 最小场景持续时间（秒）
# 小于此持续时间的场景将被忽略
min_scene_duration = 0.8

# 帧采样率（每秒采样多少帧用于分析）
# 值越大，处理越精确但速度越慢
sample_rate = 0.5

# Webhook 回调 URL（可选）
# 处理完成后会向此 URL 发送 POST 请求，包含处理结果信息
# 格式: JSON POST 请求
# 示例: https://your-api.com/webhook/video-processed
# 默认值: 空（不启用）
webhook_url =
```

### 高性能配置（快速处理）

```ini
[video_parse]
threshold = 0.3
min_scene_duration = 1.0
sample_rate = 0.3
```

### 高质量配置（精确检测）

```ini
[video_parse]
threshold = 0.4
min_scene_duration = 0.5
sample_rate = 1.0
```

## 使用示例

### 示例1: 使用配置文件

```bash
# 1. 创建配置文件
cat > video-parse.ini << EOF
[video_parse]
threshold = 0.4
min_scene_duration = 1.0
sample_rate = 0.6
EOF

# 2. 运行处理（自动读取配置文件）
./dist/main process --input input.mp4 --output output
```

### 示例2: 使用环境变量

```bash
# 设置环境变量
export VIDEO_PARSE_THRESHOLD=0.4
export VIDEO_PARSE_SAMPLE_RATE=0.6

# 运行处理
./dist/main process --input input.mp4 --output output
```

### 示例3: 命令行参数覆盖配置

```bash
# 即使配置文件或环境变量设置了值，命令行参数也会优先
./dist/main process \
  --input input.mp4 \
  --output output \
  --threshold 0.5  # 覆盖配置文件或环境变量中的值
```

### 示例4: 服务模式使用配置

```bash
# 设置环境变量
export VIDEO_PARSE_THRESHOLD=0.4
export VIDEO_PARSE_SAMPLE_RATE=0.6

# 启动服务（服务会自动读取环境变量）
./dist/main serve

# API调用时会使用环境变量中的配置
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{"input": "input.mp4"}'
```

## 配置验证

运行时会显示使用的配置：

```bash
$ ./dist/main process --input input.mp4 --output output
使用配置: threshold=0.40, min_scene_duration=1.00s, sample_rate=0.60 fps
开始处理视频: input.mp4
...
```

## 最佳实践

1. **开发环境**: 使用配置文件 `video-parse.ini`，便于版本控制
2. **生产环境**: 使用环境变量，便于容器化部署
3. **临时测试**: 使用命令行参数，快速调整参数
4. **函数计算**: 在函数配置中设置环境变量

## 故障排除

### 配置文件未找到

如果配置文件不存在，程序会使用默认值，不会报错。

### 配置文件格式错误

如果配置文件格式错误，程序会跳过配置文件，使用环境变量或默认值。

### 环境变量格式错误

如果环境变量值无法解析为数字，程序会忽略该环境变量，使用配置文件或默认值。

## Webhook 回调配置

### 功能说明

当视频处理完成后，如果配置了 `webhook_url`，程序会自动向该 URL 发送 POST 请求，包含处理结果的详细信息。

### 配置方式

#### 1. 配置文件

```ini
[video_parse]
webhook_url = https://your-api.com/webhook/video-processed
```

#### 2. 环境变量

```bash
export VIDEO_PARSE_WEBHOOK_URL=https://your-api.com/webhook/video-processed
```

### Webhook 请求格式

**请求方法**: `POST`  
**Content-Type**: `application/json`  
**超时时间**: 30 秒

**请求体示例**:

```json
{
  "status": "success",
  "input_video": "/path/to/input.mp4",
  "output_dir": "/path/to/output",
  "scene_count": 12,
  "keyframe_count": 12,
  "audio_file": "audio.aac",
  "metadata": {
    "input_video": "/path/to/input.mp4",
    "total_duration": 120.5,
    "fps": 30.0,
    "resolution": "1920x1080",
    "scene_count": 12,
    "audio_file": "audio.aac",
    "scenes": [
      {
        "scene_id": 0,
        "keyframe_file": "keyframe_0000.jpg",
        "start_time": 0.0,
        "end_time": 10.5,
        "duration": 10.5
      }
    ]
  },
  "timestamp": "2024-12-07T15:30:00Z"
}
```

### 使用示例

#### 示例1: 在配置文件中设置

```ini
[video_parse]
threshold = 0.35
webhook_url = https://api.example.com/webhook/video-processed
```

#### 示例2: 使用环境变量

```bash
export VIDEO_PARSE_WEBHOOK_URL=https://api.example.com/webhook/video-processed
make demo
```

#### 示例3: 函数计算环境

在 `s.yaml` 中配置：

```yaml
environmentVariables:
  VIDEO_PARSE_WEBHOOK_URL: "https://api.example.com/webhook/video-processed"
```

### 注意事项

1. **Webhook 失败不影响处理结果**: 如果 webhook 调用失败，程序会记录警告日志，但不会影响视频处理的结果
2. **超时处理**: Webhook 请求超时时间为 30 秒，超时后会记录警告
3. **异步调用**: Webhook 调用是异步的，不会阻塞主处理流程
4. **安全性**: 建议使用 HTTPS URL，并在 webhook 服务端验证请求来源




