use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
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
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            threshold: 0.3,
            min_scene_duration: 1.0,
            sample_rate: 2.0,
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

    println!("开始处理视频: {}", input_video_path.display());
    
    // 创建输出目录
    std::fs::create_dir_all(output_dir)
        .context("创建输出目录失败")?;

    // 1. 初始化视频处理器
    let processor = VideoProcessor::new(input_video_path)?;
    
    // 2. 获取视频信息
    let (fps, width, height) = processor.get_video_info()?;
    println!("视频信息: {}x{}, {:.2} fps", width, height, fps);

    // 3. 提取视频帧
    println!("正在提取视频帧...");
    let frames = processor.extract_frames(Some(config.sample_rate))?;
    println!("提取了 {} 帧", frames.len());

    // 4. 检测场景变化
    println!("正在检测场景变化...");
    let detector = SceneDetector::new(config.threshold, config.min_scene_duration);
    let scene_changes = detector.detect_scenes(&frames, fps)?;
    println!("检测到 {} 个场景", scene_changes.len());

    // 5. 提取关键帧并保存
    println!("正在保存关键帧...");
    let mut scenes_metadata = Vec::new();
    let mut keyframe_files = Vec::new();
    let total_duration = frames.last().map(|(t, _)| *t).unwrap_or(0.0);
    
    for (i, &scene_start) in scene_changes.iter().enumerate() {
        // 找到场景开始时间对应的帧
        let keyframe_idx = frames.iter()
            .position(|(t, _)| (*t - scene_start).abs() < 0.1)
            .unwrap_or(0);
        
        let (keyframe_time, keyframe_img) = &frames[keyframe_idx];
        
        // 确定场景结束时间
        let scene_end = if i + 1 < scene_changes.len() {
            scene_changes[i + 1]
        } else {
            total_duration
        };
        
        let duration = scene_end - scene_start;
        
        // 保存关键帧图片
        let keyframe_filename = format!("keyframe_{:04}.jpg", i);
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
    }

    // 6. 提取音频
    println!("正在提取音频...");
    let audio_filename = "audio.aac";
    let audio_path = output_dir.join(&audio_filename);
    let audio_extractor = AudioExtractor::new(input_video_path)?;
    audio_extractor.extract_to_file(&audio_path)?;
    println!("音频已保存到: {}", audio_path.display());

    // 7. 生成元数据 JSON
    println!("正在生成元数据...");
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
    println!("元数据已保存到: {}", metadata_path.display());

    println!("\n处理完成！");
    println!("输出目录: {}", output_dir.display());
    println!("关键帧数量: {}", metadata.scene_count);
    println!("音频文件: {}", audio_filename);

    Ok(ProcessOutput {
        output_dir: output_dir.to_path_buf(),
        metadata,
        keyframe_files,
        audio_file: audio_filename.to_string(),
    })
}