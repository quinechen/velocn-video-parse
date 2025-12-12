# OSS 事件测试脚本使用说明

## 概述

`test-oss-event.sh` 是一个用于本地测试 OSS 事件处理的 Shell 脚本。它可以：

1. 构建 Docker 镜像
2. 运行 Docker 容器（支持 STS Token 注入）
3. 构造 OSS Event JSON
4. 发送请求到 `/invoke` 端点
5. 查看容器日志和响应结果

## 前置要求

- Docker 已安装并运行
- 已配置阿里云 OSS 凭证（Access Key ID 和 Secret）
- 可选：STS Token（用于临时凭证）
- `curl` 和 `jq` 工具（用于发送请求和格式化 JSON）

## 环境变量配置

在运行脚本前，需要设置以下环境变量：

```bash
# 必需的环境变量
export ALIBABA_CLOUD_ACCESS_KEY_ID="your-access-key-id"
export ALIBABA_CLOUD_ACCESS_KEY_SECRET="your-access-key-secret"

# 可选的环境变量（如果使用 STS Token）
export ALIBABA_CLOUD_SECURITY_TOKEN="your-security-token"

# 可选：自定义 Docker 镜像名称和端口
export DOCKER_IMAGE_NAME="video-parse:latest"
export DOCKER_PORT="9000:9000"
export FC_REQUEST_ID="custom-request-id"  # 默认会自动生成
```

## 使用方法

### 1. 构建 Docker 镜像

```bash
./test-oss-event.sh --build
```

### 2. 运行容器（后台运行）

```bash
./test-oss-event.sh --run
```

容器会在后台运行，并自动注入 STS Token 等环境变量。

### 3. 发送 OSS 事件测试请求

```bash
./test-oss-event.sh \
  --bucket your-bucket-name \
  --key path/to/video.mp4 \
  --region cn-hangzhou
```

### 4. 完整流程（构建 + 运行 + 测试）

```bash
# 设置环境变量
export ALIBABA_CLOUD_ACCESS_KEY_ID="your-key-id"
export ALIBABA_CLOUD_ACCESS_KEY_SECRET="your-secret"
export ALIBABA_CLOUD_SECURITY_TOKEN="your-token"  # 可选

# 构建镜像
./test-oss-event.sh --build

# 运行容器并发送测试请求
./test-oss-event.sh \
  --run \
  --bucket your-bucket-name \
  --key videos/test.mp4 \
  --region cn-hangzhou
```

### 5. 停止容器

```bash
./test-oss-event.sh --stop
```

### 6. 清理资源

```bash
./test-oss-event.sh --cleanup
```

这会停止并删除容器，以及删除 Docker 镜像。

## 命令行参数

| 参数 | 说明 | 必需 |
|------|------|------|
| `-b, --bucket BUCKET` | OSS Bucket 名称 | 发送请求时必需 |
| `-k, --key KEY` | OSS Object Key（文件路径） | 发送请求时必需 |
| `-r, --region REGION` | OSS Region（如：cn-hangzhou） | 发送请求时必需 |
| `-e, --event EVENT_NAME` | 事件名称（默认：ObjectCreated:Put） | 否 |
| `-i, --image IMAGE_NAME` | Docker 镜像名称 | 否 |
| `-p, --port PORT` | 端口映射（默认：9000:9000） | 否 |
| `--build` | 构建 Docker 镜像 | 否 |
| `--run` | 运行 Docker 容器（后台） | 否 |
| `--stop` | 停止并删除容器 | 否 |
| `--cleanup` | 清理容器和镜像 | 否 |
| `-h, --help` | 显示帮助信息 | 否 |

## 示例场景

### 场景 1：测试本地视频处理

