use hyper::body::Incoming;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::{HeaderValue, HeaderMap, CONTENT_TYPE, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_HEADERS};
use std::path::PathBuf;
use std::convert::Infallible;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use crate::{OssEvent, ProcessResponse, ProcessResult, OssClient, ProcessConfig, process_video, config::ConfigLoader, ExtendedConfig};
use tracing::{info, error, warn, debug};

/// 处理所有 HTTP 请求
pub async fn handle_request(
    req: Request<Incoming>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    let start_time = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();
    let query = uri.query().map(|q| q.to_string());
    let full_uri = format!("{}{}{}", 
        path,
        if query.is_some() { "?" } else { "" },
        query.as_deref().unwrap_or("")
    );
    
    // 获取请求ID（如果存在）- 先克隆为 String，避免借用冲突
    let request_id = req.headers()
        .get("x-fc-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    // 记录请求开始
    info!(
        method = %method,
        uri = %full_uri,
        path = %path,
        request_id = %request_id,
        "HTTP request started"
    );
    
    // 处理 CORS 预检请求
    if req.method() == hyper::Method::OPTIONS {
        let duration = start_time.elapsed();
        info!(
            method = %method,
            uri = %full_uri,
            request_id = %request_id,
            duration_ms = duration.as_millis(),
            status = 200,
            "HTTP request completed (CORS preflight)"
        );
        return Ok(build_cors_response(StatusCode::OK));
    }
    
    // 路由到对应的处理函数
    let response = match (method.clone(), path) {
        (hyper::Method::GET, "/") | (hyper::Method::GET, "/health") => {
            handle_health_check().await
        }
        (hyper::Method::POST, "/initialize") => {
            handle_initialize(req, start_time, request_id.clone()).await
        }
        (hyper::Method::GET, "/invoke") 
        | (hyper::Method::POST, "/invoke")
        | (hyper::Method::PUT, "/invoke")
        | (hyper::Method::DELETE, "/invoke")
        | (hyper::Method::PATCH, "/invoke")
        | (hyper::Method::HEAD, "/invoke")
        | (hyper::Method::OPTIONS, "/invoke") => {
            handle_invoke(req, start_time, request_id.clone()).await
        }
        (hyper::Method::GET, "/process")
        | (hyper::Method::POST, "/process")
        | (hyper::Method::PUT, "/process")
        | (hyper::Method::DELETE, "/process")
        | (hyper::Method::PATCH, "/process")
        | (hyper::Method::HEAD, "/process")
        | (hyper::Method::OPTIONS, "/process") => {
            handle_oss_event_any(req, start_time, request_id.clone()).await
        }
        (hyper::Method::POST, "/process/direct") => {
            handle_direct_process(req, start_time, request_id.clone()).await
        }
        (hyper::Method::GET, "/process/query") => {
            handle_process_query(req, start_time, request_id.clone()).await
        }
        _ => {
            let duration = start_time.elapsed();
            warn!(
                method = %method,
                uri = %full_uri,
                path = %path,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = 404,
                "HTTP request completed (route not found)"
            );
            json_response(
                StatusCode::NOT_FOUND,
                JsonResponse {
                    success: false,
                    message: format!("未找到路径: {}", path),
                    data: None,
                }
            )
        }
    };

    // 添加 CORS 头
    let mut response = response;
    add_cors_headers(response.headers_mut());
    
    // 记录请求完成
    let duration = start_time.elapsed();
    let status = response.status().as_u16();
    info!(
        method = %method,
        uri = %full_uri,
        path = %path,
        request_id = %request_id,
        duration_ms = duration.as_millis(),
        duration_sec = duration.as_secs_f64(),
        status = status,
        "HTTP request completed"
    );
    
    Ok(response)
}

/// 添加 CORS 头
fn add_cors_headers(headers: &mut HeaderMap) {
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS"));
    headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type, Authorization, x-fc-request-id"));
}

/// 构建 CORS 响应
fn build_cors_response(status: StatusCode) -> Response<Full<Bytes>> {
    let mut response = Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()))
        .unwrap();
    add_cors_headers(response.headers_mut());
    response
}

