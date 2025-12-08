use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
use image::DynamicImage;
use crate::{VideoProcessor, SceneDetector, AudioExtractor, metadata::VideoMetadata};

/// 视频处理配置
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// 场景变化检测阈值
    pub threshold: f64,
    /// 最小场景持续时间（秒）
    pub min_scene_duration: f64,
    /// 帧采样率（每秒采样多少帧）
    pub sample_rate: f64,
    /// Webhook URL（处理完成后回调）
    pub webhook_url: Option<String>,
}

impl ProcessConfig {
    /// 从环境变量和配置文件加载配置
    pub fn from_env_and_file(config_file: Option<&std::path::Path>) -> anyhow::Result<Self> {
        use crate::config::ConfigLoader;
        ConfigLoader::load_config(config_file, None, None, None, None)
    }
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            threshold: 0.35,
            min_scene_duration: 0.8,
            sample_rate: 0.5,
            webhook_url: None,
        }
    }
}

/// 处理结果
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    /// 输出目录
    pub output_dir: PathBuf,
    /// 元数据
    pub metadata: VideoMetadata,
    /// 关键帧文件列表
    pub keyframe_files: Vec<String>,
    /// 音频文件
    pub audio_file: String,
}

/// 处理视频文件
pub async fn process_video(
    input_video_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    config: ProcessConfig,
) -> Result<ProcessOutput> {
    let input_video_path = input_video_path.as_ref();
    let output_dir = output_dir.as_ref();

    let total_start = Instant::now();
    println!("开始处理视频: {}", input_video_path.display());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 创建输出目录
    let dir_start = Instant::now();
    std::fs::create_dir_all(output_dir)
        .context("创建输出目录失败")?;
    println!("[{}ms] ✓ 创建输出目录", dir_start.elapsed().as_millis());

    // 1. 初始化视频处理器
    let init_start = Instant::now();
    let processor = VideoProcessor::new(input_video_path)?;
    println!("[{}ms] ✓ 初始化视频处理器", init_start.elapsed().as_millis());
    
    // 2. 获取视频信息
    let info_start = Instant::now();
    let (fps, width, height) = processor.get_video_info()?;
    println!("[{}ms] ✓ 获取视频信息: {}x{}, {:.2} fps", 
        info_start.elapsed().as_millis(), width, height, fps);

    // 3. 提取视频帧
    let extract_start = Instant::now();
    println!("⏳ 正在提取视频帧（采样率: {:.1} fps）...", config.sample_rate);
    let frames = processor.extract_frames(Some(config.sample_rate))?;
    let extract_duration = extract_start.elapsed();
    println!("[{}ms] ✓ 提取视频帧完成: {} 帧 (平均 {:.2}ms/帧)", 
        extract_duration.as_millis(), 
        frames.len(),
        if frames.len() > 0 { extract_duration.as_millis() as f64 / frames.len() as f64 } else { 0.0 });

    // 4. 检测场景变化
    let scene_start = Instant::now();
    println!("⏳ 正在检测场景变化...");
    let detector = SceneDetector::new(config.threshold, config.min_scene_duration);
    let scene_changes = detector.detect_scenes(&frames, fps)?;
    let scene_duration = scene_start.elapsed();
    println!("[{}ms] ✓ 场景检测完成: {} 个场景 (平均 {:.2}ms/场景)", 
        scene_duration.as_millis(),
        scene_changes.len(),
        if scene_changes.len() > 0 { scene_duration.as_millis() as f64 / scene_changes.len() as f64 } else { 0.0 });

    // 5. 提取关键帧并保存
    let keyframe_start = Instant::now();
    println!("⏳ 正在提取并保存关键帧...");
    let mut scenes_metadata = Vec::new();
    let mut keyframe_files = Vec::new();
    let total_duration = frames.last().map(|(t, _)| *t).unwrap_or(0.0);
    
    // 检查是否有提取的帧
    if frames.is_empty() {
        anyhow::bail!("没有提取到任何视频帧，无法提取关键帧");
    }
    
    // 创建场景检测器用于计算帧差异
    let detector = SceneDetector::new(config.threshold, config.min_scene_duration);
    let mut keyframe_counter = 0;
    
    for (i, &scene_start) in scene_changes.iter().enumerate() {
        // 确定场景结束时间
        let scene_end = if i + 1 < scene_changes.len() {
            scene_changes[i + 1]
        } else {
            total_duration
        };
        
        let duration = scene_end - scene_start;
        
        // 找到属于当前场景的所有帧
        let scene_frames: Vec<(usize, &(f64, DynamicImage))> = frames.iter()
            .enumerate()
            .filter(|(_, (t, _))| *t >= scene_start && *t < scene_end)
            .collect();
        
        if scene_frames.is_empty() {
            // 如果没有找到帧，使用场景开始时间附近的帧
            let fallback_idx = frames.iter()
                .enumerate()
                .min_by(|(_, (t1, _)), (_, (t2, _))| {
                    ((*t1 - scene_start).abs()).partial_cmp(&((*t2 - scene_start).abs())).unwrap()
                })
                .map(|(idx, _)| idx);
            
            // 如果找不到回退帧，跳过这个场景
            let fallback_idx = match fallback_idx {
                Some(idx) => idx,
                None => {
                    println!("⚠️  场景 {}: 没有找到合适的帧，跳过", i);
                    continue;
                }
            };
            
            let (_keyframe_time, keyframe_img) = &frames[fallback_idx];
            let keyframe_filename = format!("keyframe_{:04}.jpg", keyframe_counter);
            let keyframe_path = output_dir.join(&keyframe_filename);
            keyframe_img.save(&keyframe_path)
                .context(format!("保存关键帧失败: {}", keyframe_filename))?;
            
            keyframe_files.push(keyframe_filename.clone());
            scenes_metadata.push(crate::metadata::SceneMetadata {
                scene_id: i,
                keyframe_file: keyframe_filename,
                start_time: scene_start,
                end_time: scene_end,
                duration,
            });
            keyframe_counter += 1;
            continue;
        }
        
        // 每个场景只提取1个关键帧
        // 策略：在场景中间区域（30%-70%）选择最稳定的帧（与相邻帧差异最小）
        let scene_mid_start = scene_start + duration * 0.3;
        let scene_mid_end = scene_start + duration * 0.7;
        
        // 找到中间区域的帧
        let mid_region_frames: Vec<(usize, &(f64, DynamicImage))> = scene_frames.iter()
            .filter(|(_, (t, _))| *t >= scene_mid_start && *t <= scene_mid_end)
            .cloned()
            .collect();
        
        let keyframe_idx = if mid_region_frames.is_empty() {
            // 如果中间区域没有帧，选择场景中间位置的帧
            let target_time = scene_start + duration * 0.5;
            scene_frames.iter()
                .min_by(|(_, (t1, _)), (_, (t2, _))| {
                    ((*t1 - target_time).abs()).partial_cmp(&((*t2 - target_time).abs())).unwrap()
                })
                .map(|(idx, _)| *idx)
                .unwrap_or_else(|| {
                    // 如果找不到最接近的帧，使用第一个帧
                    scene_frames.first()
                        .map(|(idx, _)| *idx)
                        .unwrap_or_else(|| {
                            // 如果 scene_frames 也为空（理论上不应该发生），使用第一个全局帧
                            println!("⚠️  场景 {}: 没有找到合适的帧，使用第一个全局帧", i);
                            0
                        })
                })
        } else if mid_region_frames.len() == 1 {
            // 如果只有一个帧，直接使用
            mid_region_frames[0].0
        } else {
            // 在中间区域选择最稳定的帧（与前后帧差异最小）
            let mut best_idx = mid_region_frames[0].0;
            let mut min_avg_diff = f64::MAX;
            
            for (frame_idx, (_, _)) in mid_region_frames.iter() {
                let frame_idx_in_all = *frame_idx;
                
                // 计算与前后帧的平均差异
                let mut diffs = Vec::new();
                
                // 与前一个帧的差异
                if frame_idx_in_all > 0 {
                    let prev_frame = &frames[frame_idx_in_all - 1];
                    if prev_frame.0 >= scene_start {
                        let diff = detector.calculate_frame_difference(
                            &prev_frame.1,
                            &frames[frame_idx_in_all].1
                        );
                        diffs.push(diff);
                    }
                }
                
                // 与后一个帧的差异
                if frame_idx_in_all + 1 < frames.len() {
                    let next_frame = &frames[frame_idx_in_all + 1];
                    if next_frame.0 < scene_end {
                        let diff = detector.calculate_frame_difference(
                            &frames[frame_idx_in_all].1,
                            &next_frame.1
                        );
                        diffs.push(diff);
                    }
                }
                
                // 计算平均差异
                let avg_diff = if diffs.is_empty() {
                    f64::MAX
                } else {
                    diffs.iter().sum::<f64>() / diffs.len() as f64
                };
                
                // 选择差异最小的帧（最稳定的帧）
                if avg_diff < min_avg_diff {
                    min_avg_diff = avg_diff;
                    best_idx = frame_idx_in_all;
                }
            }
            
            best_idx
        };
        
        let (_keyframe_time, keyframe_img) = &frames[keyframe_idx];
        
        // 保存关键帧图片
        let keyframe_filename = format!("keyframe_{:04}.jpg", keyframe_counter);
        let keyframe_path = output_dir.join(&keyframe_filename);
        keyframe_img.save(&keyframe_path)
            .context(format!("保存关键帧失败: {}", keyframe_filename))?;
        
        keyframe_files.push(keyframe_filename.clone());
        
        // 场景元数据
        scenes_metadata.push(crate::metadata::SceneMetadata {
            scene_id: i,
            keyframe_file: keyframe_filename,
            start_time: scene_start,
            end_time: scene_end,
            duration,
        });
        
        keyframe_counter += 1;
    }
    let keyframe_duration = keyframe_start.elapsed();
    println!("[{}ms] ✓ 关键帧提取完成: {} 个关键帧 (平均 {:.2}ms/帧)", 
        keyframe_duration.as_millis(),
        keyframe_files.len(),
        if keyframe_files.len() > 0 { keyframe_duration.as_millis() as f64 / keyframe_files.len() as f64 } else { 0.0 });

    // 6. 提取音频
    let audio_start = Instant::now();
    println!("⏳ 正在提取音频...");
    let audio_filename = "audio.aac";
    let audio_path = output_dir.join(&audio_filename);
    let audio_extractor = AudioExtractor::new(input_video_path)?;
    audio_extractor.extract_to_file(&audio_path)?;
    let audio_duration = audio_start.elapsed();
    println!("[{}ms] ✓ 音频提取完成: {}", audio_duration.as_millis(), audio_path.display());

    // 7. 生成元数据 JSON
    let metadata_start = Instant::now();
    println!("⏳ 正在生成元数据...");
    let metadata = VideoMetadata {
        input_video: input_video_path.to_string_lossy().to_string(),
        total_duration,
        fps,
        resolution: format!("{}x{}", width, height),
        scene_count: scenes_metadata.len(),
        audio_file: audio_filename.to_string(),
        scenes: scenes_metadata,
    };
    
    let metadata_path = output_dir.join("metadata.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .context("序列化元数据失败")?;
    std::fs::write(&metadata_path, metadata_json)
        .context("写入元数据文件失败")?;
    let metadata_duration = metadata_start.elapsed();
    println!("[{}ms] ✓ 元数据生成完成: {}", metadata_duration.as_millis(), metadata_path.display());
    
    // 总结
    let total_duration = total_start.elapsed();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 处理完成！总耗时: {:.2}秒 ({:.0}ms)", 
        total_duration.as_secs_f64(), 
        total_duration.as_millis());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 性能统计:");
    println!("   • 视频帧提取: {:.2}秒 ({:.1}%)", 
        extract_duration.as_secs_f64(),
        extract_duration.as_secs_f64() / total_duration.as_secs_f64() * 100.0);
    println!("   • 场景检测: {:.2}秒 ({:.1}%)", 
        scene_duration.as_secs_f64(),
        scene_duration.as_secs_f64() / total_duration.as_secs_f64() * 100.0);
    println!("   • 关键帧提取: {:.2}秒 ({:.1}%)", 
        keyframe_duration.as_secs_f64(),
        keyframe_duration.as_secs_f64() / total_duration.as_secs_f64() * 100.0);
    println!("   • 音频提取: {:.2}秒 ({:.1}%)", 
        audio_duration.as_secs_f64(),
        audio_duration.as_secs_f64() / total_duration.as_secs_f64() * 100.0);
    println!("   • 元数据生成: {:.2}秒 ({:.1}%)", 
        metadata_duration.as_secs_f64(),
        metadata_duration.as_secs_f64() / total_duration.as_secs_f64() * 100.0);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📁 输出目录: {}", output_dir.display());
    println!("📸 关键帧数量: {}", metadata.scene_count);
    println!("🎵 音频文件: {}", audio_filename);

    let result = ProcessOutput {
        output_dir: output_dir.to_path_buf(),
        metadata: metadata.clone(),
        keyframe_files: keyframe_files.clone(),
        audio_file: audio_filename.to_string(),
    };

    // 调用 webhook 回调（如果配置了）
    if let Some(webhook_url) = &config.webhook_url {
        if let Err(e) = call_webhook(webhook_url, &result, &metadata).await {
            tracing::warn!("Webhook 回调失败: {}", e);
        } else {
            println!("✓ Webhook 回调成功");
        }
    }

    Ok(result)
}

