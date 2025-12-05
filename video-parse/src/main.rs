use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::path::PathBuf;
use video_parse::{ProcessConfig, process_video};

/// 视频拉片工具 - 分析视频内容，提取关键帧和场景信息
#[derive(Parser, Debug)]
#[command(name = "video-parse")]
#[command(about = "视频拉片工具：提取关键帧、检测场景变化、生成元数据", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// CLI 模式：处理本地视频文件
    Process {
        /// 输入视频文件路径
        #[arg(short, long)]
        input: String,

        /// 输出目录
        #[arg(short, long, default_value = "./output")]
        output: String,

        /// 场景变化检测阈值 (0.0-1.0)，值越大越敏感
        #[arg(long, default_value_t = 0.3)]
        threshold: f64,

        /// 最小场景持续时间（秒）
        #[arg(long, default_value_t = 1.0)]
        min_scene_duration: f64,

        /// 帧采样率（每秒采样多少帧用于分析）
        #[arg(long, default_value_t = 2.0)]
        sample_rate: f64,
    },
    /// Web 服务模式：启动 HTTP 服务器处理 OSS event
    Serve {
        /// 监听地址（默认从环境变量 FC_SERVER_PORT 读取，如果不存在则使用 0.0.0.0:9000）
        #[arg(short, long)]
        bind: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Commands::Process {
            input,
            output,
            threshold,
            min_scene_duration,
            sample_rate,
        } => {
            // CLI 模式
            let config = ProcessConfig {
                threshold,
                min_scene_duration,
                sample_rate,
            };
            
            process_video(&input, &output, config)
                .await
                .context("处理视频失败")?;
        }
        Commands::Serve { bind } => {
            // Web 服务模式
            // 优先使用命令行参数，其次使用环境变量 FC_SERVER_PORT，最后使用默认值 9000
            let bind_addr = bind.unwrap_or_else(|| {
                std::env::var("FC_SERVER_PORT")
                    .map(|port| format!("0.0.0.0:{}", port))
                    .unwrap_or_else(|_| "0.0.0.0:9000".to_string())
            });
            start_web_server(&bind_addr).await?;
        }
    }

    Ok(())
}

async fn start_web_server(bind: &str) -> Result<()> {
    use axum::{
        routing::{get, post},
        Router,
    };
    use tower_http::cors::CorsLayer;
    use video_parse::handler;

    let app = Router::new()
        .route("/", get(handler::health_check))
        .route("/health", get(handler::health_check))
        .route("/process", post(handler::handle_oss_event))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .context(format!("绑定地址失败: {}", bind))?;

    tracing::info!("Web 服务器启动在: http://{}", bind);
    tracing::info!("健康检查: http://{}/health", bind);
    tracing::info!("OSS Event 处理: http://{}/process", bind);

    axum::serve(listener, app)
        .await
        .context("启动服务器失败")?;

    Ok(())
}