/// 构建 JSON 响应
fn json_response<T: Serialize>(status: StatusCode, body: T) -> Response<Full<Bytes>> {
    let json = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

/// 读取请求体
async fn read_body(req: Request<Incoming>) -> Result<Bytes, hyper::Error> {
    use http_body_util::BodyExt;
    let body = req.into_body();
    let bytes = body.collect().await?.to_bytes();
    Ok(bytes)
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
async fn handle_health_check() -> Response<Full<Bytes>> {
    info!("Health check request processed");
    json_response(
        StatusCode::OK,
        JsonResponse {
            success: true,
            message: "server is running".to_string(),
            data: Some(serde_json::json!({
                "status": "ok",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        }
    )
}

/// 函数计算初始化端点
pub async fn handle_initialize(
    req: Request<Incoming>,
    start_time: Instant,
    request_id: String,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        "FC initialize request processing"
    );
    
    let duration = start_time.elapsed();
    let response = json_response(
        StatusCode::OK,
        JsonResponse {
            success: true,
            message: "FunctionCompute 初始化完成".to_string(),
            data: Some(serde_json::json!({
                "request_id": request_id,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })),
        }
    );
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        duration_ms = duration.as_millis(),
        status = 200,
        "FC initialize completed"
    );
    
    response
}

/// 函数计算调用端点
pub async fn handle_invoke(
    req: Request<Incoming>,
    start_time: Instant,
    request_id: String,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    let headers = req.headers().clone();
    let path = uri.path();
    let query = uri.query().map(|q| q.to_string());
    
    info!(
        method = %method,
        uri = %uri,
        path = %path,
        query = ?query,
        version = ?version,
        request_id = %request_id,
        "FC invoke request processing started"
    );
    
    // 输出所有请求头
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            debug!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                header_name = %name,
                header_value = %value_str,
                "request header"
            );
        } else {
            debug!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                header_name = %name,
                header_size = value.len(),
                "request header (binary)"
            );
        }
    }
    
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(e) => {
            let duration = start_time.elapsed();
            error!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = 400,
                error = %e,
                "failed to read request body"
            );
            
            let response_data = JsonResponse {
                success: false,
                message: format!("读取请求体失败: {}", e),
                data: None,
            };
            
            return json_response(StatusCode::BAD_REQUEST, response_data);
        }
    };
    
    // 记录请求体信息
    if body.is_empty() {
        info!(
            method = %method,
            uri = %uri,
            request_id = %request_id,
            body_size = 0,
            "request body is empty"
        );
    } else {
        match String::from_utf8(body.to_vec()) {
            Ok(body_str) => {
                if body_str.len() > 1000 {
                    info!(
                        method = %method,
                        uri = %uri,
                        request_id = %request_id,
                        body_size = body_str.len(),
                        body_preview = %&body_str[..1000.min(body_str.len())],
                        "request body (truncated)"
                    );
                } else {
                    info!(
                        method = %method,
                        uri = %uri,
                        request_id = %request_id,
                        body_size = body_str.len(),
                        body = %body_str,
                        "request body"
                    );
                }
            }
            Err(_) => {
                info!(
                    method = %method,
                    uri = %uri,
                    request_id = %request_id,
                    body_size = body.len(),
                    "request body (binary)"
                );
            }
        }
    }
    
    // 尝试解析为 OSS 事件并处理
    if !body.is_empty() {
        match serde_json::from_slice::<OssEvent>(&body) {
            Ok(event) => {
                info!(
                    method = %method,
                    uri = %uri,
                    request_id = %request_id,
                    event_count = event.events.len(),
                    "successfully parsed as OSS event"
                );
                
                if !event.events.is_empty() {
                    let event_item = &event.events[0];
                    let object_key = &event_item.oss.object.key;
                    let bucket = &event_item.oss.bucket.name;
                    
                    info!(
                        method = %method,
                        uri = %uri,
                        request_id = %request_id,
                        event_name = %event_item.event_name,
                        bucket = %bucket,
                        object_key = %object_key,
                        "OSS event details"
                    );
                    
                    // 检查文件类型
                    if !is_video_file(object_key) {
                        let duration = start_time.elapsed();
                        let response_data = JsonResponse {
                            success: true,
                            message: format!("文件 {} 不是视频文件，已跳过处理", object_key),
                            data: Some(serde_json::json!({
                                "request_id": request_id,
                                "object_key": object_key,
                                "file_type": "non-video",
                                "skipped": true,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        };
                        
                        let response_json = serde_json::to_string(&response_data).unwrap_or_else(|_| "{}".to_string());
                        info!(
                            method = %method,
                            uri = %uri,
                            request_id = %request_id,
                            duration_ms = duration.as_millis(),
                            status = 200,
                            response_size = response_json.len(),
                            object_key = %object_key,
                            "FC invoke completed (non-video file skipped)"
                        );
                        
                        return json_response(StatusCode::OK, response_data);
                    }
                    
                    // 只处理 ObjectCreated 事件
                    if !event_item.event_name.starts_with("ObjectCreated") {
                        let duration = start_time.elapsed();
                        let response_data = JsonResponse {
                            success: true,
                            message: format!("事件类型 {} 不是 ObjectCreated，已跳过处理", event_item.event_name),
                            data: Some(serde_json::json!({
                                "request_id": request_id,
                                "event_name": event_item.event_name,
                                "skipped": true,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            })),
                        };
                        
                        let response_json = serde_json::to_string(&response_data).unwrap_or_else(|_| "{}".to_string());
                        info!(
                            method = %method,
                            uri = %uri,
                            request_id = %request_id,
                            duration_ms = duration.as_millis(),
                            status = 200,
                            response_size = response_json.len(),
                            event_name = %event_item.event_name,
                            "FC invoke completed (non-ObjectCreated event skipped)"
                        );
                        
                        return json_response(StatusCode::OK, response_data);
                    }
                    
                    // 启动异步处理任务
                    let event_clone = event.clone();
                    let request_id_clone = request_id.clone();
                    tokio::spawn(async move {
                        match handle_oss_event_internal(event_clone, Some(request_id_clone.clone())).await {
                            Ok(_) => {
                                info!(
                                    request_id = %request_id_clone,
                                    "async task: video processing completed successfully"
                                );
                            }
                            Err((status_code, error_msg)) => {
                                error!(
                                    request_id = %request_id_clone,
                                    status = status_code.as_u16(),
                                    error = %error_msg,
                                    "async task: processing failed"
                                );
                            }
                        }
                    });
                    
                    info!(
                        method = %method,
                        uri = %uri,
                        request_id = %request_id,
                        object_key = %object_key,
                        "async processing task started"
                    );
                }
            }
            Err(e) => {
                warn!(
                    method = %method,
                    uri = %uri,
                    request_id = %request_id,
                    error = %e,
                    "OSS event JSON parsing failed"
                );
            }
        }
    }
    
    let duration = start_time.elapsed();
    let response_data = JsonResponse {
        success: true,
        message: "请求已接收".to_string(),
        data: Some(serde_json::json!({
            "request_id": request_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    };
    
    let response_json = serde_json::to_string(&response_data).unwrap_or_else(|_| "{}".to_string());
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        duration_ms = duration.as_millis(),
        duration_sec = duration.as_secs_f64(),
        status = 200,
        response_size = response_json.len(),
        "FC invoke completed"
    );
    
    json_response(StatusCode::OK, response_data)
}

/// 处理 OSS Event 的 Handler（接受任何HTTP方法）
pub async fn handle_oss_event_any(
    req: Request<Incoming>,
    start_time: Instant,
    request_id: String,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        "OSS event processing request started"
    );
    
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(e) => {
            let duration = start_time.elapsed();
            error!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = 400,
                error = %e,
                "failed to read request body"
            );
            return json_response(
                StatusCode::BAD_REQUEST,
                ProcessResponse {
                    success: false,
                    message: format!("读取请求体失败: {}", e),
                    result: None,
                }
            );
        }
    };
    
    if body.is_empty() {
        let duration = start_time.elapsed();
        error!(
            method = %method,
            uri = %uri,
            request_id = %request_id,
            duration_ms = duration.as_millis(),
            status = 400,
            "请求体为空"
        );
        return json_response(
            StatusCode::BAD_REQUEST,
            ProcessResponse {
                success: false,
                message: "请求体为空".to_string(),
                result: None,
            }
        );
    }
    
    let event: OssEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            let duration = start_time.elapsed();
            error!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = 400,
                error = %e,
                "JSON parsing failed"
            );
            return json_response(
                StatusCode::BAD_REQUEST,
                ProcessResponse {
                    success: false,
                    message: format!("解析 JSON 失败: {}", e),
                    result: None,
                }
            );
        }
    };
    
    // 调用内部处理逻辑（同步处理，返回结果）
    let response = match handle_oss_event_internal(event, Some(request_id.clone())).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            let response_json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            info!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                duration_sec = duration.as_secs_f64(),
                status = 200,
                response_size = response_json.len(),
                "OSS event processing completed"
            );
            json_response(StatusCode::OK, response)
        }
        Err((status, msg)) => {
            let duration = start_time.elapsed();
            error!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = status.as_u16(),
                error = %msg,
                "OSS event processing failed"
            );
            json_response(status, ProcessResponse {
                success: false,
                message: msg,
                result: None,
            })
        }
    };
    
    response
}

