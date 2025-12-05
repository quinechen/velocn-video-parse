use axum::{
    extract::Json,
    http::StatusCode,
    response::Json as ResponseJson,
};
use std::path::PathBuf;
use crate::{OssEvent, ProcessResponse, ProcessResult, OssClient, ProcessConfig, process_video};
use tracing::{info, error};

/// 处理 OSS Event 的 Handler
pub async fn handle_oss_event(
    Json(event): Json<OssEvent>,
) -> Result<ResponseJson<ProcessResponse>, (StatusCode, String)> {
    info!("收到 OSS Event: {:?}", event);

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
        return Err((
            StatusCode::BAD_REQUEST,
            "事件列表为空".to_string(),
        ));
    }

    // 处理第一个事件（通常只有一个）
    let event_item = &event.events[0];
    
    // 只处理 ObjectCreated 事件（包括 Put, Post, Copy, CompleteMultipartUpload, PutSymlink）
    if !event_item.event_name.starts_with("ObjectCreated") {
        return Ok(ResponseJson(ProcessResponse {
            success: false,
            message: format!("不支持的事件类型: {}", event_item.event_name),
            result: None,
        }));
    }

    let bucket = &event_item.oss.bucket.name;
    let mut object_key = event_item.oss.object.key.clone();
    let region = &event_item.region;
    
    // 处理符号链接事件（参考 Python 示例）
    if event_item.event_name == "ObjectCreated:PutSymlink" {
        // TODO: 实现符号链接解析
        // 在函数计算环境中，符号链接需要通过 OSS API 解析
        // 当前先记录日志，后续可以实现 get_symlink 功能
        info!("检测到符号链接事件，需要解析符号链接: {}", object_key);
        // 注意：符号链接解析需要 OSS SDK 支持，当前暂不处理
        return Ok(ResponseJson(ProcessResponse {
            success: false,
            message: format!("暂不支持符号链接事件: {}", event_item.event_name),
            result: None,
        }));
    }
    
    info!("处理视频文件: bucket={}, key={}, region={}", bucket, object_key, region);

    // 创建临时目录
    // 尝试使用函数计算的 request_id（如果可用），否则使用时间戳+UUID
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("创建临时目录失败: {}", e)))?;

    // 下载视频文件
    let oss_client = OssClient::new()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("创建 OSS 客户端失败: {}", e)))?;
    
    let video_filename = PathBuf::from(&object_key)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4");
    
    let video_path = temp_dir.join(video_filename);
    
    // 构建 internal endpoint（内网访问更快且免费）
    // 格式: oss-{region}-internal.aliyuncs.com
    let endpoint = format!("oss-{}-internal.aliyuncs.com", region);
    
    // 尝试从 OSS 下载（使用 internal endpoint）
    let downloaded_path = oss_client
        .download_file(bucket, &object_key, Some(&endpoint), &video_path)
        .await
        .map_err(|e| {
            error!("下载文件失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("下载文件失败: {}", e))
        })?;

    // 创建输出目录
    let output_dir = temp_dir.join("output");
    
    // 处理视频
    let config = ProcessConfig::default();
    let process_result = process_video(&downloaded_path, &output_dir, config)
        .await
        .map_err(|e| {
            error!("处理视频失败: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("处理视频失败: {}", e))
        })?;

    // 上传处理结果到目标 bucket（如果配置了目标 bucket）
    let uploaded_files = if let (Some(dest_bucket), Some(dest_region)) = (
        std::env::var("DESTINATION_BUCKET").ok(),
        std::env::var("DESTINATION_REGION").ok(),
    ) {
        info!("开始上传处理结果到目标 bucket: {}/{}", dest_region, dest_bucket);
        
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
        if !upload_errors.is_empty() {
            error!("部分文件上传失败，共 {} 个错误", upload_errors.len());
            for err in &upload_errors {
                error!("  - {}", err);
            }
        }
        
        info!("上传完成: 成功 {} 个，失败 {} 个", uploaded.len(), upload_errors.len());
        
        Some(uploaded)
    } else {
        info!("未配置目标 bucket，跳过上传");
        None
    };

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
            keyframes: process_result.keyframe_files,
            audio_file: process_result.audio_file,
            metadata_file: "metadata.json".to_string(),
        }),
    };

    info!("处理完成: {:?}", response);

    Ok(ResponseJson(response))
}

/// 健康检查 Handler
pub async fn health_check() -> &'static str {
    "OK"
}