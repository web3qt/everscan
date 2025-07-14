use anyhow::Result;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{info, error};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod config;
mod clients;
mod models;
mod tasks;
mod web;

use config::AppConfig;
use clients::CoinMarketCapClient;
use tasks::{
    TaskManager,
    CryptoMarketTaskBuilder,
    FearGreedTaskBuilder,
    AltcoinSeasonTaskBuilder,
};
use web::{api::create_api_routes, cache::DataCache};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🚀 启动 EverScan 区块链数据聚合平台");
    
    // 调试：检查API密钥是否被加载
    if let Ok(api_key) = env::var("COINMARKETCAP_API_KEY") {
        info!("✅ CoinMarketCap API密钥已加载: {}...", &api_key[..8]);
    } else {
        error!("❌ CoinMarketCap API密钥未找到");
    }

    // 检查是否为测试模式
    if env::var("EVERSCAN_TEST_MODE").is_ok() {
        info!("🧪 运行在测试模式");
        run_test_mode().await?;
        return Ok(());
    }

    // 生产模式
    run_production_mode().await
}

/// 测试模式 - 仅测试API连接
async fn run_test_mode() -> Result<()> {
    info!("🔧 初始化测试环境");

    // 从环境变量获取API密钥
    let api_key = env::var("COINMARKETCAP_API_KEY").ok();
    
    // 创建客户端
    let coinmarketcap_client = Arc::new(CoinMarketCapClient::new(api_key, Duration::from_secs(30))?);

    // 测试CoinMarketCap贪婪恐惧指数
    info!("🧪 测试CoinMarketCap贪婪恐惧指数API");
    match coinmarketcap_client.get_fear_greed_index().await {
        Ok(fear_greed) => info!("✅ CoinMarketCap贪婪恐惧指数获取成功: {} - {}", fear_greed.value, fear_greed.value_classification),
        Err(e) => error!("❌ CoinMarketCap贪婪恐惧指数获取失败: {}", e),
    }

    // 测试山寨币季节指数
    info!("🧪 测试山寨币季节指数");
    match coinmarketcap_client.get_altcoin_season_index().await {
        Ok(altcoin_season) => info!("✅ 山寨币季节指数获取成功: {} - {}", altcoin_season.value, altcoin_season.classification_zh),
        Err(e) => error!("❌ 山寨币季节指数获取失败: {}", e),
    }

    info!("🧪 测试模式完成");
    Ok(())
}

/// 生产模式 - 完整功能
async fn run_production_mode() -> Result<()> {
    info!("🔧 初始化生产环境");

    // 加载配置
    let config = AppConfig::from_file("config.toml")?;
    info!("📖 配置加载成功");

    // 创建数据缓存
    let cache = Arc::new(DataCache::new());
    info!("💾 数据缓存初始化完成");

    // 创建客户端
    let coinmarketcap_client = Arc::new(CoinMarketCapClient::new(
        config.data_sources.coinmarketcap.api_key.clone(),
        Duration::from_secs(config.data_sources.coinmarketcap.timeout_seconds),
    )?);

    info!("🔗 API客户端创建完成");

    // 创建任务管理器
    let mut task_manager = TaskManager::new();

    // 创建并注册任务
    let crypto_task = CryptoMarketTaskBuilder::new()
        .name("加密货币市场数据采集".to_string())
        .coinmarketcap_client(coinmarketcap_client.clone())
        .interval_seconds(config.monitoring.update_interval_seconds)
        .build()?;

    let fear_greed_task = FearGreedTaskBuilder::new()
        .name("贪婪恐惧指数采集".to_string())
        .client(coinmarketcap_client.clone())
        .interval_seconds(3600) // 1小时
        .build()?;

    let altcoin_season_task = AltcoinSeasonTaskBuilder::new()
        .name("山寨币季节指数采集".to_string())
        .client(coinmarketcap_client.clone())
        .interval_seconds(3600) // 1小时
        .build()?;

    task_manager.register_task(Box::new(crypto_task)).await?;
    task_manager.register_task(Box::new(fear_greed_task)).await?;
    task_manager.register_task(Box::new(altcoin_season_task)).await?;

    info!("📋 任务注册完成，共 {} 个任务", task_manager.get_tasks().await.len());

    // 创建Web服务器
    let app = axum::Router::new()
        .nest("/api", create_api_routes(cache.clone()))
        .nest_service("/", ServeDir::new("static").append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .with_state(cache.clone());

    // 启动Web服务器
    let addr = format!("{}:{}", config.server.host, config.server.port);
    info!("🌐 启动Web服务器: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // 启动任务管理器（在后台运行）
    let task_cache = cache.clone();
    tokio::spawn(async move {
        if let Err(e) = task_manager.start(task_cache).await {
            error!("❌ 任务管理器启动失败: {}", e);
        }
    });

    // 启动Web服务器
    info!("✅ EverScan 启动完成，等待连接...");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("👋 EverScan 已停止");
    Ok(())
}

/// 优雅关闭信号处理
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🛑 收到 Ctrl+C 信号，开始优雅关闭");
        },
        _ = terminate => {
            info!("🛑 收到终止信号，开始优雅关闭");
        },
    }
}