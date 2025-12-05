# OSS 事件触发处理完整流程指南

本文档说明如何配置和使用 OSS 事件触发功能，实现自动下载视频、处理拉片、并上传结果到目标 bucket。

## 功能概述

完整的处理流程：

1. **OSS 事件触发** → OSS bucket 中有新视频文件上传时触发
2. **下载视频** → 从源 bucket 下载视频文件到临时目录
3. **处理视频** → 执行拉片处理（提取关键帧、音频、元数据）
4. **上传结果** → 将处理结果上传到目标 bucket

## 配置步骤

### 1. 配置环境变量

在 `s.yaml` 中添加环境变量配置：

```yaml
resources:
  hello_world:
    component: fc3 
    props:
      environmentVariables:
        # DEBUG 模式：设置为 true 时，函数直接返回成功，跳过实际处理
        # 用于测试事件触发逻辑和部署是否正确
        DEBUG: "false"  # 生产环境设置为 false，测试时设置为 true
        
        # 目标 bucket 名称（必需）
        DESTINATION_BUCKET: "your-destination-bucket"
        # 目标 bucket 所在区域（必需）
        DESTINATION_REGION: "cn-hangzhou"
        # 目标路径前缀（可选，默认为源文件的目录路径）
        DESTINATION_PREFIX: "processed"
```

### 1.1 DEBUG 模式（测试用）

**用途**：快速验证函数部署和事件触发是否正确

**启用方式**：
```yaml
environmentVariables:
  DEBUG: "true"  # 启用 DEBUG 模式
```

**行为**：
- ✅ 接收并解析 OSS 事件
- ✅ 记录事件信息到日志
- ✅ **跳过**视频下载、处理、上传
- ✅ 直接返回成功响应

**详细说明**：请参考 [DEBUG_MODE_GUIDE.md](DEBUG_MODE_GUIDE.md)

### 2. 配置 OSS 触发器

在阿里云函数计算控制台或使用 s.yaml 配置 OSS 触发器：

```yaml
triggers:
  - triggerName: ossTrigger
    triggerType: oss
    description: 'OSS 事件触发器'
    qualifier: LATEST
    triggerConfig:
      bucketName: "your-source-bucket"  # 源 bucket
      events:
        - oss:ObjectCreated:Put
        - oss:ObjectCreated:Post
      filter:
        prefix: "videos/"  # 可选：只处理指定前缀的文件
        suffix: ".mp4"     # 可选：只处理指定后缀的文件
```

### 3. 配置函数权限

确保函数有权限访问源 bucket 和目标 bucket：

1. **源 bucket**：需要读取权限
2. **目标 bucket**：需要写入权限

在 s.yaml 中配置角色：

```yaml
props:
  role: "acs:ram::YOUR_ACCOUNT_ID:role/your-role-name"
```

角色需要包含以下权限策略：

```json
{
  "Version": "1",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "oss:GetObject",
        "oss:PutObject",
        "oss:HeadObject"
      ],
      "Resource": [
        "acs:oss:*:*:your-source-bucket/*",
        "acs:oss:*:*:your-destination-bucket/*"
      ]
    }
  ]
}
```

## 处理流程详解

### 1. 事件接收

函数通过 HTTP POST 接收 OSS 事件：

```json
{
  "events": [
    {
      "eventName": "ObjectCreated:Put",
      "oss": {
        "bucket": {
          "name": "source-bucket"
        },
        "object": {
          "key": "videos/example.mp4",
          "size": 1024000
        }
      },
      "region": "cn-hangzhou"
    }
  ]
}
```

### 2. 下载视频

- 从源 bucket 下载视频文件
- 使用 internal endpoint（内网访问，更快且免费）
- 保存到临时目录：`/tmp/video-parse/{request_id}/`

### 3. 处理视频

执行拉片处理，生成：

- **关键帧图片**：`keyframe_0000.jpg`, `keyframe_0001.jpg`, ...
- **音频文件**：`audio.aac`
- **元数据文件**：`metadata.json`

### 4. 上传结果

将处理结果上传到目标 bucket，文件结构：

