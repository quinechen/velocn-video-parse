use axum::{
    extract::{Json, Query},
    body::Bytes,
    http::{StatusCode, HeaderMap},
    response::Json as ResponseJson,
};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::{OssEvent, ProcessResponse, ProcessResult, OssClient, ProcessConfig, process_video, config::ConfigLoader};
use tracing::{info, error, warn, debug};

/// 处理 OSS Event 的 Handler（接受任何HTTP方法）
/// 用于函数计算环境，兼容不同的调用方式
pub async fn handle_oss_event_any(
    headers: HeaderMap,
    body: Bytes,
) -> Result<ResponseJson<ProcessResponse>, (StatusCode, String)> {
    // 从请求头读取请求ID（如果存在）
    let request_id = headers
        .get("x-fc-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("[OSS Event Any] 收到请求 RequestId: {}", request_id);
    info!("请求方法: 任意");
    
    // 打印请求头信息（用于调试）
    debug!("请求头信息:");
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            debug!("  {}: {}", name, value_str);
        }
    }
    
    // 尝试解析请求体为 JSON
    if body.is_empty() {
        error!("[OSS Event Any] 请求体为空");
        return Err((
            StatusCode::BAD_REQUEST,
            "请求体为空".to_string(),
        ));
    }
    
    let body_str = String::from_utf8_lossy(&body);
    info!("请求体内容: {}", body_str);
    
    // 解析为 OSS 事件
    let event: OssEvent = serde_json::from_slice(&body)
        .map_err(|e| {
            error!("[OSS Event Any] 解析 JSON 失败: {}", e);
            (StatusCode::BAD_REQUEST, format!("解析 JSON 失败: {}", e))
        })?;
    
    // 调用原有的处理逻辑
    handle_oss_event_internal(event, Some(request_id.to_string())).await
}

/// 处理 OSS Event 的 Handler（原始版本，仅接受POST JSON）
pub async fn handle_oss_event(
    Json(event): Json<OssEvent>,
) -> Result<ResponseJson<ProcessResponse>, (StatusCode, String)> {
    handle_oss_event_internal(event, None).await
}

