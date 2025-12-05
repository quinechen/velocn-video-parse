# OSS 文件上传功能实现总结

本文档总结了基于 `reference/sync-oss-bucket` JavaScript 代码学习后，在 Rust 代码中实现的 OSS 文件上传功能。

## 已实现的功能

### 1. OSS 客户端增强 ✅

**文件：** `video-parse/src/oss_client.rs`

#### 新增方法：

1. **`upload_file()`** - 上传文件到 OSS
   ```rust
   pub async fn upload_file(
       &self,
       bucket: &str,
       object_key: &str,
       file_path: impl AsRef<Path>,
       endpoint: Option<&str>,
   ) -> Result<()>
   ```
   - 支持上传本地文件到 OSS
   - 自动检测 Content-Type
   - 支持 STS 临时凭证（通过 Security Token Header）

2. **`head_object()`** - 检查对象是否存在并获取元数据
   ```rust
   pub async fn head_object(
       &self,
       bucket: &str,
       object_key: &str,
       endpoint: Option<&str>,
   ) -> Result<Option<String>>  // 返回 ETag
   ```
   - 检查文件是否存在
   - 获取文件的 ETag（可用于判断文件是否变化）

3. **`guess_content_type()`** - 根据文件扩展名猜测 Content-Type
   - 支持常见文件类型的 Content-Type 检测

### 2. Handler 增强 ✅

**文件：** `video-parse/src/handler.rs`

#### 新增功能：

- ✅ 处理完成后自动上传结果文件到目标 bucket
- ✅ 支持通过环境变量配置目标 bucket：
  - `DESTINATION_BUCKET`: 目标 bucket 名称
  - `DESTINATION_REGION`: 目标 region
  - `DESTINATION_PREFIX`: 目标路径前缀（可选）
- ✅ 上传的文件包括：
  - 关键帧图片（`keyframes/` 目录）
  - 音频文件
  - 元数据文件（`metadata.json`）

## 使用方式

### 1. 配置环境变量

在 `s.yaml` 中配置目标 bucket：

```yaml
environmentVariables:
  DESTINATION_BUCKET: "your-destination-bucket"
  DESTINATION_REGION: "cn-hangzhou"
  DESTINATION_PREFIX: "processed"  # 可选，默认为源文件的目录路径
```

### 2. 工作流程

1. **接收 OSS 事件** → 从源 bucket 下载视频文件
2. **处理视频** → 提取关键帧、音频、元数据
3. **上传结果** → 将处理结果上传到目标 bucket

### 3. 上传的文件结构

假设源文件是 `videos/example.mp4`，上传后的结构：

```
目标 bucket/
└── videos/  (或 DESTINATION_PREFIX 指定的路径)
    ├── keyframes/
    │   ├── keyframe_0000.jpg
    │   ├── keyframe_0001.jpg
    │   └── ...
    ├── audio.aac
    └── metadata.json
```

## 与 JavaScript 实现的对比

| 功能 | JavaScript (sync-oss-bucket) | Rust (当前实现) | 状态 |
|------|------------------------------|-----------------|------|
| OSS 客户端初始化 | ✅ `new OSS({...})` | ✅ `OssClient::new()` | ✅ 一致 |
| 凭证获取 | ✅ `context.credentials` | ✅ 环境变量 | ✅ 一致 |
| Internal endpoint | ✅ `internal: true` | ✅ `oss-{region}-internal` | ✅ 一致 |
| 下载文件 | ✅ `client.get()` | ✅ `download_file()` | ✅ 一致 |
| 上传文件 | ✅ `client.put()` | ✅ `upload_file()` | ✅ 一致 |
| 检查文件 | ✅ `client.head()` | ✅ `head_object()` | ✅ 一致 |
| ETag 比较 | ✅ 支持 | ✅ 支持 | ✅ 一致 |

## 注意事项

### 1. 认证和签名 ⚠️

**当前状态：**
- ✅ 支持 STS 临时凭证（通过 Security Token Header）
- ⚠️ **待实现**：OSS 签名算法（HMAC-SHA1）

**影响：**
- 当前仅支持公共读/写的 bucket
- 私有 bucket 需要实现签名算法

**解决方案：**
- 选项 1：实现 OSS 签名算法（参考：https://help.aliyun.com/document_detail/31951.html）
- 选项 2：使用 OSS Rust SDK（如果存在）

### 2. 大文件处理

**当前实现：**
- 使用 `fs::read()` 一次性读取文件到内存
- 适合中小型文件（< 100MB）

**优化建议：**
- 对于大文件，实现流式上传
- 支持分片上传（Multipart Upload）

### 3. 错误处理

**当前实现：**
- 上传失败会记录错误日志
- 不会中断整个处理流程
- 返回成功响应（即使部分文件上传失败）

**改进建议：**
- 可以添加重试机制
- 可以返回详细的上传结果

## 环境变量说明

### 必需的环境变量（函数计算自动提供）

- `ALIBABA_CLOUD_ACCESS_KEY_ID` - Access Key ID
- `ALIBABA_CLOUD_ACCESS_KEY_SECRET` - Access Key Secret
- `ALIBABA_CLOUD_SECURITY_TOKEN` - Security Token（STS）

### 可选的环境变量（需要配置）

- `DESTINATION_BUCKET` - 目标 bucket 名称
- `DESTINATION_REGION` - 目标 region（如 `cn-hangzhou`）
- `DESTINATION_PREFIX` - 目标路径前缀（可选）
- `OSS_ENDPOINT` - OSS endpoint（如果未提供，会从事件中提取）

## 示例配置

### s.yaml 配置示例

```yaml
resources:
  hello_world:
    component: fc3 
    props:
      environmentVariables:
        DESTINATION_BUCKET: "processed-videos"
        DESTINATION_REGION: "cn-hangzhou"
        DESTINATION_PREFIX: "processed"
      # ... 其他配置 ...
```

## 后续优化建议

1. **实现 OSS 签名算法**
   - 支持私有 bucket
   - 完整的认证流程

2. **流式上传**
   - 支持大文件流式上传
   - 减少内存占用

3. **分片上传**
   - 支持 Multipart Upload
   - 提高大文件上传成功率

4. **上传重试**
   - 自动重试失败的上传
   - 提高可靠性

5. **进度追踪**
   - 记录上传进度
   - 支持断点续传

## 参考文档

- [OSS REST API 文档](https://help.aliyun.com/document_detail/31947.html)
- [OSS 签名算法](https://help.aliyun.com/document_detail/31951.html)
- [OSS 内网访问](https://help.aliyun.com/document_detail/31837.html)
- [OSS Multipart Upload](https://help.aliyun.com/document_detail/31991.html)
