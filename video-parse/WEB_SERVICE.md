# Web 服务模式使用指南

## 概述

Web 服务模式允许程序作为 HTTP 服务器运行，接收阿里云函数计算的 OSS event，自动处理视频文件。

## 快速开始

### 1. 启动服务器

```bash
# 使用默认端口 8080
cargo run --release -- serve

# 自定义端口
cargo run --release -- serve --bind 0.0.0.0:3000
```

### 2. 测试健康检查

```bash
curl http://localhost:8080/health
# 应该返回: OK
```

### 3. 发送 OSS Event

```bash
curl -X POST http://localhost:8080/process \
  -H "Content-Type: application/json" \
  -d @examples/oss_event_example.json
```

## 部署到阿里云函数计算

### 1. 编译为可执行文件

```bash
# Linux (函数计算运行环境)
cargo build --release --target x86_64-unknown-linux-gnu

# 如果需要交叉编译，安装目标平台工具链
rustup target add x86_64-unknown-linux-gnu
```

### 2. 创建部署包

```bash
# 创建部署目录
mkdir -p deploy
cp target/x86_64-unknown-linux-gnu/release/video-parse deploy/bootstrap
chmod +x deploy/bootstrap

# 打包
cd deploy
zip -r ../function.zip bootstrap
cd ..
```

### 3. 配置函数计算

1. 在阿里云函数计算控制台创建函数
2. 运行时选择：Custom Runtime
3. 上传 `function.zip`
4. 配置环境变量（如需要）：
   - `OSS_ENDPOINT`: OSS endpoint

### 4. 配置 OSS 触发器

1. 在 OSS bucket 中配置事件通知
2. 事件类型选择：`ObjectCreated:Put`
3. 目标函数选择：你创建的函数
4. 触发前缀/后缀（可选）：例如只处理 `.mp4` 文件

## API 文档

### POST /process

处理 OSS event，自动下载并处理视频。

**请求体**：OSS Event JSON（见 `examples/oss_event_example.json`）

**响应**：

成功响应（200）：
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

错误响应（400/500）：
```json
{
  "success": false,
  "message": "错误信息",
  "result": null
}
```

### GET /health

健康检查端点。

**响应**：`OK` (纯文本)

## 处理流程

1. **接收 Event**：服务器接收 POST 请求，解析 OSS event JSON
2. **验证 Event**：检查事件类型是否为 `ObjectCreated:*`
3. **下载视频**：从 OSS 下载视频文件到临时目录
4. **处理视频**：
   - 提取视频帧
   - 检测场景变化
   - 提取关键帧
   - 提取音频
   - 生成元数据
5. **返回结果**：返回处理结果 JSON

## 注意事项

### OSS 访问权限

- 如果 OSS bucket 是私有的，需要配置函数计算的角色权限
- 或者使用预签名 URL（需要修改代码）
- 或者将 bucket 设置为公共读（不推荐生产环境）

### 临时文件清理

- 处理后的文件保存在临时目录（`/tmp/video-parse/`）
- 函数计算环境会在函数执行完成后清理临时目录
- 如需持久化结果，需要上传到 OSS 或其他存储服务

### 超时设置

- 函数计算默认超时时间可能较短
- 对于大视频文件，需要增加函数超时时间
- 建议在函数配置中设置足够的超时时间（如 10 分钟）

### 内存限制

- 视频处理需要较多内存
- 确保函数计算配置足够的内存（建议至少 2GB）

## 扩展功能

### 上传结果到 OSS

可以在处理完成后，将结果文件上传回 OSS：

```rust
// 在 handler.rs 中添加
// 上传关键帧、音频、元数据到 OSS
```

### 支持其他存储服务

可以扩展 `oss_client.rs` 支持：
- 阿里云 OSS（已实现）
- AWS S3
- 腾讯云 COS
- 其他对象存储服务

### 异步处理

对于大视频文件，可以实现异步处理：
1. 接收 event 后立即返回
2. 在后台处理视频
3. 处理完成后通过回调或消息队列通知

## 故障排查

### 下载失败

- 检查 OSS bucket 权限
- 检查网络连接
- 检查 `OSS_ENDPOINT` 环境变量

### 处理失败

- 检查 FFmpeg 是否正确安装
- 检查视频文件格式是否支持
- 检查内存是否足够

### 超时

- 增加函数超时时间
- 降低采样率以减少处理时间
- 考虑使用异步处理