/// 内部处理 OSS Event 的逻辑（提取公共部分）
async fn handle_oss_event_internal(
    event: OssEvent,
    request_id: Option<String>,
) -> Result<ResponseJson<ProcessResponse>, (StatusCode, String)> {
    // 记录接收到的请求详情
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("[OSS Event] 收到 OSS 事件触发请求");
    info!("事件数量: {}", event.events.len());
    
    if !event.events.is_empty() {
        let event_item = &event.events[0];
        info!("事件详情:");
        info!("  • 事件名称: {}", event_item.event_name);
        info!("  • 事件源: {}", event_item.event_source);
        info!("  • 事件时间: {}", event_item.event_time);
        info!("  • 区域: {}", event_item.region);
        info!("  • Bucket: {}", event_item.oss.bucket.name);
        info!("  • Object Key: {}", event_item.oss.object.key);
        info!("  • 文件大小: {} bytes", event_item.oss.object.size);
        info!("  • ETag: {}", event_item.oss.object.e_tag);
        info!("  • 请求ID: {}", event_item.response_elements.request_id);
        info!("  • 源IP: {}", event_item.request_parameters.source_ip_address);
        debug!("完整事件数据: {:?}", event);
    } else {
        warn!("收到空事件列表");
    }
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // DEBUG 模式：如果设置了 DEBUG=true，直接返回成功，用于测试部署和事件触发
    if std::env::var("DEBUG")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true"
    {
        info!("DEBUG 模式已启用，跳过实际处理，直接返回成功");
        
        // 提取事件信息用于日志
        let event_info = if !event.events.is_empty() {
            let event_item = &event.events[0];
            format!(
                "bucket={}, key={}, region={}, eventName={}",
                event_item.oss.bucket.name,
                event_item.oss.object.key,
                event_item.region,
                event_item.event_name
            )
        } else {
            "无事件信息".to_string()
        };
        
        info!("DEBUG 模式 - 事件信息: {}", event_info);
        
        return Ok(ResponseJson(ProcessResponse {
            success: true,
            message: format!("DEBUG 模式：事件接收成功，事件信息: {}", event_info),
            result: None,
        }));
    }

    // 检查是否有事件
    if event.events.is_empty() {
        error!("[OSS Event] 事件列表为空，无法处理");
        return Err((
            StatusCode::BAD_REQUEST,
            "事件列表为空".to_string(),
        ));
    }

    // 处理第一个事件（通常只有一个）
    let event_item = &event.events[0];
    
    // 只处理 ObjectCreated 事件（包括 Put, Post, Copy, CompleteMultipartUpload, PutSymlink）
    if !event_item.event_name.starts_with("ObjectCreated") {
        warn!("[OSS Event] 不支持的事件类型: {}，跳过处理", event_item.event_name);
        return Ok(ResponseJson(ProcessResponse {
            success: false,
            message: format!("不支持的事件类型: {}", event_item.event_name),
            result: None,
        }));
    }

    let bucket = &event_item.oss.bucket.name;
    let object_key = event_item.oss.object.key.clone();
    let region = &event_item.region;
    
    // 处理符号链接事件（参考 Python 示例）
    if event_item.event_name == "ObjectCreated:PutSymlink" {
        // TODO: 实现符号链接解析
        // 在函数计算环境中，符号链接需要通过 OSS API 解析
        // 当前先记录日志，后续可以实现 get_symlink 功能
        warn!("[OSS Event] 检测到符号链接事件，需要解析符号链接: {}，当前暂不支持", object_key);
        // 注意：符号链接解析需要 OSS SDK 支持，当前暂不处理
        return Ok(ResponseJson(ProcessResponse {
            success: false,
            message: format!("暂不支持符号链接事件: {}", event_item.event_name),
            result: None,
        }));
    }
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🎬 [视频处理] 开始处理视频文件");
    info!("  • Bucket: {}", bucket);
    info!("  • Object Key: {}", object_key);
    info!("  • Region: {}", region);
    info!("  • 文件大小: {} bytes ({:.2} MB)", 
        event_item.oss.object.size,
        event_item.oss.object.size as f64 / 1024.0 / 1024.0);
    
    let process_start_time = std::time::Instant::now();

    // 创建临时目录
    // 尝试使用函数计算的 request_id（优先使用传入的参数，其次环境变量，最后生成）
    let request_id = request_id
        .or_else(|| std::env::var("FC_REQUEST_ID").ok())
        .unwrap_or_else(|| {
            format!("{}_{}", 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                uuid::Uuid::new_v4().to_string()
            )
        });
    info!("📁 [视频处理] 创建临时目录 RequestId: {}", request_id);
    let temp_dir = std::env::temp_dir().join("video-parse").join(&request_id);
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| {
            error!("❌ [视频处理] 创建临时目录失败: {} (路径: {})", e, temp_dir.display());
            (StatusCode::INTERNAL_SERVER_ERROR, format!("创建临时目录失败: {}", e))
        })?;
    info!("✅ [视频处理] 临时目录创建成功: {}", temp_dir.display());

    // 下载视频文件
    info!("🔧 [视频处理] 初始化 OSS 客户端...");
    let oss_client = OssClient::new()
        .map_err(|e| {
            error!("❌ [视频处理] 创建 OSS 客户端失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("创建 OSS 客户端失败: {}", e))
        })?;
    info!("✅ [视频处理] OSS 客户端初始化成功");
    
    let video_path_buf = PathBuf::from(&object_key);
    let video_filename = video_path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4");
    
    let video_path = temp_dir.join(video_filename);
    
    // 构建 internal endpoint（内网访问更快且免费）
    // 格式: oss-{region}-internal.aliyuncs.com
    let endpoint = format!("oss-{}-internal.aliyuncs.com", region);
    
    // 尝试从 OSS 下载（使用 internal endpoint）
    info!("⬇️  [视频处理] 开始下载视频文件");
    info!("  • 源地址: oss://{}/{}", bucket, object_key);
    info!("  • 目标路径: {}", video_path.display());
    info!("  • Endpoint: {}", endpoint);
    let download_start = std::time::Instant::now();
    let downloaded_path = oss_client
        .download_file(bucket, &object_key, Some(&endpoint), &video_path)
        .await
        .map_err(|e| {
            error!("❌ [视频处理] 下载文件失败: bucket={}, key={}, error={}", bucket, object_key, e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("下载文件失败: {}", e))
        })?;
    let download_duration = download_start.elapsed();
    let file_size_mb = event_item.oss.object.size as f64 / 1024.0 / 1024.0;
    let download_speed = file_size_mb / download_duration.as_secs_f64();
    info!("✅ [视频处理] 文件下载成功");
    info!("  • 文件路径: {}", downloaded_path.display());
    info!("  • 下载耗时: {:.2}秒", download_duration.as_secs_f64());
    info!("  • 下载速度: {:.2} MB/s", download_speed);
    
    // 验证下载的文件
    if let Ok(metadata) = std::fs::metadata(&downloaded_path) {
        info!("  • 实际文件大小: {} bytes ({:.2} MB)", 
            metadata.len(),
            metadata.len() as f64 / 1024.0 / 1024.0);
    }

    // 创建输出目录
    info!("📁 [视频处理] 创建输出目录...");
    let output_dir = temp_dir.join("output");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| {
            error!("❌ [视频处理] 创建输出目录失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("创建输出目录失败: {}", e))
        })?;
    info!("✅ [视频处理] 输出目录创建成功: {}", output_dir.display());
    
    // 处理视频：从环境变量和配置文件加载配置
    info!("⚙️  [视频处理] 加载处理配置...");
    let config = ConfigLoader::load_config(None, None, None, None, None)
        .unwrap_or_else(|_| ProcessConfig::default());
    info!("📋 [视频处理] 处理配置:");
    info!("  • 场景检测阈值: {:.2}", config.threshold);
    info!("  • 最小场景持续时间: {:.2}秒", config.min_scene_duration);
    info!("  • 帧采样率: {:.2} fps", config.sample_rate);
    
    info!("🎞️  [视频处理] 开始视频拉片处理...");
    info!("  • 输入文件: {}", downloaded_path.display());
    info!("  • 输出目录: {}", output_dir.display());
    let video_process_start = std::time::Instant::now();
    let process_result = process_video(&downloaded_path, &output_dir, config)
        .await
        .map_err(|e| {
            error!("❌ [视频处理] 处理视频失败: path={}, error={}", downloaded_path.display(), e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("处理视频失败: {}", e))
        })?;
    let video_process_duration = video_process_start.elapsed();
    info!("✅ [视频处理] 视频处理完成");
    info!("  • 处理耗时: {:.2}秒", video_process_duration.as_secs_f64());
    info!("  • 检测到场景数: {}", process_result.metadata.scene_count);
    info!("  • 提取关键帧数: {}", process_result.keyframe_files.len());
    info!("  • 音频文件: {}", process_result.audio_file);

    // 上传处理结果到目标 bucket（如果配置了目标 bucket）
    let (uploaded_files, upload_duration) = if let (Some(dest_bucket), Some(dest_region)) = (
        std::env::var("DESTINATION_BUCKET").ok(),
        std::env::var("DESTINATION_REGION").ok(),
    ) {
        info!("⬆️  [视频处理] 开始上传处理结果到目标 bucket");
        info!("  • 目标 Bucket: {}", dest_bucket);
        info!("  • 目标 Region: {}", dest_region);
        let upload_start = std::time::Instant::now();
        
        // 构建目标 endpoint
        let dest_endpoint = format!("oss-{}-internal.aliyuncs.com", dest_region);
        
        // 构建目标路径前缀（保持源文件的目录结构）
        let dest_prefix = std::env::var("DESTINATION_PREFIX")
            .unwrap_or_else(|_| {
                // 默认使用源文件的目录部分作为前缀
                PathBuf::from(&object_key)
                    .parent()
                    .and_then(|p| p.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "processed".to_string())
            });
        
        let mut uploaded = Vec::new();
        let mut upload_errors = Vec::new();
        
        // 上传关键帧
        // keyframe_files 是文件名列表，需要与 output_dir 组合成完整路径
        for keyframe_filename in &process_result.keyframe_files {
            let keyframe_path = output_dir.join(keyframe_filename);
            if keyframe_path.exists() {
                let keyframe_name = keyframe_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("keyframe.jpg");
                let dest_key = format!("{}/keyframes/{}", dest_prefix, keyframe_name);
                
                match oss_client.upload_file(
                    &dest_bucket,
                    &dest_key,
                    &keyframe_path,
                    Some(&dest_endpoint),
                ).await {
                    Ok(_) => {
                        info!("已上传关键帧: {} -> {}", keyframe_path.display(), dest_key);
                        uploaded.push(dest_key.clone());
                    }
                    Err(e) => {
                        let error_msg = format!("上传关键帧失败 {}: {}", dest_key, e);
                        error!("{}", error_msg);
                        upload_errors.push(error_msg);
                    }
                }
            } else {
                let error_msg = format!("关键帧文件不存在: {}", keyframe_path.display());
                error!("{}", error_msg);
                upload_errors.push(error_msg);
            }
        }
        
        // 上传音频文件
        // audio_file 是文件名，需要与 output_dir 组合成完整路径
        let audio_path = output_dir.join(&process_result.audio_file);
        if audio_path.exists() {
            let audio_name = audio_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("audio.aac");
            let dest_key = format!("{}/{}", dest_prefix, audio_name);
            
            match oss_client.upload_file(
                &dest_bucket,
                &dest_key,
                &audio_path,
                Some(&dest_endpoint),
            ).await {
                Ok(_) => {
                    info!("已上传音频文件: {} -> {}", audio_path.display(), dest_key);
                    uploaded.push(dest_key.clone());
                }
                Err(e) => {
                    let error_msg = format!("上传音频文件失败 {}: {}", dest_key, e);
                    error!("{}", error_msg);
                    upload_errors.push(error_msg);
                }
            }
        } else {
            let error_msg = format!("音频文件不存在: {}", audio_path.display());
            error!("{}", error_msg);
            upload_errors.push(error_msg);
        }
        
        // 上传元数据文件
        let metadata_path = output_dir.join("metadata.json");
        if metadata_path.exists() {
            let dest_key = format!("{}/metadata.json", dest_prefix);
            
            match oss_client.upload_file(
                &dest_bucket,
                &dest_key,
                &metadata_path,
                Some(&dest_endpoint),
            ).await {
                Ok(_) => {
                    info!("已上传元数据文件: {} -> {}", metadata_path.display(), dest_key);
                    uploaded.push(dest_key.clone());
                }
                Err(e) => {
                    let error_msg = format!("上传元数据文件失败 {}: {}", dest_key, e);
                    error!("{}", error_msg);
                    upload_errors.push(error_msg);
                }
            }
        } else {
            let error_msg = format!("元数据文件不存在: {}", metadata_path.display());
            error!("{}", error_msg);
            upload_errors.push(error_msg);
        }
        
        // 记录上传结果
        let upload_duration = upload_start.elapsed();
        if !upload_errors.is_empty() {
            warn!("⚠️  [视频处理] 部分文件上传失败，共 {} 个错误", upload_errors.len());
            for err in &upload_errors {
                warn!("  • {}", err);
            }
        }
        
        info!("✅ [视频处理] 上传完成");
        info!("  • 上传耗时: {:.2}秒", upload_duration.as_secs_f64());
        info!("  • 成功: {} 个文件", uploaded.len());
        info!("  • 失败: {} 个文件", upload_errors.len());
        
        (Some(uploaded), Some(upload_duration))
    } else {
        info!("ℹ️  [视频处理] 未配置目标 bucket，跳过上传");
        (None, None)
    };
    
    let total_duration = process_start_time.elapsed();
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🎉 [视频处理] 全部处理完成");
    info!("  • 总耗时: {:.2}秒", total_duration.as_secs_f64());
    info!("  • 下载耗时: {:.2}秒", download_duration.as_secs_f64());
    info!("  • 处理耗时: {:.2}秒", video_process_duration.as_secs_f64());
    if let Some(duration) = upload_duration {
        info!("  • 上传耗时: {:.2}秒", duration.as_secs_f64());
    }
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 清理临时目录（可选，函数计算会自动清理）
    // 如果需要保留文件用于调试，可以注释掉下面的代码
    // if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
    //     error!("清理临时目录失败: {}", e);
    // }

    // 构建响应
    let response = ProcessResponse {
        success: true,
        message: format!(
            "成功处理视频，检测到 {} 个场景{}",
            process_result.metadata.scene_count,
            if uploaded_files.is_some() {
                "，已上传到目标 bucket"
            } else {
                ""
            }
        ),
        result: Some(ProcessResult {
            video_file: downloaded_path.to_string_lossy().to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
            scene_count: process_result.metadata.scene_count,
            keyframes: process_result.keyframe_files.clone(),
            audio_file: process_result.audio_file.clone(),
            metadata_file: "metadata.json".to_string(),
        }),
    };

    // 记录处理完成后的详细输出日志
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("[OSS Event] 处理完成");
    info!("处理结果:");
    info!("  • 状态: 成功");
    info!("  • 视频文件: {}", downloaded_path.display());
    info!("  • 输出目录: {}", output_dir.display());
    info!("  • 检测到场景数: {}", process_result.metadata.scene_count);
    info!("  • 关键帧数量: {}", process_result.keyframe_files.len());
    if !process_result.keyframe_files.is_empty() {
        info!("  • 关键帧文件:");
        for (idx, keyframe) in process_result.keyframe_files.iter().enumerate() {
            info!("    {}. {}", idx + 1, keyframe);
        }
    }
    info!("  • 音频文件: {}", process_result.audio_file);
    info!("  • 元数据文件: metadata.json");
    if let Some(ref uploaded) = uploaded_files {
        info!("  • 已上传文件数: {}", uploaded.len());
        if !uploaded.is_empty() {
            info!("  • 上传文件列表:");
            for (idx, file) in uploaded.iter().enumerate() {
                info!("    {}. {}", idx + 1, file);
            }
        }
    } else {
        info!("  • 上传状态: 未配置目标 bucket，未上传");
    }
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(ResponseJson(response))
}

