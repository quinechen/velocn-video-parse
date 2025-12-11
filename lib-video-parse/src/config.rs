use anyhow::Result;
use std::path::{Path, PathBuf};
use std::env;
use crate::processor::ProcessConfig;

/// 配置加载器
pub struct ConfigLoader;

impl ConfigLoader {
    /// 从多个源加载配置，优先级：命令行参数 > 环境变量 > 配置文件 > 默认值
    pub fn load_config(
        config_file: Option<&Path>,
        threshold: Option<f64>,
        min_scene_duration: Option<f64>,
        sample_rate: Option<f64>,
        webhook_url: Option<String>,
    ) -> Result<ProcessConfig> {
        // 1. 先加载配置文件（如果存在）
        let file_config = if let Some(config_path) = config_file {
            Self::load_from_file(config_path).ok()
        } else {
            // 尝试从默认位置加载
            Self::load_from_default_locations().ok()
        };

        // 2. 加载环境变量
        let (env_threshold, env_min_scene_duration, env_sample_rate, env_webhook_url) = Self::load_from_env();

        // 3. 合并配置（优先级：命令行 > 环境变量 > 配置文件 > 默认值）
        let config = ProcessConfig {
            threshold: threshold
                .or(env_threshold)
                .or(file_config.as_ref().map(|c| c.threshold))
                .unwrap_or(0.35),
            min_scene_duration: min_scene_duration
                .or(env_min_scene_duration)
                .or(file_config.as_ref().map(|c| c.min_scene_duration))
                .unwrap_or(0.8),
            sample_rate: sample_rate
                .or(env_sample_rate)
                .or(file_config.as_ref().map(|c| c.sample_rate))
                .unwrap_or(0.5),
            webhook_url: webhook_url
                .or(env_webhook_url)
                .or(file_config.as_ref().and_then(|c| c.webhook_url.clone())),
        };

        Ok(config)
    }

    /// 从环境变量加载配置（返回Option值，表示是否从环境变量中读取到）
    fn load_from_env() -> (Option<f64>, Option<f64>, Option<f64>, Option<String>) {
        (
            env::var("VIDEO_PARSE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok()),
            env::var("VIDEO_PARSE_MIN_SCENE_DURATION")
                .ok()
                .and_then(|v| v.parse().ok()),
            env::var("VIDEO_PARSE_SAMPLE_RATE")
                .ok()
                .and_then(|v| v.parse().ok()),
            env::var("VIDEO_PARSE_WEBHOOK_URL")
                .ok(),
        )
    }

    /// 从INI配置文件加载配置
    fn load_from_file(config_path: &Path) -> Result<ProcessConfig> {
        if !config_path.exists() {
            return Err(anyhow::anyhow!("配置文件不存在: {}", config_path.display()));
        }

        let mut config_parser = configparser::ini::Ini::new();
        config_parser.load(config_path)
            .map_err(|e| anyhow::anyhow!("读取配置文件失败: {}: {}", config_path.display(), e))?;

        // 尝试从 [video_parse] 节读取，如果没有则使用 [DEFAULT] 节
        let threshold = config_parser.get("video_parse", "threshold")
            .or_else(|| config_parser.get("DEFAULT", "threshold"))
            .and_then(|v| v.parse().ok());
        
        let min_scene_duration = config_parser.get("video_parse", "min_scene_duration")
            .or_else(|| config_parser.get("DEFAULT", "min_scene_duration"))
            .and_then(|v| v.parse().ok());
        
        let sample_rate = config_parser.get("video_parse", "sample_rate")
            .or_else(|| config_parser.get("DEFAULT", "sample_rate"))
            .and_then(|v| v.parse().ok());

        let webhook_url = config_parser.get("video_parse", "webhook_url")
            .or_else(|| config_parser.get("DEFAULT", "webhook_url"))
            .filter(|v| !v.is_empty());

        Ok(ProcessConfig {
            threshold: threshold.unwrap_or(0.35),
            min_scene_duration: min_scene_duration.unwrap_or(0.8),
            sample_rate: sample_rate.unwrap_or(0.5),
            webhook_url,
        })
    }

    /// 从默认位置加载配置文件
    fn load_from_default_locations() -> Result<ProcessConfig> {
        // 1. 当前目录的 video-parse.ini
        let current_dir_config = PathBuf::from("video-parse.ini");
        if current_dir_config.exists() {
            return Self::load_from_file(&current_dir_config);
        }

        // 2. 当前目录的 .video-parse.ini
        let hidden_config = PathBuf::from(".video-parse.ini");
        if hidden_config.exists() {
            return Self::load_from_file(&hidden_config);
        }

        // 3. 用户主目录的 .video-parse.ini
        if let Some(home) = env::var_os("HOME") {
            let home_config = PathBuf::from(home).join(".video-parse.ini");
            if home_config.exists() {
                return Self::load_from_file(&home_config);
            }
        }

        // 4. /etc/video-parse.ini (Linux/macOS)
        let etc_config = PathBuf::from("/etc/video-parse.ini");
        if etc_config.exists() {
            return Self::load_from_file(&etc_config);
        }

        Err(anyhow::anyhow!("未找到配置文件"))
    }

    /// 创建默认配置文件
    pub fn create_default_config(config_path: &Path) -> Result<()> {
        let mut config_parser = configparser::ini::Ini::new();
        config_parser.set("video_parse", "threshold", Some("0.35".to_string()));
        config_parser.set("video_parse", "min_scene_duration", Some("0.8".to_string()));
        config_parser.set("video_parse", "sample_rate", Some("0.5".to_string()));
        config_parser.set("video_parse", "webhook_url", Some("".to_string()));

        config_parser.write(config_path)
            .map_err(|e| anyhow::anyhow!("写入配置文件失败: {}: {}", config_path.display(), e))?;

        Ok(())
    }
}
