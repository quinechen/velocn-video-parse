# OSS Bucket 同步代码分析

本文档分析了 `reference/sync-oss-bucket` 中 JavaScript 代码的实现方式，用于指导 Rust 代码的实现。

## 核心实现分析

### 1. OSS 客户端初始化

**JavaScript 代码（sync-worker/index.js）：**
```javascript
destOssClient = new OSS({
  region: `oss-${destinationRegion}`,
  bucket: destinationBucket,
  accessKeyId: context.credentials.accessKeyId,
  accessKeySecret: context.credentials.accessKeySecret,
  stsToken: context.credentials.securityToken,
  internal: true,  // 关键：使用内网访问
});
```

**关键点：**
- ✅ 从 `context.credentials` 获取临时凭证（STS）
- ✅ 使用 `internal: true` 启用内网访问（更快且免费）
- ✅ Region 格式：`oss-{region}`（如 `oss-cn-hangzhou`）

### 2. 下载文件

**JavaScript 代码：**
```javascript
const sourceOssClient = new OSS({
  region: `oss-${sourceRegion}`,
  bucket: sourceBucket,
  accessKeyId: context.credentials.accessKeyId,
  accessKeySecret: context.credentials.accessKeySecret,
  stsToken: context.credentials.securityToken,
  internal: sourceRegion === context.region,  // 同区域使用内网
});

const tempFilePath = `/tmp/${name}`;
// 确保目录存在
fs.mkdirSync(tempDir, { recursive: true });
// 下载文件
await sourceOssClient.get(name, tempFilePath);
```

**关键点：**
- ✅ 使用 `get(objectKey, localPath)` 方法下载
- ✅ 自动处理目录创建
- ✅ 使用临时目录 `/tmp`

### 3. 上传文件

**JavaScript 代码：**
```javascript
await destOssClient.put(name, tempFilePath);
```

**关键点：**
- ✅ 使用 `put(objectKey, localPath)` 方法上传
- ✅ 自动处理文件读取和上传

### 4. 检查文件是否存在

**JavaScript 代码：**
```javascript
const result = await destOssClient.head(name, {});
const {
  res: {
    headers: { etag: currentEtag },
  },
} = result;
if (currentEtag === etag) {
  console.log(`Object ${name} not changed`);
  return;  // 跳过同步
}
```

**关键点：**
- ✅ 使用 `head()` 方法检查文件元数据
- ✅ 通过 ETag 比较判断文件是否变化
- ✅ 如果 ETag 相同，跳过同步

## Rust 实现要点

### 1. 凭证获取

在函数计算环境中，凭证通过环境变量提供：
- `ALIBABA_CLOUD_ACCESS_KEY_ID`
- `ALIBABA_CLOUD_ACCESS_KEY_SECRET`
- `ALIBABA_CLOUD_SECURITY_TOKEN`

### 2. OSS API 调用方式

**选项 1：使用 OSS Rust SDK（推荐）**
- 查找可用的 OSS Rust SDK（如 `aliyun-oss-rust-sdk`）
- 使用 SDK 提供的客户端和方法

**选项 2：手动实现 OSS API（当前方案）**
- 实现 OSS REST API 调用
- 需要实现签名算法（HMAC-SHA1）
- 需要处理各种 HTTP 请求和响应

### 3. 需要实现的功能

1. **下载文件** (`GET /{bucket}/{object}`)
   - 支持 STS 认证
   - 支持内网 endpoint
   - 流式下载大文件

2. **上传文件** (`PUT /{bucket}/{object}`)
   - 支持 STS 认证
   - 支持内网 endpoint
   - 流式上传大文件
   - 支持分片上传（大文件）

3. **检查文件** (`HEAD /{bucket}/{object}`)
   - 获取文件元数据
   - 获取 ETag

4. **列出文件** (`GET /{bucket}?list-type=2`)
   - 分页列出对象
   - 支持 continuation-token

## 参考文档

- [OSS REST API 文档](https://help.aliyun.com/document_detail/31947.html)
- [OSS 签名算法](https://help.aliyun.com/document_detail/31951.html)
- [OSS 内网访问](https://help.aliyun.com/document_detail/31837.html)

## 实现建议

### 短期方案（当前）
1. ✅ 已实现：从环境变量读取凭证
2. ✅ 已实现：使用 internal endpoint
3. ⚠️ 待实现：OSS 签名算法（支持私有 bucket）
4. ⚠️ 待实现：上传文件功能

### 长期方案（推荐）
1. 使用成熟的 OSS Rust SDK
2. 减少维护成本
3. 支持所有 OSS 功能