/// 通用 JSON 响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// 健康检查 Handler
pub async fn health_check() -> ResponseJson<JsonResponse> {
    info!("[Health Check] 收到健康检查请求");
    ResponseJson(JsonResponse {
        success: true,
        message: "服务运行正常".to_string(),
        data: Some(serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    })
}

/// 检查文件扩展名是否为视频文件
fn is_video_file(filename: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    let video_extensions = [
        "mp4", "avi", "mov", "mkv", "wmv", "flv", "webm", "m4v",
        "mpg", "mpeg", "3gp", "3g2", "asf", "rm", "rmvb", "vob",
        "ts", "mts", "m2ts", "f4v", "ogv", "divx", "xvid",
    ];
    
    if let Some(ext) = PathBuf::from(&filename_lower)
        .extension()
        .and_then(|e| e.to_str())
    {
        video_extensions.contains(&ext)
    } else {
        false
    }
}

/// 直接处理请求（支持本地文件路径或OSS事件）
#[derive(Debug, Deserialize)]
pub struct DirectProcessRequest {
    /// 视频文件路径（本地路径或OSS路径）
    pub input: String,
    /// 输出目录（可选，默认使用临时目录）
    pub output: Option<String>,
    /// 场景变化检测阈值
    pub threshold: Option<f64>,
    /// 最小场景持续时间（秒）
    pub min_scene_duration: Option<f64>,
    /// 帧采样率
    pub sample_rate: Option<f64>,
    /// 是否为OSS路径（如果为true，会从OSS下载）
    pub is_oss_path: Option<bool>,
    /// OSS bucket（如果is_oss_path为true，需要提供）
    pub oss_bucket: Option<String>,
    /// OSS region（如果is_oss_path为true，需要提供）
    pub oss_region: Option<String>,
}

/// 直接处理视频的 Handler（支持本地文件和OSS文件）
pub async fn handle_direct_process(
    Json(request): Json<DirectProcessRequest>,
) -> Result<ResponseJson<ProcessResponse>, (StatusCode, String)> {
    // 记录接收到的请求详情
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("[Direct Process] 收到直接处理请求");
    info!("请求详情:");
    info!("  • 输入文件: {}", request.input);
    info!("  • 输出目录: {:?}", request.output);
    info!("  • 是否为OSS路径: {:?}", request.is_oss_path);
    if request.is_oss_path.unwrap_or(false) {
        info!("  • OSS Bucket: {:?}", request.oss_bucket);
        info!("  • OSS Region: {:?}", request.oss_region);
    }
    if request.threshold.is_some() || request.min_scene_duration.is_some() || request.sample_rate.is_some() {
        info!("  • 自定义参数:");
        if let Some(t) = request.threshold {
            info!("    - threshold: {}", t);
        }
        if let Some(m) = request.min_scene_duration {
            info!("    - min_scene_duration: {}s", m);
        }
        if let Some(s) = request.sample_rate {
            info!("    - sample_rate: {} fps", s);
        }
    }
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 确定输入文件路径
    let input_path = if request.is_oss_path.unwrap_or(false) {
        // OSS路径，需要下载
        let bucket = request.oss_bucket.ok_or_else(|| {
            (StatusCode::BAD_REQUEST, "OSS路径需要提供 oss_bucket".to_string())
        })?;
        let region = request.oss_region.ok_or_else(|| {
            (StatusCode::BAD_REQUEST, "OSS路径需要提供 oss_region".to_string())
        })?;
        
        // 创建临时目录
        let request_id = std::env::var("FC_REQUEST_ID")
            .unwrap_or_else(|_| {
                format!("{}_{}", 
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    uuid::Uuid::new_v4().to_string()
                )
            });
        let temp_dir = std::env::temp_dir().join("video-parse").join(&request_id);
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| {
                error!("[Direct Process] 创建临时目录失败: {} (路径: {})", e, temp_dir.display());
                (StatusCode::INTERNAL_SERVER_ERROR, format!("创建临时目录失败: {}", e))
            })?;
        
        // 下载文件
        let oss_client = OssClient::new()
            .map_err(|e| {
                error!("[Direct Process] 创建 OSS 客户端失败: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("创建 OSS 客户端失败: {}", e))
            })?;
        
        let input_path_buf = PathBuf::from(&request.input);
        let video_filename = input_path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("video.mp4");
        let video_path = temp_dir.join(video_filename);
        
        let endpoint = format!("oss-{}-internal.aliyuncs.com", region);
        
        info!("[Direct Process] 开始下载OSS文件: bucket={}, key={}, endpoint={}", bucket, request.input, endpoint);
        oss_client
            .download_file(&bucket, &request.input, Some(&endpoint), &video_path)
            .await
            .map_err(|e| {
                error!("[Direct Process] 下载文件失败: bucket={}, key={}, error={}", bucket, request.input, e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("下载文件失败: {}", e))
            })?;
        info!("[Direct Process] 文件下载成功: {}", video_path.display());
        
        video_path
    } else {
        // 本地路径
        PathBuf::from(&request.input)
    };
    
    // 检查文件是否存在
    if !input_path.exists() {
        error!("[Direct Process] 视频文件不存在: {}", input_path.display());
        return Err((
            StatusCode::NOT_FOUND,
            format!("视频文件不存在: {}", input_path.display()),
        ));
    }
    
    // 确定输出目录
    let output_dir = if let Some(output) = request.output {
        PathBuf::from(output)
    } else {
        // 使用临时目录
        let request_id = std::env::var("FC_REQUEST_ID")
            .unwrap_or_else(|_| {
                format!("{}_{}", 
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    uuid::Uuid::new_v4().to_string()
                )
            });
        std::env::temp_dir().join("video-parse").join(&request_id).join("output")
    };
    
    // 创建输出目录
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| {
            error!("[Direct Process] 创建输出目录失败: {} (路径: {})", e, output_dir.display());
            (StatusCode::INTERNAL_SERVER_ERROR, format!("创建输出目录失败: {}", e))
        })?;
    
    // 构建配置：优先级为 请求参数 > 环境变量 > 配置文件 > 默认值
    let config = ConfigLoader::load_config(
        None,
        request.threshold,
        request.min_scene_duration,
        request.sample_rate,
        None, // webhook_url 从配置文件或环境变量读取
    )
    .unwrap_or_else(|_| ProcessConfig::default());
    
    // 处理视频
    info!("[Direct Process] 开始处理视频: {}", input_path.display());
    let process_result = process_video(&input_path, &output_dir, config)
        .await
        .map_err(|e| {
            error!("[Direct Process] 处理视频失败: path={}, error={}", input_path.display(), e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("处理视频失败: {}", e))
        })?;
    info!("[Direct Process] 视频处理完成: 场景数={}", process_result.metadata.scene_count);
    
    // 构建响应
    let response = ProcessResponse {
        success: true,
        message: format!(
            "成功处理视频，检测到 {} 个场景",
            process_result.metadata.scene_count
        ),
        result: Some(ProcessResult {
            video_file: input_path.to_string_lossy().to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
            scene_count: process_result.metadata.scene_count,
            keyframes: process_result.keyframe_files.clone(),
            audio_file: process_result.audio_file.clone(),
            metadata_file: "metadata.json".to_string(),
        }),
    };
    
    // 记录处理完成后的详细输出日志
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("[Direct Process] 处理完成");
    info!("处理结果:");
    info!("  • 状态: 成功");
    info!("  • 视频文件: {}", input_path.display());
    info!("  • 输出目录: {}", output_dir.display());
    info!("  • 检测到场景数: {}", process_result.metadata.scene_count);
    info!("  • 关键帧数量: {}", process_result.keyframe_files.len());
    if !process_result.keyframe_files.is_empty() {
        info!("  • 关键帧文件:");
        for (idx, keyframe) in process_result.keyframe_files.iter().enumerate() {
            info!("    {}. {}", idx + 1, keyframe);
        }
    }
    info!("  • 音频文件: {}", process_result.audio_file);
    info!("  • 元数据文件: metadata.json");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(ResponseJson(response))
}

