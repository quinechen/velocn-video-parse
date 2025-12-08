use serde::{Deserialize, Serialize};

/// 阿里云 OSS Event 数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OssEvent {
    /// 事件名称
    #[serde(rename = "events")]
    pub events: Vec<OssEventItem>,
}

/// OSS Event 项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OssEventItem {
    /// 事件名称
    #[serde(rename = "eventName")]
    pub event_name: String,
    
    /// 事件源
    #[serde(rename = "eventSource")]
    pub event_source: String,
    
    /// 事件时间
    #[serde(rename = "eventTime")]
    pub event_time: String,
    
    /// 事件版本
    #[serde(rename = "eventVersion")]
    pub event_version: String,
    
    /// OSS 信息
    #[serde(rename = "oss")]
    pub oss: OssInfo,
    
    /// 区域信息
    #[serde(rename = "region")]
    pub region: String,
    
    /// 请求参数
    #[serde(rename = "requestParameters")]
    pub request_parameters: RequestParameters,
    
    /// 响应元素
    #[serde(rename = "responseElements")]
    pub response_elements: ResponseElements,
    
    /// 用户身份
    #[serde(rename = "userIdentity")]
    pub user_identity: UserIdentity,
}

/// OSS 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OssInfo {
    /// Bucket 信息
    #[serde(rename = "bucket")]
    pub bucket: BucketInfo,
    
    /// 对象信息
    #[serde(rename = "object")]
    pub object: ObjectInfo,
    
    /// OSS Schema 版本
    #[serde(rename = "ossSchemaVersion")]
    pub oss_schema_version: String,
    
    /// 规则 ID
    #[serde(rename = "ruleId")]
    pub rule_id: String,
}

/// Bucket 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    /// Bucket ARN
    #[serde(rename = "arn")]
    pub arn: String,
    
    /// Bucket 名称
    #[serde(rename = "name")]
    pub name: String,
    
    /// 拥有者身份
    #[serde(rename = "ownerIdentity")]
    pub owner_identity: UserIdentity,
    
    /// 虚拟主机名
    #[serde(rename = "virtualHostedBucketName")]
    pub virtual_hosted_bucket_name: String,
}

/// 对象信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    /// Delta 大小
    #[serde(rename = "deltaSize")]
    pub delta_size: Option<i64>,
    
    /// ETag
    #[serde(rename = "eTag")]
    pub e_tag: String,
    
    /// 键（文件路径）
    #[serde(rename = "key")]
    pub key: String,
    
    /// 大小
    #[serde(rename = "size")]
    pub size: i64,
}

/// 请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestParameters {
    /// 源 IP
    #[serde(rename = "sourceIPAddress")]
    pub source_ip_address: String,
}

/// 响应元素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseElements {
    /// 请求 ID
    #[serde(rename = "requestId")]
    pub request_id: String,
}

/// 用户身份
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// 主体 ID
    #[serde(rename = "principalId")]
    pub principal_id: String,
}

/// 处理请求的响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResponse {
    /// 是否成功
    pub success: bool,
    
    /// 消息
    pub message: String,
    
    /// 处理结果信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ProcessResult>,
}

/// 处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    /// 视频文件路径
    pub video_file: String,
    
    /// 输出目录
    pub output_dir: String,
    
    /// 场景数量
    pub scene_count: usize,
    
    /// 关键帧文件列表
    pub keyframes: Vec<String>,
    
    /// 音频文件
    pub audio_file: String,
    
    /// 元数据文件
    pub metadata_file: String,
}