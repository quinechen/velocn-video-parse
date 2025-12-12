use anyhow::Result;
use std::path::{Path, PathBuf};
use std::env;
use crate::processor::ProcessConfig;

/// 扩展配置（包含输出路径、OSS配置等）
#[derive(Debug, Clone)]
pub struct ExtendedConfig {
    /// 视频处理配置
    pub process: ProcessConfig,
    /// DEBUG 模式开关
    pub debug_mode: bool,
    /// 输出路径（可选，如果未设置则使用临时目录）
    pub output_path: Option<PathBuf>,
    /// 目标 OSS Bucket
    pub destination_bucket: Option<String>,
    /// 目标 OSS Region
    pub destination_region: Option<String>,
    /// 目标 OSS 路径前缀
    pub destination_prefix: Option<String>,
    /// 日志级别（trace, debug, info, warn, error）
    pub log_level: String,
}

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

    /// 加载扩展配置（包含输出路径、OSS配置等）
    pub fn load_extended_config(config_file: Option<&Path>) -> Result<ExtendedConfig> {
        // 1. 加载视频处理配置
        let process_config = Self::load_config(config_file, None, None, None, None)
            .unwrap_or_else(|_| ProcessConfig::default());

        // 2. 加载配置文件（如果存在）
        let file_config = if let Some(config_path) = config_file {
            Self::load_extended_from_file(config_path).ok()
        } else {
            Self::load_extended_from_default_locations().ok()
        };

        // 3. 加载环境变量
        let debug_mode = env::var("DEBUG")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or_else(|_| {
                file_config.as_ref()
                    .map(|c| c.debug_mode)
                    .unwrap_or(false)
            });

        let output_path = env::var("OUTPUT_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| file_config.as_ref().and_then(|c| c.output_path.clone()));

        let destination_bucket = env::var("DESTINATION_BUCKET")
            .ok()
            .or_else(|| file_config.as_ref().and_then(|c| c.destination_bucket.clone()));

        let destination_region = env::var("DESTINATION_REGION")
            .ok()
            .or_else(|| file_config.as_ref().and_then(|c| c.destination_region.clone()));

        let destination_prefix = env::var("DESTINATION_PREFIX")
            .ok()
            .or_else(|| file_config.as_ref().and_then(|c| c.destination_prefix.clone()));

        let log_level = env::var("LOG_LEVEL")
            .ok()
            .or_else(|| file_config.as_ref().map(|c| c.log_level.clone()))
            .unwrap_or_else(|| "info".to_string());

        Ok(ExtendedConfig {
            process: process_config,
            debug_mode,
            output_path,
            destination_bucket,
            destination_region,
            destination_prefix,
            log_level,
        })
    }

    /// 从INI配置文件加载扩展配置
    fn load_extended_from_file(config_path: &Path) -> Result<ExtendedConfig> {
        if !config_path.exists() {
            return Err(anyhow::anyhow!("配置文件不存在: {}", config_path.display()));
        }

        let mut config_parser = configparser::ini::Ini::new();
        config_parser.load(config_path)
            .map_err(|e| anyhow::anyhow!("读取配置文件失败: {}: {}", config_path.display(), e))?;

        // 加载视频处理配置
        let process_config = Self::load_from_file(config_path)?;

        // 加载扩展配置
        let debug_mode = config_parser.get("video_parse", "debug_mode")
            .or_else(|| config_parser.get("DEFAULT", "debug_mode"))
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        let output_path = config_parser.get("video_parse", "output_path")
            .or_else(|| config_parser.get("DEFAULT", "output_path"))
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);

        let destination_bucket = config_parser.get("oss", "destination_bucket")
            .or_else(|| config_parser.get("DEFAULT", "destination_bucket"))
            .filter(|v| !v.is_empty());

        let destination_region = config_parser.get("oss", "destination_region")
            .or_else(|| config_parser.get("DEFAULT", "destination_region"))
            .filter(|v| !v.is_empty());

        let destination_prefix = config_parser.get("oss", "destination_prefix")
            .or_else(|| config_parser.get("DEFAULT", "destination_prefix"))
            .filter(|v| !v.is_empty());

        let log_level = config_parser.get("logging", "level")
            .or_else(|| config_parser.get("DEFAULT", "log_level"))
            .unwrap_or_else(|| "info".to_string());

        Ok(ExtendedConfig {
            process: process_config,
            debug_mode,
            output_path,
            destination_bucket,
            destination_region,
            destination_prefix,
            log_level,
        })
    }

    /// 从默认位置加载扩展配置文件
    fn load_extended_from_default_locations() -> Result<ExtendedConfig> {
        // 1. 当前目录的 video-parse.ini
        let current_dir_config = PathBuf::from("video-parse.ini");
        if current_dir_config.exists() {
            return Self::load_extended_from_file(&current_dir_config);
        }

        // 2. 当前目录的 .video-parse.ini
        let hidden_config = PathBuf::from(".video-parse.ini");
        if hidden_config.exists() {
            return Self::load_extended_from_file(&hidden_config);
        }

        // 3. 用户主目录的 .video-parse.ini
        if let Some(home) = env::var_os("HOME") {
            let home_config = PathBuf::from(home).join(".video-parse.ini");
            if home_config.exists() {
                return Self::load_extended_from_file(&home_config);
            }
        }

        // 4. /etc/video-parse.ini (Linux/macOS)
        let etc_config = PathBuf::from("/etc/video-parse.ini");
        if etc_config.exists() {
            return Self::load_extended_from_file(&etc_config);
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
        config_parser.set("video_parse", "debug_mode", Some("false".to_string()));
        config_parser.set("video_parse", "output_path", Some("".to_string()));
        config_parser.set("oss", "destination_bucket", Some("".to_string()));
        config_parser.set("oss", "destination_region", Some("".to_string()));
        config_parser.set("oss", "destination_prefix", Some("processed".to_string()));
        config_parser.set("logging", "level", Some("info".to_string()));

        config_parser.write(config_path)
            .map_err(|e| anyhow::anyhow!("写入配置文件失败: {}: {}", config_path.display(), e))?;

        Ok(())
    }
}