/// 处理视频的查询参数版本（用于GET请求，方便测试）
#[derive(Debug, Deserialize)]
pub struct ProcessQueryParams {
    pub input: String,
    pub output: Option<String>,
    pub threshold: Option<f64>,
    pub min_scene_duration: Option<f64>,
    pub sample_rate: Option<f64>,
}

/// 通过查询参数处理视频（GET请求，方便测试）
pub async fn handle_process_query(
    Query(params): Query<ProcessQueryParams>,
) -> Result<ResponseJson<ProcessResponse>, (StatusCode, String)> {
    // 记录接收到的请求详情
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("[Process Query] 收到查询参数处理请求");
    info!("请求详情:");
    info!("  • 输入文件: {}", params.input);
    info!("  • 输出目录: {:?}", params.output);
    if params.threshold.is_some() || params.min_scene_duration.is_some() || params.sample_rate.is_some() {
        info!("  • 自定义参数:");
        if let Some(t) = params.threshold {
            info!("    - threshold: {}", t);
        }
        if let Some(m) = params.min_scene_duration {
            info!("    - min_scene_duration: {}s", m);
        }
        if let Some(s) = params.sample_rate {
            info!("    - sample_rate: {} fps", s);
        }
    }
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let request = DirectProcessRequest {
        input: params.input,
        output: params.output,
        threshold: params.threshold,
        min_scene_duration: params.min_scene_duration,
        sample_rate: params.sample_rate,
        is_oss_path: Some(false),
        oss_bucket: None,
        oss_region: None,
    };
    
    handle_direct_process(Json(request)).await
}