```bash
# 1. 设置凭证
export ALIBABA_CLOUD_ACCESS_KEY_ID="LTAI5t..."
export ALIBABA_CLOUD_ACCESS_KEY_SECRET="xxx..."

# 2. 构建并运行
./test-oss-event.sh --build
./test-oss-event.sh --run

# 3. 发送测试事件
./test-oss-event.sh \
  --bucket my-video-bucket \
  --key uploads/sample.mp4 \
  --region cn-hangzhou \
  --event ObjectCreated:Put
```

### 场景 2：使用 STS Token

```bash
# 设置 STS Token（从阿里云控制台或 API 获取）
export ALIBABA_CLOUD_SECURITY_TOKEN="CAIS..."

# 运行容器（会自动注入 STS Token）
./test-oss-event.sh --run

# 发送请求
./test-oss-event.sh \
  --bucket my-bucket \
  --key videos/test.mp4 \
  --region cn-hangzhou
```

### 场景 3：查看容器日志

```bash
# 容器运行后，可以实时查看日志
docker logs -f video-parse-test-*

# 或者查看最后100行
docker logs --tail 100 video-parse-test-*
```

## 生成的 OSS Event JSON 格式

脚本会生成符合阿里云 OSS 事件格式的 JSON，例如：

```json
{
  "events": [
    {
      "eventName": "ObjectCreated:Put",
      "eventSource": "acs:oss",
      "eventTime": "2024-01-01T12:00:00.000Z",
      "eventVersion": "1.0",
      "oss": {
        "bucket": {
          "arn": "acs:oss:cn-hangzhou:*:my-bucket",
          "name": "my-bucket",
          "ownerIdentity": {
            "principalId": "test-user-id"
          },
          "virtualHostedBucketName": "my-bucket.oss-cn-hangzhou.aliyuncs.com"
        },
        "object": {
          "deltaSize": null,
          "eTag": "d41d8cd98f00b204e9800998ecf8427e",
          "key": "videos/test.mp4",
          "size": 0
        },
        "ossSchemaVersion": "1.0",
        "ruleId": "test-rule-id"
      },
      "region": "cn-hangzhou",
      "requestParameters": {
        "sourceIPAddress": "127.0.0.1"
      },
      "responseElements": {
        "requestId": "test-request-id"
      },
      "userIdentity": {
        "principalId": "test-user-id"
      }
    }
  ]
}
```

## 注意事项

1. **文件大小**：脚本默认使用 `FILE_SIZE` 环境变量作为文件大小，如果未设置则使用 0。你可以通过环境变量设置：
   ```bash
   export FILE_SIZE=10485760  # 10MB
   ```

2. **容器名称**：容器名称会自动生成（格式：`video-parse-test-<timestamp>`），避免冲突。

3. **端口冲突**：如果端口 9000 已被占用，可以通过 `-p` 参数修改：
   ```bash
   ./test-oss-event.sh -p 9001:9000 --run
   ```

4. **STS Token 过期**：如果使用 STS Token，注意 Token 的有效期。过期后需要重新获取并设置环境变量。

5. **网络访问**：确保 Docker 容器能够访问阿里云 OSS 服务（需要网络连接）。

## 故障排查

### 问题 1：容器启动失败

```bash
# 查看容器日志
docker logs video-parse-test-*

# 检查镜像是否存在
docker images | grep video-parse
```

### 问题 2：请求失败（401/403）

- 检查 Access Key ID 和 Secret 是否正确
- 检查 STS Token 是否过期
- 检查 OSS Bucket 权限设置

### 问题 3：无法下载文件

- 检查 Object Key 路径是否正确
- 检查文件是否存在于 OSS
- 检查网络连接和 OSS endpoint 配置

### 问题 4：JSON 解析错误

- 检查生成的 JSON 格式是否正确
- 使用 `jq` 工具验证 JSON：
  ```bash
  echo "$event_json" | jq .
  ```

## 相关文件

- `test-oss-event.sh` - 测试脚本
- `Dockerfile` - Docker 镜像构建文件
- `lib-video-parse/src/handler.rs` - 事件处理逻辑
- `lib-video-parse/src/oss_event.rs` - OSS 事件数据结构
