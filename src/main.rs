use anyhow::Result;
use tracing::{info, error};
use std::env;

mod config;
// mod orchestrator; // 暂时不使用
mod tasks;
mod clients;
mod storage;
mod models;
mod web; // Web服务模块

use config::Config;

/// 应用程序入口点
/// 
/// 主要功能：
/// 1. 初始化日志系统
/// 2. 加载配置
/// 3. 启动测试模式或Web服务器
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
    
    info!("🚀 启动EverScan区块链数据聚合平台");
    
    // 加载配置
    let config = Config::load().await?;
    info!("✅ 配置加载完成");
    
    // 检查是否为测试模式
    let is_test_mode = env::var("EVERSCAN_TEST_MODE")
        .unwrap_or_else(|_| "true".to_string()) // 默认为测试模式
        .parse::<bool>()
        .unwrap_or(true);
    
    if is_test_mode {
        info!("🧪 运行在测试模式");
        run_test_mode(&config).await?;
    } else {
        info!("🌐 运行在生产模式");
        run_web_only_mode(config).await?;
    }
    
    Ok(())
}

/// Web服务模式：仅启动Web服务器和缓存
async fn run_web_only_mode(config: Config) -> Result<()> {
    use crate::web::{WebServer, cache::DataCache};
    use std::sync::Arc;
    
    // 创建数据缓存
    let cache = Arc::new(DataCache::new());
    
    // 创建Web服务器
    let web_server = WebServer::new(config, cache.clone(), None);
    
    // 获取Web服务器端口
    let web_port = env::var("WEB_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    
    info!("🌐 启动Web服务器在端口: {}", web_port);
    info!("📊 访问仪表板: http://localhost:{}", web_port);
    info!("📡 API端点: http://localhost:{}/api", web_port);
    
    // 启动Web服务器（这会一直运行）
    web_server.start(web_port).await?;
    
    Ok(())
}

/// 测试模式：仅测试数据获取功能
async fn run_test_mode(config: &Config) -> Result<()> {
    info!("🧪 开始测试模式");
    
    // 测试CoinGecko客户端
    info!("🧪 测试CoinGecko客户端...");
    let coingecko_client = crate::clients::CoinGeckoClient::new(
        config.api_keys.coingecko_api_key.clone(),
        std::time::Duration::from_secs(30)
    )?;
    
    // 从配置文件获取要测试的币种列表
    let test_coins = if let Some(crypto_config) = &config.crypto_monitoring {
        crypto_config.coins.clone()
    } else {
        vec!["bitcoin".to_string(), "hyperliquid".to_string()]
    };
    
    info!("📊 开始测试 {} 个币种的数据获取", test_coins.len());
    
    // 测试每个币种的数据获取
    for (index, coin_id) in test_coins.iter().enumerate() {
        info!("🔍 [{}/{}] 测试币种: {}", index + 1, test_coins.len(), coin_id);
        
        match coingecko_client.get_enhanced_market_data(coin_id, "usd").await {
            Ok(enhanced_data) => {
                let coin_price = &enhanced_data.coin_price;
                let indicators = &enhanced_data.technical_indicators;
                
                info!("✅ {} ({}) 数据获取成功:", coin_price.name, coin_price.symbol.to_uppercase());
                info!("   💰 当前价格: ${:.2}", coin_price.current_price);
                
                if let Some(volume) = coin_price.total_volume {
                    info!("   📈 24小时交易量: ${:.0}", volume);
                }
                if let Some(change) = coin_price.price_change_percentage_24h {
                    let change_symbol = if change >= 0.0 { "📈" } else { "📉" };
                    info!("   {} 24小时涨跌幅: {:.2}%", change_symbol, change);
                }
                if let Some(market_cap) = coin_price.market_cap {
                    info!("   🏛️ 市值: ${:.0}", market_cap);
                }
                
                // 技术指标
                info!("   📊 技术指标:");
                info!("      布林带上轨: ${:.2}", indicators.bollinger_bands.upper);
                info!("      布林带中轨: ${:.2}", indicators.bollinger_bands.middle);
                info!("      布林带下轨: ${:.2}", indicators.bollinger_bands.lower);
                
                let rsi_signal = if indicators.rsi.value >= indicators.rsi.overbought_threshold {
                    "超买 ⚠️"
                } else if indicators.rsi.value <= indicators.rsi.oversold_threshold {
                    "超卖 ⚠️"
                } else {
                    "正常 ✅"
                };
                info!("      RSI: {:.2} ({})", indicators.rsi.value, rsi_signal);
            }
            Err(e) => {
                error!("❌ {} 数据获取失败: {}", coin_id, e);
            }
        }
        
        // 在请求之间添加延迟
        if index < test_coins.len() - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
    
    info!("🧪 测试模式完成");
    Ok(())
} 