/// Webhook 回调数据结构
#[derive(Debug, serde::Serialize)]
struct WebhookPayload {
    /// 处理状态
    status: String,
    /// 输入视频路径
    input_video: String,
    /// 输出目录
    output_dir: String,
    /// 场景数量
    scene_count: usize,
    /// 关键帧数量
    keyframe_count: usize,
    /// 音频文件
    audio_file: String,
    /// 视频元数据
    metadata: VideoMetadata,
    /// 处理时间戳
    timestamp: String,
}

/// 调用 webhook 回调
async fn call_webhook(
    webhook_url: &str,
    result: &ProcessOutput,
    metadata: &VideoMetadata,
) -> Result<()> {
    use chrono::Utc;

    let timestamp = Utc::now().to_rfc3339();

    let payload = WebhookPayload {
        status: "success".to_string(),
        input_video: metadata.input_video.clone(),
        output_dir: result.output_dir.to_string_lossy().to_string(),
        scene_count: metadata.scene_count,
        keyframe_count: result.keyframe_files.len(),
        audio_file: result.audio_file.clone(),
        metadata: metadata.clone(),
        timestamp,
    };

    let client = reqwest::Client::new();
    let response = client
        .post(webhook_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .context("Webhook 请求失败")?;

    let status = response.status();
    if status.is_success() {
        tracing::info!("Webhook 回调成功: {}", webhook_url);
    } else {
        let error_text = response.text().await.unwrap_or_default();
        tracing::warn!(
            "Webhook 回调返回错误状态: {} - {}",
            status,
            error_text
        );
        return Err(anyhow::anyhow!(
            "Webhook 返回错误状态: {}",
            status
        ));
    }

    Ok(())
}