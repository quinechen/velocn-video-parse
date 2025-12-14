use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use video_parse::{process_video, config::ConfigLoader};

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

fn main() {
    // 使用简单的错误处理，避免 anyhow 的复杂性
    if let Err(e) = try_main() {
        eprintln!("[Main] Error: {}", e);
        std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    // 立即输出，确保程序已启动（在 tokio runtime 之前）
    eprintln!("[Main] Program starting...");
    std::io::stderr().flush().ok();
    
    // 先解析命令行参数（在 tokio runtime 之前）
    eprintln!("[Main] Parsing command line arguments...");
    std::io::stderr().flush().ok();
    let args = Args::parse();
    eprintln!("[Main] Arguments parsed: {:?}", args.command);
    std::io::stderr().flush().ok();
    
    // 初始化日志（在 tokio runtime 之前）
    eprintln!("[Main] Initializing logging...");
    std::io::stderr().flush().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_writer(std::io::stderr)  // 使用 stderr 确保日志立即输出
        .with_ansi(true)
        .with_target(false)  // 不显示模块路径，简化输出
        .init();
    
    eprintln!("[Main] Logging initialized");
    std::io::stderr().flush().ok();
    
    // 手动创建 tokio runtime，使用多线程运行时
    eprintln!("[Main] Creating tokio runtime...");
    std::io::stderr().flush().ok();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;
    
    eprintln!("[Main] Tokio runtime created, starting async main...");
    std::io::stderr().flush().ok();
    
    rt.block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<()> {
    eprintln!("[Main] async_main started");
    std::io::stderr().flush().ok();

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
            eprintln!("[Main] Starting web server mode...");
            let bind_addr = bind.unwrap_or_else(|| {
                std::env::var("FC_SERVER_PORT")
                    .map(|port| format!("0.0.0.0:{}", port))
                    .unwrap_or_else(|_| "0.0.0.0:9000".to_string())
            });
            eprintln!("[Main] Bind address: {}", bind_addr);
            eprintln!("[Main] Calling start_web_server...");
            start_web_server(&bind_addr).await?;
        }
    }

    Ok(())
}

async fn start_web_server(bind: &str) -> Result<()> {
    eprintln!("[Main] start_web_server called with bind: {}", bind);
    use video_parse::server;
    
    eprintln!("[Main] Calling server::start_server...");
    // 启动 hyper 服务器
    server::start_server(bind).await
}