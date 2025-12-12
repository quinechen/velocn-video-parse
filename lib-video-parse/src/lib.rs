pub mod scene_detector;
pub mod video_processor;
pub mod audio_extractor;
pub mod metadata;
pub mod oss_event;
pub mod oss_client;
pub mod processor;
pub mod handler;
pub mod config;

pub use video_processor::VideoProcessor;
pub use scene_detector::SceneDetector;
pub use audio_extractor::AudioExtractor;
pub use metadata::{SceneMetadata, VideoMetadata};
pub use oss_event::{OssEvent, OssEventItem, ProcessResponse, ProcessResult};
pub use oss_client::OssClient;
pub use processor::{ProcessConfig, ProcessOutput, process_video};
pub use config::{ConfigLoader, ExtendedConfig};