use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, error};
use anyhow::{Context, Result};

use crate::handler::handle_request;

/// 启动 HTTP 服务器
pub async fn start_server(bind: &str) -> Result<()> {
    eprintln!("[Server] start_server called with bind: {}", bind);
    
    eprintln!("[Server] Parsing address...");
    let addr: SocketAddr = bind.parse()
        .context(format!("Failed to parse address: {}", bind))?;
    eprintln!("[Server] Address parsed: {}", addr);

    // 检查端口是否可用并绑定
    eprintln!("[Server] Checking if port {} is available...", addr.port());
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            eprintln!("[Server] Port {} is available", addr.port());
            listener
        }
        Err(e) => {
            let error_msg = if e.kind() == std::io::ErrorKind::AddrInUse {
                format!(
                    "Port {} is already in use. Please stop the process using this port or use a different port.",
                    addr.port()
                )
            } else {
                format!("Failed to bind to address {}: {}", addr, e)
            };
            eprintln!("[Server] ERROR: {}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    // 使用 eprintln! 确保日志立即输出到 stderr，避免缓冲
    eprintln!("[Server] ✓ Server started at: http://{}", addr);
    eprintln!("[Server] ----------------------------------------");
    eprintln!("[Server] Available endpoints:");
    eprintln!("[Server]   - health check:    GET  http://{}/health", addr);
    eprintln!("[Server]   - initialize:      POST http://{}/initialize", addr);
    eprintln!("[Server]   - invoke:          ANY  http://{}/invoke", addr);
    eprintln!("[Server]   - oss event:       ANY  http://{}/process", addr);
    eprintln!("[Server]   - direct process:  POST http://{}/process/direct", addr);
    eprintln!("[Server]   - query process:  GET  http://{}/process/query?input=<path>", addr);
    eprintln!("[Server] ----------------------------------------");
    eprintln!("[Server] ✓ HTTP server is ready, waiting for connections...");
    
    // 同时使用 info! 记录到日志系统
    info!("Server started at: http://{}", addr);
    info!("Available endpoints:");
    info!(" - health check: GET  http://{}/health", addr);
    info!(" - initialize: POST http://{}/initialize", addr);
    info!(" - invoke: ANY http://{}/invoke", addr);
    info!(" - oss event: ANY http://{}/process", addr);
    info!(" - direct process: POST http://{}/process/direct", addr);
    info!(" - query process: GET  http://{}/process/query?input=<path>", addr);
    info!("Starting HTTP server, waiting for connections...");

    loop {
        match listener.accept().await {
            Ok((stream, remote_addr)) => {
                let io = TokioIo::new(stream);
                
                tokio::task::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(io, service_fn(handle_request))
                        .await
                    {
                        error!("Error accepting connection ({}): {}", remote_addr, err);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }
}