```
目标 bucket/
└── {DESTINATION_PREFIX}/  (或源文件的目录路径)
    ├── keyframes/
    │   ├── keyframe_0000.jpg
    │   ├── keyframe_0001.jpg
    │   └── ...
    ├── audio.aac
    └── metadata.json
```

## 环境变量说明

### 测试环境变量

| 变量名 | 说明 | 默认值 | 示例 |
|--------|------|--------|------|
| `DEBUG` | DEBUG 模式：设置为 `true` 时跳过实际处理，仅用于测试 | `false` | `"true"` |

### 必需的环境变量

| 变量名 | 说明 | 示例 |
|--------|------|------|
| `DESTINATION_BUCKET` | 目标 bucket 名称 | `processed-videos` |
| `DESTINATION_REGION` | 目标 bucket 所在区域 | `cn-hangzhou` |

### 可选的环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `DESTINATION_PREFIX` | 目标路径前缀 | 源文件的目录路径 |

### 自动提供的环境变量（函数计算）

| 变量名 | 说明 |
|--------|------|
| `ALIBABA_CLOUD_ACCESS_KEY_ID` | Access Key ID |
| `ALIBABA_CLOUD_ACCESS_KEY_SECRET` | Access Key Secret |
| `ALIBABA_CLOUD_SECURITY_TOKEN` | Security Token（STS） |
| `FC_REQUEST_ID` | 请求 ID（用于日志追踪） |

## 文件路径规则

### 源文件路径示例

- 源文件：`videos/movie/example.mp4`
- 目标前缀：未设置（使用默认值）

### 上传后的文件结构

```
目标 bucket/
└── videos/movie/  (保持源文件的目录结构)
    ├── keyframes/
    │   ├── keyframe_0000.jpg
    │   ├── keyframe_0001.jpg
    │   └── ...
    ├── audio.aac
    └── metadata.json
```

### 自定义前缀示例

如果设置 `DESTINATION_PREFIX: "processed"`：

```
目标 bucket/
└── processed/
    ├── keyframes/
    │   ├── keyframe_0000.jpg
    │   └── ...
    ├── audio.aac
    └── metadata.json
```

## 错误处理

### 下载失败

- 记录错误日志
- 返回 HTTP 500 错误
- 不会继续处理

### 处理失败

- 记录错误日志
- 返回 HTTP 500 错误
- 不会上传结果

### 上传失败

- 记录错误日志
- **不会中断整个流程**
- 返回成功响应，但包含错误信息
- 已上传的文件会保留

## 日志和监控

### 日志输出

函数会输出详细的处理日志：

```
INFO: 收到 OSS Event: ...
INFO: 处理视频文件: bucket=source-bucket, key=videos/example.mp4, region=cn-hangzhou
INFO: 正在从 OSS 下载文件: bucket=source-bucket, key=videos/example.mp4, endpoint=oss-cn-hangzhou-internal.aliyuncs.com
INFO: 文件已下载到: /tmp/video-parse/xxx/video.mp4
INFO: 开始上传处理结果到目标 bucket: cn-hangzhou/processed-videos
INFO: 已上传关键帧: videos/movie/keyframes/keyframe_0000.jpg
INFO: 已上传音频文件: videos/movie/audio.aac
INFO: 已上传元数据文件: videos/movie/metadata.json
INFO: 上传完成: 成功 15 个，失败 0 个
INFO: 处理完成: ...
```

### 监控指标

可以通过函数计算的监控功能查看：

- 函数调用次数
- 成功/失败次数
- 执行时间
- 内存使用情况

## 性能优化建议

### 1. 使用 Internal Endpoint

- ✅ 已自动使用 internal endpoint
- 优势：更快、免费、更安全

### 2. 配置合适的超时时间

根据视频大小和处理时间配置：

```yaml
timeout: 600  # 10 分钟（大视频可能需要更长时间）
```

### 3. 配置足够的资源

```yaml
memorySize: 2048  # 2GB（处理大视频需要更多内存）
cpu: 2            # 2 核（加快处理速度）
diskSize: 10240   # 10GB（存储临时文件）
```

