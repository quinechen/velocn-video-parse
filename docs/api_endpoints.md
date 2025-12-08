# API 端点文档

## 概述

视频处理服务提供多种API端点，支持命令行模式和服务化模式，可以处理本地文件和OSS文件。

## 端点列表

### 1. 健康检查

**端点**: `GET /health` 或 `GET /`

**描述**: 检查服务是否正常运行

**请求**:
```bash
curl http://localhost:9000/health
```

**响应**:
```
OK
```

---

### 2. OSS事件处理（函数计算模式）

**端点**: `POST /process`

**描述**: 处理阿里云函数计算的OSS事件，自动下载、处理并上传结果

**请求体**:
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
          "name": "source-bucket",
          "arn": "acs:oss:cn-hangzhou:123456789:source-bucket"
        },
        "object": {
          "key": "videos/example.mp4",
          "size": 1024000,
          "eTag": "abc123"
        }
      },
      "region": "cn-hangzhou"
    }
  ]
}
```

**响应**:
```json
{
  "success": true,
  "message": "成功处理视频，检测到 12 个场景，已上传到目标 bucket",
  "result": {
    "video_file": "/tmp/video-parse/1234567890_uuid/video.mp4",
    "output_dir": "/tmp/video-parse/1234567890_uuid/output",
    "scene_count": 12,
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

**特点**:
- ✅ 自动从OSS下载视频
- ✅ 自动上传处理结果到目标bucket（如果配置了）
- ✅ 适用于函数计算环境

---

### 3. 直接处理（支持本地文件和OSS文件）

**端点**: `POST /process/direct`

**描述**: 直接处理视频文件，支持本地路径和OSS路径

**请求体**:
```json
{
  "input": "/path/to/video.mp4",
  "output": "/path/to/output",
  "threshold": 0.35,
  "min_scene_duration": 0.8,
  "sample_rate": 0.5,
  "is_oss_path": false,
  "oss_bucket": null,
  "oss_region": null
}
```

**参数说明**:

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `input` | string | 是 | 视频文件路径（本地路径或OSS路径） |
| `output` | string | 否 | 输出目录（默认使用临时目录） |
| `threshold` | number | 否 | 场景变化检测阈值（默认: 0.35） |
| `min_scene_duration` | number | 否 | 最小场景持续时间（秒，默认: 0.8） |
| `sample_rate` | number | 否 | 帧采样率（默认: 0.5） |
| `is_oss_path` | boolean | 否 | 是否为OSS路径（默认: false） |
| `oss_bucket` | string | 条件 | OSS bucket（is_oss_path为true时必需） |
| `oss_region` | string | 条件 | OSS region（is_oss_path为true时必需） |

**示例1: 处理本地文件**
```bash
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{
    "input": "/path/to/video.mp4",
    "output": "/path/to/output"
  }'
```

**示例2: 处理OSS文件**
```bash
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{
    "input": "videos/example.mp4",
    "is_oss_path": true,
    "oss_bucket": "source-bucket",
    "oss_region": "cn-hangzhou"
  }'
```

**响应**:
```json
{
  "success": true,
  "message": "成功处理视频，检测到 12 个场景",
  "result": {
    "video_file": "/path/to/video.mp4",
    "output_dir": "/path/to/output",
    "scene_count": 12,
    "keyframes": ["keyframe_0000.jpg", ...],
    "audio_file": "audio.aac",
    "metadata_file": "metadata.json"
  }
}
```

---

### 4. 查询参数处理（GET请求，方便测试）

**端点**: `GET /process/query`

**描述**: 通过查询参数处理视频，方便测试和调试

**请求参数**:

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `input` | string | 是 | 视频文件路径（本地路径） |
| `output` | string | 否 | 输出目录 |
| `threshold` | number | 否 | 场景变化检测阈值 |
| `min_scene_duration` | number | 否 | 最小场景持续时间（秒） |
| `sample_rate` | number | 否 | 帧采样率 |

**示例**:
```bash
curl "http://localhost:9000/process/query?input=/path/to/video.mp4&output=/path/to/output&threshold=0.35"
```

**响应**: 同 `/process/direct`

---

## 使用场景

### 场景1: 命令行模式（CLI）

```bash
# 直接使用命令行
./dist/main process \
  --input input.mp4 \
  --output output \
  --sample-rate 0.5 \
  --threshold 0.35 \
  --min-scene-duration 0.8
```

### 场景2: 本地服务模式

```bash
# 1. 启动服务
./dist/main serve --bind 0.0.0.0:9000

# 2. 通过API调用
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{
    "input": "input.mp4",
    "output": "output"
  }'
```

### 场景3: 函数计算模式

```bash
# 1. 部署到函数计算
# 2. 配置OSS触发器
# 3. 自动处理OSS事件
```

**OSS事件会自动触发 `/process` 端点**

### 场景4: 混合模式（服务+OSS）

```bash
# 启动服务
./dist/main serve

# 通过API处理OSS文件
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{
    "input": "videos/example.mp4",
    "is_oss_path": true,
    "oss_bucket": "source-bucket",
    "oss_region": "cn-hangzhou"
  }'
```

---

## 环境变量配置

### OSS相关

- `OSS_ENDPOINT`: OSS endpoint（默认: `oss-cn-hangzhou.aliyuncs.com`）
- `OSS_ACCESS_KEY_ID`: OSS Access Key ID
- `OSS_ACCESS_KEY_SECRET`: OSS Access Key Secret

### 函数计算相关

- `FC_REQUEST_ID`: 函数计算的请求ID（自动设置）
- `FC_SERVER_PORT`: 服务端口（默认: 9000）

### 目标bucket配置（可选）

- `DESTINATION_BUCKET`: 目标bucket名称
- `DESTINATION_REGION`: 目标region
- `DESTINATION_PREFIX`: 目标路径前缀（默认: 源文件的目录路径）

### 调试模式

- `DEBUG`: 设置为 `true` 时，跳过实际处理，直接返回成功（用于测试部署）

---

## 错误处理

所有端点都返回标准的HTTP状态码：

- `200 OK`: 处理成功
- `400 Bad Request`: 请求参数错误
- `404 Not Found`: 文件不存在
- `500 Internal Server Error`: 服务器内部错误

错误响应格式：
```json
{
  "success": false,
  "message": "错误描述",
  "result": null
}
```

---

## 性能优化建议

1. **本地文件**: 使用 `/process/direct` 或 `/process/query`，避免OSS下载开销
2. **OSS文件**: 使用 `/process/direct` 并设置 `is_oss_path: true`，自动下载
3. **批量处理**: 使用函数计算的OSS触发器，自动处理上传的文件
4. **参数优化**: 使用 `lib-video-parse/scripts/optimize_params.py` 找到最优参数组合

---

## 完整示例

### 示例1: 命令行处理本地文件

```bash
./dist/main process \
  --input input.mp4 \
  --output output
```

### 示例2: 服务模式处理本地文件

```bash
# 终端1: 启动服务
./dist/main serve

# 终端2: 调用API
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{
    "input": "input.mp4",
    "output": "output"
  }'
```

### 示例3: 服务模式处理OSS文件

```bash
# 启动服务
./dist/main serve

# 调用API处理OSS文件
curl -X POST http://localhost:9000/process/direct \
  -H "Content-Type: application/json" \
  -d '{
    "input": "videos/example.mp4",
    "is_oss_path": true,
    "oss_bucket": "my-bucket",
    "oss_region": "cn-hangzhou",
    "output": "/tmp/output"
  }'
```

### 示例4: 函数计算OSS事件处理

```yaml
# s.yaml 配置
triggers:
  - triggerName: ossTrigger
    triggerType: oss
    triggerConfig:
      bucketName: "source-bucket"
      events:
        - oss:ObjectCreated:Put
```

当文件上传到OSS时，函数计算会自动调用 `/process` 端点。

---

## 总结

现在系统支持三种使用方式：

1. **命令行模式**: 直接使用 `process` 子命令
2. **服务模式**: 启动HTTP服务器，通过API调用
3. **函数计算模式**: 部署到函数计算，自动处理OSS事件

所有模式都使用相同的核心处理逻辑，确保一致性和可维护性。