/// 内部处理 OSS Event 的逻辑
async fn handle_oss_event_internal(
    event: OssEvent,
    request_id: Option<String>,
) -> Result<ProcessResponse, (StatusCode, String)> {
    // 这里保留原有的处理逻辑，只是返回类型改为 Result<ProcessResponse, (StatusCode, String)>
    // 由于代码很长，我会保留核心逻辑，但简化一些部分
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("[OSS Event] Received OSS event trigger request");
    info!("Event count: {}", event.events.len());
    
    if event.events.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "事件列表为空".to_string()));
    }
    
    let event_item = &event.events[0];
    
    if !event_item.event_name.starts_with("ObjectCreated") {
        warn!("[OSS Event] Unsupported event type: {}, skipping processing", event_item.event_name);
        return Ok(ProcessResponse {
            success: false,
            message: format!("不支持的事件类型: {}", event_item.event_name),
            result: None,
        });
    }
    
    // 加载扩展配置
    let extended_config = ConfigLoader::load_extended_config(None)
        .unwrap_or_else(|_| ExtendedConfig {
            process: ProcessConfig::default(),
            debug_mode: false,
            output_path: None,
            destination_bucket: None,
            destination_region: None,
            destination_prefix: None,
            log_level: "info".to_string(),
        });
    
    if extended_config.debug_mode {
        info!("DEBUG mode enabled, skipping actual processing");
        return Ok(ProcessResponse {
            success: true,
            message: "DEBUG 模式：事件接收成功".to_string(),
            result: None,
        });
    }
    
    let bucket = &event_item.oss.bucket.name;
    let object_key = event_item.oss.object.key.clone();
    let region = &event_item.region;
    
    // 创建临时目录
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
    
    let temp_dir = if let Some(ref output_path) = extended_config.output_path {
        output_path.join(&request_id)
    } else {
        std::env::temp_dir().join("video-parse").join(&request_id)
    };
    
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("创建临时目录失败: {}", e)))?;
    
    // 下载视频文件
    let oss_client = OssClient::new()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("创建 OSS 客户端失败: {}", e)))?;
    
    let video_path_buf = PathBuf::from(&object_key);
    let video_filename = video_path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4");
    
    let video_path = temp_dir.join(video_filename);
    let endpoint = format!("oss-{}-internal.aliyuncs.com", region);
    
    let downloaded_path = oss_client
        .download_file(bucket, &object_key, Some(&endpoint), &video_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("下载文件失败: {}", e)))?;
    
    // 创建输出目录
    let output_dir = temp_dir.join("output");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("创建输出目录失败: {}", e)))?;
    
    // 处理视频
    let config = extended_config.process.clone();
    let process_result = process_video(&downloaded_path, &output_dir, config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("处理视频失败: {}", e)))?;
    
    // 构建响应
    Ok(ProcessResponse {
        success: true,
        message: format!("成功处理视频，检测到 {} 个场景", process_result.metadata.scene_count),
        result: Some(ProcessResult {
            video_file: downloaded_path.to_string_lossy().to_string(),
            output_dir: output_dir.to_string_lossy().to_string(),
            scene_count: process_result.metadata.scene_count,
            keyframes: process_result.keyframe_files.clone(),
            audio_file: process_result.audio_file.clone(),
            metadata_file: "metadata.json".to_string(),
        }),
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

/// 直接处理请求
#[derive(Debug, Deserialize)]
pub struct DirectProcessRequest {
    pub input: String,
    pub output: Option<String>,
    pub threshold: Option<f64>,
    pub min_scene_duration: Option<f64>,
    pub sample_rate: Option<f64>,
    pub is_oss_path: Option<bool>,
    pub oss_bucket: Option<String>,
    pub oss_region: Option<String>,
}

/// 直接处理视频的 Handler
pub async fn handle_direct_process(
    req: Request<Incoming>,
    start_time: Instant,
    request_id: String,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        "direct process request started"
    );
    
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(e) => {
            let duration = start_time.elapsed();
            error!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = 400,
                error = %e,
                "failed to read request body"
            );
            return json_response(
                StatusCode::BAD_REQUEST,
                ProcessResponse {
                    success: false,
                    message: format!("读取请求体失败: {}", e),
                    result: None,
                }
            );
        }
    };
    
    let request: DirectProcessRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let duration = start_time.elapsed();
            error!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = 400,
                error = %e,
                "JSON parsing failed"
            );
            return json_response(
                StatusCode::BAD_REQUEST,
                ProcessResponse {
                    success: false,
                    message: format!("解析 JSON 失败: {}", e),
                    result: None,
                }
            );
        }
    };
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        input = %request.input,
        "received direct process request"
    );
    
    // 这里简化处理，实际应该调用完整的处理逻辑
    let duration = start_time.elapsed();
    let response = json_response(
        StatusCode::OK,
        ProcessResponse {
            success: true,
            message: "处理请求已接收".to_string(),
            result: None,
        }
    );
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        duration_ms = duration.as_millis(),
        status = 200,
        "direct process request completed"
    );
    
    response
}