### 4. 使用 FFmpeg 层

已配置 FFmpeg 层，减少函数包大小：

```yaml
layers:
  - layerName: ffmpeg-layer
    code: ./layers/ffmpeg-layer.tar.gz
```

## 测试方法

### 1. DEBUG 模式测试（推荐首次部署）

**快速验证部署和事件触发**：

1. **启用 DEBUG 模式**：
   ```yaml
   environmentVariables:
     DEBUG: "true"
   ```

2. **部署函数**：
   ```bash
   s deploy
   ```

3. **触发测试**：
   - 在源 bucket 上传任意文件（不会实际处理）
   - 或发送 HTTP 请求

4. **查看结果**：
   ```bash
   s logs --tail
   ```
   - 应该看到 "DEBUG 模式已启用" 的日志
   - 响应应该包含事件信息

5. **关闭 DEBUG 模式**：
   ```yaml
   environmentVariables:
     DEBUG: "false"  # 或注释掉
   ```

详细说明请参考：[DEBUG_MODE_GUIDE.md](DEBUG_MODE_GUIDE.md)

### 2. 本地测试

```bash
# 启动服务
cargo run --release -- serve

# 发送测试事件
curl -X POST http://localhost:9000/process \
  -H "Content-Type: application/json" \
  -d @video-parse/examples/oss_event_example.json
```

### 3. 函数计算完整测试

1. **确保 DEBUG 模式关闭**：
   ```yaml
   environmentVariables:
     DEBUG: "false"
   ```

2. **部署函数**：
   ```bash
   s deploy
   ```

3. **在源 bucket 上传测试视频**

4. **查看函数计算日志**：
   ```bash
   s logs --tail
   ```

5. **检查目标 bucket**：
   - 应该看到处理结果文件
   - 关键帧、音频、元数据文件

## 故障排查

### 问题：函数未触发

- 检查 OSS 触发器配置
- 检查事件类型是否匹配
- 检查文件前缀/后缀过滤条件

### 问题：下载失败

- 检查源 bucket 权限
- 检查凭证是否正确
- 查看错误日志

### 问题：处理失败

- 检查视频格式是否支持
- 检查 FFmpeg 层是否正确配置
- 检查内存和磁盘空间是否足够

### 问题：上传失败

- 检查目标 bucket 权限
- 检查目标 bucket 是否存在
- 检查区域配置是否正确
- 查看错误日志

## 完整配置示例

```yaml
edition: 3.0.0
name: video-parse-app
access: "{{ access }}"

vars:
  region: "cn-hangzhou"
  sourceBucket: "source-videos"
  destinationBucket: "processed-videos"
  destinationRegion: "cn-hangzhou"

resources:
  video_parse_function:
    component: fc3 
    actions:    
      pre-${regex('deploy|local')}:
        - run: make video-parse
    props:
      region: ${vars.region} 
      functionName: "velocn-video-parse-function"
      runtime: "custom"
      description: '视频拉片处理函数'
      timeout: 600
      memorySize: 2048
      cpu: 2
      diskSize: 10240
      code: ./code/target
      layers:
        - layerName: ffmpeg-layer
          code: ./layers/ffmpeg-layer.tar.gz
      environmentVariables:
        DESTINATION_BUCKET: ${vars.destinationBucket}
        DESTINATION_REGION: ${vars.destinationRegion}
        DESTINATION_PREFIX: "processed"
      customRuntimeConfig:
        command:
          - '/code/main'
          - 'serve'
        port: 9000
      triggers:
        - triggerName: ossTrigger
          triggerType: oss
          description: 'OSS 事件触发器'
          qualifier: LATEST
          triggerConfig:
            bucketName: ${vars.sourceBucket}
            events:
              - oss:ObjectCreated:Put
            filter:
              prefix: "videos/"
              suffix: ".mp4"
```

## 参考文档

- [OSS 事件触发器文档](https://help.aliyun.com/document_detail/70140.html)
- [函数计算环境变量](https://help.aliyun.com/document_detail/69777.html)
- [OSS SDK 使用文档](OSS_SDK_IMPLEMENTATION.md)
