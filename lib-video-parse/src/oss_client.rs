use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use ali_oss_rs::Client;
use ali_oss_rs::object::ObjectOperations;
use ali_oss_rs::object_common::PutObjectOptions;

/// OSS 客户端，用于下载和上传文件
/// 
/// 使用 ali-oss-rs SDK 实现 OSS 操作
/// 
/// 函数计算环境会提供以下环境变量用于认证：
/// - ALIBABA_CLOUD_ACCESS_KEY_ID
/// - ALIBABA_CLOUD_ACCESS_KEY_SECRET  
/// - ALIBABA_CLOUD_SECURITY_TOKEN
pub struct OssClient {
    /// Access Key ID
    access_key_id: String,
    /// Access Key Secret
    access_key_secret: String,
    /// Security Token（STS 临时凭证，可选）
    security_token: Option<String>,
}

impl OssClient {
    /// 创建新的 OSS 客户端
    /// 
    /// 从环境变量读取凭证：
    /// - ALIBABA_CLOUD_ACCESS_KEY_ID
    /// - ALIBABA_CLOUD_ACCESS_KEY_SECRET
    /// - ALIBABA_CLOUD_SECURITY_TOKEN
    pub fn new() -> Result<Self> {
        // 从环境变量获取凭证
        let access_key_id = std::env::var("ALIBABA_CLOUD_ACCESS_KEY_ID")
            .context("未找到 ALIBABA_CLOUD_ACCESS_KEY_ID 环境变量")?;
        let access_key_secret = std::env::var("ALIBABA_CLOUD_ACCESS_KEY_SECRET")
            .context("未找到 ALIBABA_CLOUD_ACCESS_KEY_SECRET 环境变量")?;
        let security_token = std::env::var("ALIBABA_CLOUD_SECURITY_TOKEN").ok();

        Ok(Self {
            access_key_id,
            access_key_secret,
            security_token,
        })
    }

    /// 从 endpoint 提取 region
    /// 
    /// 例如：oss-cn-hangzhou-internal.aliyuncs.com -> cn-hangzhou
    fn extract_region_from_endpoint(endpoint: &str) -> String {
        // 移除协议前缀（如果有）
        let endpoint = endpoint.trim_start_matches("http://").trim_start_matches("https://");
        
        // 提取 region（格式：oss-{region}-internal.aliyuncs.com 或 oss-{region}.aliyuncs.com）
        if let Some(start) = endpoint.strip_prefix("oss-") {
            if let Some(end) = start.find("-internal") {
                start[..end].to_string()
            } else if let Some(end) = start.find(".aliyuncs.com") {
                start[..end].to_string()
            } else {
                // 默认值
                "cn-hangzhou".to_string()
            }
        } else {
            // 默认值
            "cn-hangzhou".to_string()
        }
    }

    /// 创建 OSS Client 实例
    /// 
    /// # 参数
    /// - `endpoint`: OSS endpoint（例如：oss-cn-hangzhou-internal.aliyuncs.com）
    fn create_client(&self, endpoint: &str) -> Result<Client> {
        // 从 endpoint 提取 region
        let region = Self::extract_region_from_endpoint(endpoint);
        
        // 创建客户端
        // 注意：ali-oss-rs 的 Client::new 需要 access_key_id, access_key_secret, region, endpoint
        // 但目前不支持直接传递 STS token，可能需要使用 ClientBuilder
        // 让我们先尝试基本的方式，如果需要 STS token，可能需要查看 ClientBuilder
        
        let client = Client::new(
            &self.access_key_id,
            &self.access_key_secret,
            &region,
            endpoint,
        );
        
        // TODO: 如果需要支持 STS token，可能需要使用 ClientBuilder
        // 目前先使用基本方式
        
        Ok(client)
    }

    /// 从 OSS 下载文件
    /// 
    /// # 参数
    /// - `bucket`: OSS bucket 名称
    /// - `object_key`: OSS 对象键（文件路径）
    /// - `endpoint`: OSS endpoint（推荐使用 internal endpoint，格式: oss-{region}-internal.aliyuncs.com）
    /// - `output_path`: 本地保存路径
    pub async fn download_file(
        &self,
        bucket: &str,
        object_key: &str,
        endpoint: Option<&str>,
        output_path: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let output_path = output_path.as_ref();
        
        // 确保父目录存在
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .context("创建输出目录失败")?;
        }

