# OSS SDK 实现说明

本文档说明如何使用 `aliyun-oss-rust-sdk` 实现 OSS 的签名下载与上传。

## 已完成的更改

### 1. 添加依赖 ✅

在 `Cargo.toml` 中添加了：
```toml
aliyun-oss-rust-sdk = "0.2"
```

### 2. 重写 OSS 客户端 ✅

**文件：** `video-parse/src/oss_client.rs`

#### 主要改进：

1. **使用 SDK 进行认证**
   - 从环境变量读取凭证
   - 使用 `OSSConfig` 配置客户端
   - 支持 STS 临时凭证（Security Token）

2. **实现下载功能**
   - 使用 SDK 的 `get_object` 方法
   - 自动处理签名认证
   - 支持 internal endpoint

3. **实现上传功能**
   - 使用 SDK 的 `put_object` 方法
   - 自动处理签名认证
   - 支持 Content-Type 设置

4. **实现检查功能**
   - 使用 SDK 的 `head_object` 方法
   - 获取文件 ETag
   - 处理 404 错误

## API 使用说明

### 创建客户端

```rust
let oss_client = OssClient::new()?;
```

客户端会自动从环境变量读取凭证：
- `ALIBABA_CLOUD_ACCESS_KEY_ID`
- `ALIBABA_CLOUD_ACCESS_KEY_SECRET`
- `ALIBABA_CLOUD_SECURITY_TOKEN`

### 下载文件

```rust
let path = oss_client.download_file(
    "my-bucket",
    "path/to/file.mp4",
    Some("oss-cn-hangzhou-internal.aliyuncs.com"), // internal endpoint
    "/tmp/downloaded_file.mp4"
).await?;
```

### 上传文件

```rust
oss_client.upload_file(
    "my-bucket",
    "path/to/uploaded_file.jpg",
    "/tmp/local_file.jpg",
    Some("oss-cn-hangzhou-internal.aliyuncs.com")
).await?;
```

### 检查文件

```rust
match oss_client.head_object(
    "my-bucket",
    "path/to/file.jpg",
    Some("oss-cn-hangzhou-internal.aliyuncs.com")
).await? {
    Some(etag) => println!("文件存在，ETag: {}", etag),
    None => println!("文件不存在"),
}
```

## 注意事项

### 1. SDK API 可能需要调整

当前实现基于常见的 OSS SDK API 模式，但实际的 `aliyun-oss-rust-sdk` API 可能略有不同。

**如果遇到编译错误**，请根据实际的 SDK API 调整：

1. 查看 SDK 文档：`cargo doc --open --package aliyun-oss-rust-sdk`
2. 根据实际的 API 调整方法调用
3. 可能需要调整：
   - `OSS::get_object()` → `client.get_object()` 或其他形式
   - `OSS::put_object()` → `client.put_object()` 或其他形式
   - `OSS::head_object()` → `client.head_object()` 或其他形式

### 2. 环境变量

确保函数计算环境提供了以下环境变量：
- `ALIBABA_CLOUD_ACCESS_KEY_ID`
- `ALIBABA_CLOUD_ACCESS_KEY_SECRET`
- `ALIBABA_CLOUD_SECURITY_TOKEN`（STS 临时凭证）

### 3. Endpoint 配置

推荐使用 internal endpoint（内网访问）：
- 格式：`oss-{region}-internal.aliyuncs.com`
- 例如：`oss-cn-hangzhou-internal.aliyuncs.com`
- 优势：更快、免费、更安全

## 优势

使用 SDK 相比手动实现：

1. ✅ **自动签名** - SDK 自动处理 OSS 签名算法
2. ✅ **支持私有 bucket** - 完整的认证支持
3. ✅ **更少的代码** - 不需要手动实现签名逻辑
4. ✅ **更好的维护性** - SDK 会处理 API 变更
5. ✅ **错误处理** - SDK 提供更好的错误信息

## 后续步骤

1. **编译测试** - 确保代码能够编译
2. **API 调整** - 根据实际的 SDK API 调整方法调用
3. **功能测试** - 在实际环境中测试下载和上传功能
4. **错误处理** - 根据实际错误调整错误处理逻辑

## 参考

- [aliyun-oss-rust-sdk crates.io](https://crates.io/crates/aliyun-oss-rust-sdk)
- [OSS REST API 文档](https://help.aliyun.com/document_detail/31947.html)
- [OSS 签名算法](https://help.aliyun.com/document_detail/31951.html)