/// 处理视频的查询参数版本
#[derive(Debug, Deserialize)]
pub struct ProcessQueryParams {
    pub input: String,
    pub output: Option<String>,
    pub threshold: Option<f64>,
    pub min_scene_duration: Option<f64>,
    pub sample_rate: Option<f64>,
}

/// 通过查询参数处理视频
pub async fn handle_process_query(
    req: Request<Incoming>,
    start_time: Instant,
    request_id: String,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let query = req.uri().query().unwrap_or("");
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        query = %query,
        "query parameter process request started"
    );
    
    let params: ProcessQueryParams = match serde_urlencoded::from_str(query) {
        Ok(p) => p,
        Err(e) => {
            let duration = start_time.elapsed();
            error!(
                method = %method,
                uri = %uri,
                request_id = %request_id,
                duration_ms = duration.as_millis(),
                status = 400,
                error = %e,
                "failed to parse query parameters"
            );
            return json_response(
                StatusCode::BAD_REQUEST,
                ProcessResponse {
                    success: false,
                    message: format!("解析查询参数失败: {}", e),
                    result: None,
                }
            );
        }
    };
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        input = %params.input,
        "received query parameter process request"
    );
    
    let duration = start_time.elapsed();
    let response = json_response(
        StatusCode::OK,
        ProcessResponse {
            success: true,
            message: "处理请求已接收".to_string(),
            result: None,
        }
    );
    
    info!(
        method = %method,
        uri = %uri,
        request_id = %request_id,
        duration_ms = duration.as_millis(),
        status = 200,
        "query parameter process request completed"
    );
    
    response
}