/// 函数计算初始化端点
/// 函数计算在启动时会调用此端点进行初始化
pub async fn handle_initialize(
    headers: HeaderMap,
) -> Result<ResponseJson<JsonResponse>, (StatusCode, String)> {
    // 从请求头读取请求ID
    let request_id = headers
        .get("x-fc-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("FC Initialize Start RequestId: {}", request_id);
    
    // 可以在这里进行初始化操作，比如：
    // - 加载配置
    // - 初始化连接池
    // - 预热资源等
    
    info!("FC Initialize End RequestId: {}", request_id);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(ResponseJson(JsonResponse {
        success: true,
        message: "FunctionCompute 初始化完成".to_string(),
        data: Some(serde_json::json!({
            "request_id": request_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    }))
}

/// 函数计算调用端点
/// 这是函数计算事件驱动的主要入口点，OSS事件会通过此端点传递
/// 接受任何HTTP方法，打印日志，返回JSON
pub async fn handle_invoke(
    headers: HeaderMap,
    body: Bytes,
) -> Result<ResponseJson<JsonResponse>, (StatusCode, String)> {
    // 从请求头读取请求ID
    let request_id = headers
        .get("x-fc-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("FC Invoke Start RequestId: {}", request_id);
    
    // 打印请求头信息（用于调试）
    debug!("请求头信息:");
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            debug!("  {}: {}", name, value_str);
        }
    }
    
    // 打印请求体内容
    let body_str = String::from_utf8_lossy(&body);
    info!("请求体内容: {}", body_str);
    
    // 尝试解析为 OSS 事件并处理
    if !body.is_empty() {
        match serde_json::from_slice::<OssEvent>(&body) {
            Ok(event) => {
                info!("成功解析为 OSS 事件，事件数量: {}", event.events.len());
                
                // 处理事件
                if !event.events.is_empty() {
                    let event_item = &event.events[0];
                    let object_key = &event_item.oss.object.key;
                    
                    info!("OSS 事件详情:");
                    info!("  • 事件名称: {}", event_item.event_name);
                    info!("  • 事件源: {}", event_item.event_source);
                    info!("  • 事件时间: {}", event_item.event_time);
                    info!("  • 区域: {}", event_item.region);
                    info!("  • Bucket: {}", event_item.oss.bucket.name);
                    info!("  • Object Key: {}", object_key);
                    info!("  • 文件大小: {} bytes", event_item.oss.object.size);
                    info!("  • ETag: {}", event_item.oss.object.e_tag);
                    info!("  • 请求ID: {}", event_item.response_elements.request_id);
                    info!("  • 源IP: {}", event_item.request_parameters.source_ip_address);
                    debug!("完整事件数据: {:?}", event);
                    
                    // 检查文件类型
                    if !is_video_file(object_key) {
                        info!("文件 {} 不是视频文件，跳过处理", object_key);
                        info!("FC Invoke End RequestId: {}", request_id);
                        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        return Ok(ResponseJson(JsonResponse {
                            success: true,
                            message: format!("文件 {} 不是视频文件，已跳过处理", object_key),
                            data: Some(serde_json::json!({
                                "request_id": request_id,
                                "object_key": object_key,
                                "file_type": "non-video",
                                "skipped": true,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        }));
                    }
                    
                    // 只处理 ObjectCreated 事件
                    if !event_item.event_name.starts_with("ObjectCreated") {
                        info!("事件类型 {} 不是 ObjectCreated，跳过处理", event_item.event_name);
                        info!("FC Invoke End RequestId: {}", request_id);
                        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        return Ok(ResponseJson(JsonResponse {
                            success: true,
                            message: format!("事件类型 {} 不是 ObjectCreated，已跳过处理", event_item.event_name),
                            data: Some(serde_json::json!({
                                "request_id": request_id,
                                "event_name": event_item.event_name,
                                "skipped": true,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        }));
                    }
                    
                    // 是视频文件，调用处理逻辑
                    info!("✅ 检测到视频文件，开始处理: {}", object_key);
                    info!("📋 处理参数:");
                    info!("  • Bucket: {}", event_item.oss.bucket.name);
                    info!("  • Region: {}", event_item.region);
                    info!("  • 文件大小: {} bytes ({:.2} MB)", 
                        event_item.oss.object.size,
                        event_item.oss.object.size as f64 / 1024.0 / 1024.0);
                    
                    // 调用内部处理函数（异步处理，不阻塞响应）
                    // 注意：这里我们启动一个异步任务来处理，立即返回 JSON 响应
                    let event_clone = event.clone();
                    let request_id_clone = request_id.to_string();
                    let bucket_clone = event_item.oss.bucket.name.clone();
                    let object_key_clone = object_key.to_string();
                    
                    tokio::spawn(async move {
                        info!("🚀 [异步任务] 开始处理视频: bucket={}, key={}", bucket_clone, object_key_clone);
                        let start_time = std::time::Instant::now();
                        
                        match handle_oss_event_internal(event_clone, Some(request_id_clone.clone())).await {
                            Ok(response) => {
                                let duration = start_time.elapsed();
                                info!("✅ [异步任务] 视频处理成功完成 RequestId: {}", request_id_clone);
                                info!("⏱️  [异步任务] 总耗时: {:.2}秒", duration.as_secs_f64());
                                if let Some(ref result) = response.0.result {
                                    info!("📊 [异步任务] 处理结果:");
                                    info!("  • 场景数: {}", result.scene_count);
                                    info!("  • 关键帧数: {}", result.keyframes.len());
                                    info!("  • 音频文件: {}", result.audio_file);
                                }
                            }
                            Err(e) => {
                                let duration = start_time.elapsed();
                                error!("❌ [异步任务] 处理 OSS 事件失败 RequestId: {}, 耗时: {:.2}秒, 错误: {:?}", 
                                    request_id_clone, duration.as_secs_f64(), e);
                            }
                        }
                    });
                    
                    info!("✅ 已启动异步处理任务，任务ID: {}", request_id);
                } else {
                    warn!("OSS 事件列表为空");
                }
            }
            Err(e) => {
                debug!("请求体不是有效的 OSS 事件 JSON: {}", e);
                info!("请求体内容（非JSON）: {}", body_str);
            }
        }
    } else {
        info!("请求体为空");
    }
    
    info!("FC Invoke End RequestId: {}", request_id);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(ResponseJson(JsonResponse {
        success: true,
        message: "请求已接收".to_string(),
        data: Some(serde_json::json!({
            "request_id": request_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    }))
}