        // 构建 endpoint（优先使用 internal endpoint）
        let ep: String = endpoint.map(|s| s.to_string()).unwrap_or_else(|| {
            std::env::var("OSS_ENDPOINT")
                .unwrap_or_else(|_| "oss-cn-hangzhou.aliyuncs.com".to_string())
        });

        tracing::info!("正在从 OSS 下载文件: bucket={}, key={}, endpoint={}", bucket, object_key, ep);

        // 创建 OSS 客户端
        let client = self.create_client(&ep)?;

        // 下载文件到本地路径
        client
            .get_object_to_file(bucket, object_key, output_path, None)
            .await
            .context("下载文件失败")?;

        tracing::info!("文件已下载到: {}", output_path.display());

        Ok(output_path.to_path_buf())
    }

    /// 上传文件到 OSS
    /// 
    /// # 参数
    /// - `bucket`: OSS bucket 名称
    /// - `object_key`: OSS 对象键（文件路径）
    /// - `file_path`: 本地文件路径
    /// - `endpoint`: OSS endpoint（推荐使用 internal endpoint）
    pub async fn upload_file(
        &self,
        bucket: &str,
        object_key: &str,
        file_path: impl AsRef<Path>,
        endpoint: Option<&str>,
    ) -> Result<()> {
        let file_path = file_path.as_ref();
        
        // 检查文件是否存在
        if !file_path.exists() {
            anyhow::bail!("文件不存在: {}", file_path.display());
        }

        // 构建 endpoint（优先使用 internal endpoint）
        let ep: String = endpoint.map(|s| s.to_string()).unwrap_or_else(|| {
            std::env::var("OSS_ENDPOINT")
                .unwrap_or_else(|_| "oss-cn-hangzhou.aliyuncs.com".to_string())
        });

        tracing::info!("正在上传文件到 OSS: {} -> bucket={}, key={}, endpoint={}", 
            file_path.display(), bucket, object_key, ep);

        // 获取 Content-Type
        let content_type = self.guess_content_type(object_key);

        // 创建 OSS 客户端
        let client = self.create_client(&ep)?;

        // 构建上传选项
        let mut options = PutObjectOptions::default();
        options.mime_type = Some(content_type.to_string());

        // 上传文件
        client
            .put_object_from_file(bucket, object_key, file_path, Some(options))
            .await
            .context("上传文件失败")?;

        tracing::info!("文件已上传到 OSS: bucket={}, key={}", bucket, object_key);

        Ok(())
    }

    /// 检查 OSS 对象是否存在并获取元数据
    /// 
    /// # 参数
    /// - `bucket`: OSS bucket 名称
    /// - `object_key`: OSS 对象键（文件路径）
    /// - `endpoint`: OSS endpoint（推荐使用 internal endpoint）
    /// 
    /// # 返回
    /// - `Ok(Some(etag))`: 文件存在，返回 ETag
    /// - `Ok(None)`: 文件不存在
    /// - `Err`: 请求失败
    pub async fn head_object(
        &self,
        bucket: &str,
        object_key: &str,
        endpoint: Option<&str>,
    ) -> Result<Option<String>> {
        // 构建 endpoint
        let ep: String = endpoint.map(|s| s.to_string()).unwrap_or_else(|| {
            std::env::var("OSS_ENDPOINT")
                .unwrap_or_else(|_| "oss-cn-hangzhou.aliyuncs.com".to_string())
        });

        tracing::debug!("检查 OSS 对象: bucket={}, key={}, endpoint={}", bucket, object_key, ep);

        // 创建 OSS 客户端
        let client = self.create_client(&ep)?;

        // 发送 HEAD 请求
        match client.head_object(bucket, object_key, None).await {
            Ok(metadata) => {
                // 文件存在，获取 ETag
                // ObjectMetadata 应该有 etag 字段
                let etag = metadata.etag.clone();
                Ok(Some(etag))
            }
            Err(e) => {
                // 检查是否是 404 错误
                let error_str = e.to_string();
                if error_str.contains("404") || error_str.contains("NoSuchKey") || error_str.contains("not found") {
                    Ok(None)
                } else {
                    Err(e).context("检查对象失败")
                }
            }
        }
    }

    /// 根据文件扩展名猜测 Content-Type
    fn guess_content_type(&self, object_key: &str) -> &'static str {
        let ext = std::path::Path::new(object_key)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "mp4" => "video/mp4",
            "mp3" => "audio/mpeg",
            "aac" => "audio/aac",
            "json" => "application/json",
            "txt" => "text/plain",
            "html" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            _ => "application/octet-stream",
        }
    }
}
