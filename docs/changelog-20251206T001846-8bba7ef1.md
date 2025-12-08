# OSS 事件处理方式对比分析

本文档对比了阿里云官方 Python 示例和当前 Rust 代码的 OSS 事件处理方式。

## 对比结果

### ✅ 一致的部分

1. **事件结构解析**
   - ✅ 都正确解析了 `events` 数组
   - ✅ 都处理第一个事件 `events[0]`
   - ✅ 都正确提取了 `bucket.name` 和 `object.key`

2. **事件类型检查**
   - ✅ 都检查了 `ObjectCreated` 事件类型

### ❌ 不一致的部分（需要修复）

#### 1. **认证方式** ⚠️ **严重问题**

**Python 示例：**
```python
creds = context.credentials
auth = oss2.StsAuth(
    creds.access_key_id, 
    creds.access_key_secret, 
    creds.security_token
)
```

**Rust 代码：**
```rust
// 当前使用公共 URL 下载，没有认证
// 这会导致私有 bucket 无法访问
```

**问题：** Rust 代码缺少 STS 临时凭证认证，无法访问私有 bucket。

**解决方案：** 
- 函数计算会通过环境变量提供临时凭证：
  - `ALIBABA_CLOUD_ACCESS_KEY_ID`
  - `ALIBABA_CLOUD_ACCESS_KEY_SECRET`
  - `ALIBABA_CLOUD_SECURITY_TOKEN`
- 需要使用这些凭证进行 OSS 签名请求

#### 2. **Endpoint 构建** ⚠️ **重要问题**

**Python 示例：**
```python
endpoint = "oss-" + evt["region"] + "-internal.aliyuncs.com"
bucket = oss2.Bucket(auth, endpoint, bucket_name)
```

**Rust 代码：**
```rust
// 使用环境变量或默认值
let default_endpoint = std::env::var("OSS_ENDPOINT")
    .unwrap_or_else(|_| "oss-cn-hangzhou.aliyuncs.com".to_string());
```

**问题：** 
- 没有从事件中提取 `region` 字段
- 没有使用 internal endpoint（内网访问更快且免费）
- 默认值硬编码为 `cn-hangzhou`

**解决方案：**
- 从 `event_item.region` 提取区域
- 构建 internal endpoint：`format!("oss-{}-internal.aliyuncs.com", region)`

#### 3. **符号链接处理** ⚠️ **功能缺失**

**Python 示例：**
```python
if "ObjectCreated:PutSymlink" == evt["eventName"]:
    object_name = bucket.get_symlink(object_name).target_key
    if object_name == "":
        raise RuntimeError("{} is invalid symlink file".format(...))
```

**Rust 代码：**
- ❌ 没有处理符号链接事件

**解决方案：**
- 检查 `ObjectCreated:PutSymlink` 事件
- 解析符号链接获取实际对象 key

#### 4. **临时目录命名**

**Python 示例：**
```python
tmpWorkDir = "{}/{}".format(WORK_DIR, context.request_id)
```

**Rust 代码：**
```rust
let temp_dir = std::env::temp_dir().join("video-parse").join(
    format!("{}_{}", timestamp, uuid)
);
```

**问题：** 没有使用函数计算的 `request_id`，不利于调试和日志追踪。

**解决方案：**
- 从 HTTP 请求头或环境变量获取 `request_id`
- 使用 `request_id` 作为临时目录名的一部分

#### 5. **事件接收方式**

**Python 示例：**
```python
def handler(event, context):
    evt_lst = json.loads(event)  # event 是字符串
```

**Rust 代码：**
```rust
pub async fn handle_oss_event(
    Json(event): Json<OssEvent>,  // 直接解析 JSON body
)
```

**说明：** 
- Python 运行时：event 是字符串，需要手动解析
- Rust Custom Runtime：通过 HTTP 接收，Axum 自动解析 JSON body
- ✅ 两种方式都正确，只是运行时环境不同

## 修复状态

### ✅ 已修复

1. **从事件提取 region** ✅
   - ✅ 使用 `event_item.region` 构建 endpoint
   - ✅ 优先使用 internal endpoint：`oss-{region}-internal.aliyuncs.com`
   - 位置：`handler.rs` line 66-67

2. **使用 request_id** ✅
   - ✅ 从环境变量 `FC_REQUEST_ID` 获取（如果可用）
   - ✅ 用于临时目录命名
   - 位置：`handler.rs` line 42-50

3. **符号链接事件检测** ✅
   - ✅ 检查 `ObjectCreated:PutSymlink` 事件
   - ✅ 返回明确的错误消息（暂不支持）
   - 位置：`handler.rs` line 35-45

4. **认证基础结构** ✅
   - ✅ 从环境变量读取临时凭证
   - ✅ 添加 Security Token 到请求头
   - ✅ 添加签名算法的框架和 TODO
   - 位置：`oss_client.rs`

### ⚠️ 待修复（部分完成）

1. **OSS 签名算法** ⚠️
   - ✅ 已添加凭证读取和环境变量支持
   - ✅ 已添加 Security Token Header
   - ❌ **待实现**：OSS 签名算法（HMAC-SHA1）
   - 当前状态：仅支持公共读的 bucket
   - 位置：`oss_client.rs` `build_signed_url` 方法

2. **符号链接解析** ⚠️
   - ✅ 已检测符号链接事件
   - ❌ **待实现**：调用 OSS API 解析符号链接
   - 需要：OSS SDK 的 `get_symlink` 功能
   - 位置：`handler.rs` line 35-45

## 修复建议

### 优先级 1：完成 OSS 签名算法（重要）

**当前状态：** 框架已就绪，但签名算法未实现

**解决方案：**
1. 实现 OSS 签名算法（参考：https://help.aliyun.com/document_detail/31951.html）
2. 或使用阿里云 OSS Rust SDK（如 `aliyun-oss-rust-sdk`）

**影响：** 当前仅能访问公共读的 bucket，私有 bucket 会失败

### 优先级 2：实现符号链接解析（中等）

**当前状态：** 已检测但未解析

**解决方案：**
1. 使用 OSS SDK 的 `get_symlink` API
2. 或实现 OSS API 调用获取符号链接目标

**影响：** 符号链接事件会被拒绝，但通常视频文件不会使用符号链接

### 优先级 3：使用 OSS SDK（推荐）

**建议：** 使用成熟的 OSS Rust SDK 替代手动实现
- 自动处理认证和签名
- 支持所有 OSS 功能（包括符号链接）
- 减少维护成本

**可选 SDK：**
- `aliyun-oss-rust-sdk`（如果存在）
- 或使用 `reqwest` + 手动实现签名（当前方案）

## 参考文档

- [阿里云函数计算 OSS 触发器文档](https://help.aliyun.com/document_detail/70140.html)
- [OSS Python SDK 文档](https://help.aliyun.com/document_detail/32026.html)
- [OSS 签名算法](https://help.aliyun.com/document_detail/31951.html)
