use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::path::PathBuf;
use video_parse::{ProcessConfig, process_video, config::ConfigLoader};

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

        /// 配置文件路径（可选，支持 .ini 格式）
        /// 优先级：命令行参数 > 环境变量 > 配置文件 > 默认值
        #[arg(long)]
        config: Option<PathBuf>,

        /// 场景变化检测阈值 (0.0-1.0)，值越大越敏感
        /// 可通过环境变量 VIDEO_PARSE_THRESHOLD 或配置文件设置
        #[arg(long)]
        threshold: Option<f64>,

        /// 最小场景持续时间（秒）
        /// 可通过环境变量 VIDEO_PARSE_MIN_SCENE_DURATION 或配置文件设置
        #[arg(long)]
        min_scene_duration: Option<f64>,

        /// 帧采样率（每秒采样多少帧用于分析）
        /// 可通过环境变量 VIDEO_PARSE_SAMPLE_RATE 或配置文件设置
        #[arg(long)]
        sample_rate: Option<f64>,
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
            config: config_file,
            threshold,
            min_scene_duration,
            sample_rate,
        } => {
            // CLI 模式：从配置文件、环境变量和命令行参数加载配置
            let config = ConfigLoader::load_config(
                config_file.as_deref(),
                threshold,
                min_scene_duration,
                sample_rate,
                None, // webhook_url 从配置文件或环境变量读取
            )
            .context("加载配置失败")?;
            
            println!("使用配置: threshold={:.2}, min_scene_duration={:.2}s, sample_rate={:.2} fps",
                config.threshold, config.min_scene_duration, config.sample_rate);
            
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
        routing::{get, post, put, delete, patch, head, options, MethodRouter},
        Router,
    };
    use tower_http::cors::CorsLayer;
    use video_parse::handler;

    // 创建接受任何HTTP方法的路由
    let invoke_route = MethodRouter::new()
        .get(handler::handle_invoke)
        .post(handler::handle_invoke)
        .put(handler::handle_invoke)
        .delete(handler::handle_invoke)
        .patch(handler::handle_invoke)
        .head(handler::handle_invoke)
        .options(handler::handle_invoke);
    
    let process_any_route = MethodRouter::new()
        .get(handler::handle_oss_event_any)
        .post(handler::handle_oss_event_any)
        .put(handler::handle_oss_event_any)
        .delete(handler::handle_oss_event_any)
        .patch(handler::handle_oss_event_any)
        .head(handler::handle_oss_event_any)
        .options(handler::handle_oss_event_any);

    let app = Router::new()
        .route("/", get(handler::health_check))
        .route("/health", get(handler::health_check))
        // 函数计算初始化端点
        .route("/initialize", post(handler::handle_initialize))
        // 函数计算调用端点（接受任何HTTP方法）
        .route("/invoke", invoke_route)
        // OSS事件处理端点（函数计算模式，接受任何HTTP方法以兼容不同调用方式）
        .route("/process", process_any_route)
        // 直接处理端点（支持本地文件和OSS文件）
        .route("/process/direct", post(handler::handle_direct_process))
        // 查询参数处理端点（GET请求，方便测试）
        .route("/process/query", get(handler::handle_process_query))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .context(format!("绑定地址失败: {}", bind))?;

    tracing::info!("Web 服务器启动在: http://{}", bind);
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("可用端点:");
    tracing::info!("  • 健康检查: GET  http://{}/health", bind);
    tracing::info!("  • 函数计算初始化: POST http://{}/initialize", bind);
    tracing::info!("  • 函数计算调用: ANY http://{}/invoke", bind);
    tracing::info!("  • OSS事件处理: ANY http://{}/process", bind);
    tracing::info!("  • 直接处理: POST http://{}/process/direct", bind);
    tracing::info!("  • 查询处理: GET  http://{}/process/query?input=<path>", bind);
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    axum::serve(listener, app)
        .await
        .context("启动服务器失败")?;

    Ok(())
}