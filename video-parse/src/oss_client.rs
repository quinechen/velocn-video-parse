use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

type HmacSha1 = Hmac<Sha1>;

/// OSS 客户端，用于下载和上传文件
/// 
/// 手动实现 OSS API，避免 SDK 版本兼容性问题
/// 
/// 函数计算环境会提供以下环境变量用于认证：
/// - ALIBABA_CLOUD_ACCESS_KEY_ID
/// - ALIBABA_CLOUD_ACCESS_KEY_SECRET  
/// - ALIBABA_CLOUD_SECURITY_TOKEN
pub struct OssClient {
    client: Client,
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
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .context("创建 HTTP 客户端失败")?,
            access_key_id,
            access_key_secret,
            security_token,
        })
    }

    /// 构建 OSS URL
    fn build_url(&self, bucket: &str, object_key: &str, endpoint: &str) -> String {
        // URL 编码 object_key（只编码特殊字符，保留路径分隔符）
        let encoded_key = utf8_percent_encode(object_key, NON_ALPHANUMERIC).to_string();
        
        if endpoint.starts_with("http") {
            format!("{}/{}/{}", endpoint.trim_end_matches('/'), bucket, encoded_key)
        } else {
            format!("https://{}.{}/{}", bucket, endpoint, encoded_key)
        }
    }

    /// 生成 OSS 签名
    /// 
    /// 参考: https://help.aliyun.com/document_detail/31951.html
    fn sign_request(
        &self,
        method: &str,
        bucket: &str,
        object_key: &str,
        headers: &HeaderMap,
    ) -> Result<String> {
        // 1. 构建 CanonicalizedResource: /{bucket}/{object_key}
        let canonicalized_resource = format!("/{}/{}", bucket, object_key);

        // 2. 构建 CanonicalizedOSSHeaders
        let mut oss_headers = Vec::new();
        for (name, value) in headers.iter() {
            let name_lower = name.as_str().to_lowercase();
            if name_lower.starts_with("x-oss-") {
                oss_headers.push((name_lower, value.to_str().unwrap_or("")));
            }
        }
        oss_headers.sort_by(|a, b| a.0.cmp(&b.0));
        
        let canonicalized_oss_headers: String = oss_headers
            .iter()
            .map(|(k, v)| format!("{}:{}\n", k, v))
            .collect();

        // 3. 构建 StringToSign
        let date = headers
            .get("Date")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
            });

        // Content-MD5 (通常为空)
        let content_md5 = headers
            .get("Content-MD5")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Content-Type
        let content_type = headers
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let string_to_sign = format!(
            "{}\n{}\n{}\n{}\n{}{}",
            method,
            content_md5,
            content_type,
            &date,
            canonicalized_oss_headers,
            canonicalized_resource
        );

        // 4. 使用 HMAC-SHA1 签名
        let mut mac = HmacSha1::new_from_slice(self.access_key_secret.as_bytes())
            .context("创建 HMAC 失败")?;
        mac.update(string_to_sign.as_bytes());
        let signature = mac.finalize();
        let signature_bytes = signature.into_bytes();

        // 5. Base64 编码
        let signature_base64 = base64_engine.encode(&signature_bytes);

        Ok(signature_base64)
    }

    /// 构建带签名的请求头
    fn build_signed_headers(
        &self,
        method: &str,
        bucket: &str,
        object_key: &str,
        content_type: Option<&str>,
    ) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // Date header
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        headers.insert(
            "Date",
            HeaderValue::from_str(&date).context("创建 Date header 失败")?,
        );

        // Content-Type header (如果提供)
        if let Some(ct) = content_type {
            headers.insert(
                "Content-Type",
                HeaderValue::from_str(ct).context("创建 Content-Type header 失败")?,
            );
        }

        // Security Token header (STS)
        if let Some(ref token) = self.security_token {
            headers.insert(
                "x-oss-security-token",
                HeaderValue::from_str(token).context("创建 Security-Token header 失败")?,
            );
        }

        // Authorization header
        let signature = self.sign_request(method, bucket, object_key, &headers)?;
        let authorization = format!("OSS {}:{}", self.access_key_id, signature);
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&authorization).context("创建 Authorization header 失败")?,
        );

        Ok(headers)
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
        let ep = endpoint.unwrap_or_else(|| {
            std::env::var("OSS_ENDPOINT")
                .ok()
                .as_deref()
                .unwrap_or("oss-cn-hangzhou.aliyuncs.com")
        });

        tracing::info!("正在从 OSS 下载文件: bucket={}, key={}, endpoint={}", bucket, object_key, ep);

        // 构建 URL
        let url = self.build_url(bucket, object_key, ep);

        // 构建签名请求头
        let headers = self.build_signed_headers("GET", bucket, object_key, None)?;

        // 下载文件
        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("下载文件失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("下载文件失败: HTTP {} - {}", status, error_text);
        }

        // 保存文件
        let bytes = response.bytes().await.context("读取响应数据失败")?;
        fs::write(output_path, &bytes)
            .context("保存文件失败")?;

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
        let ep = endpoint.unwrap_or_else(|| {
            std::env::var("OSS_ENDPOINT")
                .ok()
                .as_deref()
                .unwrap_or("oss-cn-hangzhou.aliyuncs.com")
        });

        tracing::info!("正在上传文件到 OSS: {} -> bucket={}, key={}, endpoint={}", 
            file_path.display(), bucket, object_key, ep);

        // 读取文件内容
        let file_content = fs::read(file_path)
            .context(format!("读取文件失败: {}", file_path.display()))?;

        // 获取 Content-Type
        let content_type = self.guess_content_type(object_key);

        // 构建 URL
        let url = self.build_url(bucket, object_key, ep);

        // 构建签名请求头
        let headers = self.build_signed_headers("PUT", bucket, object_key, Some(content_type))?;

        // 上传文件
        let response = self.client
            .put(&url)
            .headers(headers)
            .body(file_content)
            .send()
            .await
            .context("上传文件失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("上传文件失败: HTTP {} - {}", status, error_text);
        }

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
        let ep = endpoint.unwrap_or_else(|| {
            std::env::var("OSS_ENDPOINT")
                .ok()
                .as_deref()
                .unwrap_or("oss-cn-hangzhou.aliyuncs.com")
        });

        tracing::debug!("检查 OSS 对象: bucket={}, key={}, endpoint={}", bucket, object_key, ep);

        // 构建 URL
        let url = self.build_url(bucket, object_key, ep);

        // 构建签名请求头
        let headers = self.build_signed_headers("HEAD", bucket, object_key, None)?;

        // 发送 HEAD 请求
        let response = self.client
            .head(&url)
            .headers(headers)
            .send()
            .await
            .context("检查对象失败")?;

        match response.status().as_u16() {
            200 => {
                // 文件存在，获取 ETag
                let etag = response.headers()
                    .get("etag")
                    .or_else(|| response.headers().get("ETag"))
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.trim_matches('"').to_string());
                Ok(etag)
            }
            404 => {
                // 文件不存在
                Ok(None)
            }
            _ => {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("检查对象失败: HTTP {} - {}", status, error_text);
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
