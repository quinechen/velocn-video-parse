use ffmpeg_next as ffmpeg;
use anyhow::{Context, Result};
use std::path::Path;

/// 音频提取器，从视频中提取音频
pub struct AudioExtractor {
    input_path: String,
}

impl AudioExtractor {
    pub fn new(input_path: impl AsRef<Path>) -> Result<Self> {
        ffmpeg::init().context("初始化 FFmpeg 失败")?;
        
        // 设置 FFmpeg 日志级别为 ERROR，抑制警告和信息消息
        unsafe {
            ffmpeg::sys::av_log_set_level(ffmpeg::sys::AV_LOG_ERROR as i32);
        }
        
        Ok(Self {
            input_path: input_path.as_ref().to_string_lossy().to_string(),
        })
    }

    /// 提取音频到文件
    pub fn extract_to_file(&self, output_path: impl AsRef<Path>) -> Result<()> {
        use std::process::Command;
        
        let output_path_str = output_path.as_ref().to_string_lossy().to_string();
        
        // 使用 ffmpeg 命令行工具提取音频
        // 使用 -loglevel error 抑制警告和信息消息
        let status = Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("error") // 只显示错误信息
            .arg("-i")
            .arg(&self.input_path)
            .arg("-vn") // 不包含视频
            .arg("-acodec")
            .arg("copy") // 尝试直接复制音频流
            .arg("-y") // 覆盖输出文件
            .arg(&output_path_str)
            .status()
            .context("执行 ffmpeg 命令失败")?;
        
        if !status.success() {
            // 如果复制失败，尝试重新编码为 AAC
            let status = Command::new("ffmpeg")
                .arg("-loglevel")
                .arg("error") // 只显示错误信息
                .arg("-i")
                .arg(&self.input_path)
                .arg("-vn")
                .arg("-acodec")
                .arg("aac")
                .arg("-b:a")
                .arg("192k")
                .arg("-y")
                .arg(&output_path_str)
                .status()
                .context("执行 ffmpeg 重新编码失败")?;
            
            if !status.success() {
                anyhow::bail!("音频提取失败");
            }
        }
        
        Ok(())
    }
}