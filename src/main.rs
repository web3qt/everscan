use anyhow::Result;
use tracing::{info, error};
use std::env;

mod config;
mod orchestrator;
mod tasks;
mod clients;
mod storage;
mod models;

use config::Config;
use orchestrator::Orchestrator;

/// 应用程序入口点
/// 
/// 主要功能：
/// 1. 初始化日志系统
/// 2. 加载配置
/// 3. 启动任务调度器
#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
        )
        .init();
    
    info!("🚀 启动区块链数据聚合平台");
    
    // 加载配置
    let config = Config::load().await?;
    info!("✅ 配置加载完成");
    
    // 创建并启动任务调度器
    let orchestrator = Orchestrator::new(config).await?;
    info!("✅ 任务调度器初始化完成");
    
    // 启动调度器（这会一直运行）
    if let Err(e) = orchestrator.start().await {
        error!("❌ 任务调度器运行失败: {}", e);
        return Err(e);
    }
    
    Ok(())
} 