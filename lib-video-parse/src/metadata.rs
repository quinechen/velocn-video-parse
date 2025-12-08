use serde::{Deserialize, Serialize};

/// 单个场景的元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMetadata {
    /// 场景编号（从 0 开始）
    pub scene_id: usize,
    /// 关键帧图片文件名
    pub keyframe_file: String,
    /// 场景开始时间（秒）
    pub start_time: f64,
    /// 场景结束时间（秒）
    pub end_time: f64,
    /// 场景持续时间（秒）
    pub duration: f64,
}

/// 整个视频的元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    /// 输入视频文件路径
    pub input_video: String,
    /// 视频总时长（秒）
    pub total_duration: f64,
    /// 视频帧率
    pub fps: f64,
    /// 视频分辨率（宽x高）
    pub resolution: String,
    /// 检测到的场景数量
    pub scene_count: usize,
    /// 音频文件路径
    pub audio_file: String,
    /// 场景列表
    pub scenes: Vec<SceneMetadata>,
}